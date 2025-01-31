use std::{collections::HashMap, vec};

use bollard::{container, image, Docker};
use futures::StreamExt;
use tokio::io::AsyncReadExt;
use traq_python_bot::create_tar_archive;

pub struct DockerManagerBuilder {
    // docker client
    docker: Docker,
    // configs
    tar_dir: String,
    // docker files
    docker_files: HashMap<String, DockerFiles>,
}

struct DockerFiles {
    dockerfile_dir: String,
    dockerfile: String,
}

impl DockerManager {
    pub fn builder(
        tar_dir: impl Into<String>,
        sandbox_dir: impl Into<String>,
    ) -> DockerManagerBuilder {
        DockerManagerBuilder {
            docker: Docker::connect_with_local_defaults().unwrap(),
            tar_dir: tar_dir.into(),
            docker_files: HashMap::new(),
        }
    }
}

impl DockerManagerBuilder {
    pub fn docker_files(
        mut self,
        name: impl Into<String> + AsRef<str>,
        dockerfile_dir: impl Into<String>,
        dockerfile: impl Into<String>,
    ) -> Self {
        println!("DockerManagerBuilder add: {}", name.as_ref());
        self.docker_files.insert(
            name.into(),
            DockerFiles {
                dockerfile_dir: dockerfile_dir.into(),
                dockerfile: dockerfile.into(),
            },
        );
        self
    }

    pub async fn build(self) -> Result<DockerManager, Box<dyn std::error::Error>> {
        let DockerManagerBuilder {
            docker,
            tar_dir,
            docker_files,
        } = self;

        let mut image_ids = HashMap::new();
        for (name, dockerfile) in docker_files {
            let docker_image = dockerfile.build_image(&docker, &name, &tar_dir).await?;
            println!("DockerManagerBuilder build: {}", name);
            image_ids.insert(name, docker_image);
        }

        Ok(DockerManager { docker, image_ids })
    }
}

impl DockerFiles {
    async fn build_image(
        &self,
        docker: &Docker,
        name: impl AsRef<str>,
        tar_dir: impl AsRef<str>,
    ) -> Result<DockerImage, Box<dyn std::error::Error>> {
        // make tar file and reed it
        let tar_file_name = format!("{}/{}.tar", tar_dir.as_ref(), name.as_ref());

        create_tar_archive(std::path::Path::new(&self.dockerfile_dir), &tar_file_name).await?;

        let mut tar_file_u8 = Vec::new();
        tokio::fs::File::open(&tar_file_name)
            .await?
            .read_to_end(&mut tar_file_u8)
            .await?;

        println!("tar file created: {}", tar_file_name);

        // build image
        let name_tug = format!("botpy-{}:{}", name.as_ref(), uuid::Uuid::now_v7());

        let build_image_options = bollard::image::BuildImageOptions {
            dockerfile: self.dockerfile.as_str(),
            t: &name_tug,
            ..Default::default()
        };

        let mut build_stream =
            docker.build_image(build_image_options, None, Some(tar_file_u8.into()));

        let mut image_id = None;

        while let Some(result) = build_stream.next().await {
            match result {
                Ok(output) => {
                    if let Some(bollard::secret::ImageId { id: Some(id) }) = output.aux {
                        if image_id.is_none() {
                            println!("Image {} ID: {}", name.as_ref(), id);
                            image_id = Some(id);
                        } else {
                            return Err("Multiple image id".into());
                        }
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        if let Some(id) = image_id {
            Ok(DockerImage {
                image_name_tug: name_tug,
                image_id: id,
            })
        } else {
            Err("No image id given".into())
        }
    }
}

pub struct DockerManager {
    docker: Docker,
    image_ids: HashMap<String, DockerImage>,
}

struct DockerImage {
    image_name_tug: String,
    image_id: String,
}

#[derive(Debug)]
pub struct RunResult {
    pub std_output: String,
    pub std_error: String,
    pub time: tokio::time::Duration,
}

impl DockerManager {
    // Build docker image from dockerfile and execute f with DockerContainer.
    // Then remove the container after f is executed.
    pub async fn run_image(
        &self,
        name: impl AsRef<str> + Into<String>,
        args: Vec<impl AsRef<str>>,
    ) -> Result<RunResult, Box<dyn std::error::Error>> {
        let image = self
            .image_ids
            .get(name.as_ref())
            .ok_or(format!("No image of the name: {}", name.as_ref()))?;

        // create and start container
        println!(
            "args: {:?}",
            args.iter().map(|arg| arg.as_ref()).collect::<Vec<_>>()
        );

        let container_config = container::Config {
            image: Some(image.image_name_tug.as_str()),
            cmd: Some(args.iter().map(|arg| arg.as_ref()).collect()),
            tty: Some(false),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let container_name = format!("botpy-{}-{}", name.as_ref(), uuid::Uuid::now_v7());

        let container = self
            .docker
            .create_container::<&str, &str>(
                Some(bollard::container::CreateContainerOptions::<&str> {
                    name: container_name.as_str(),
                    ..Default::default()
                }),
                container_config,
            )
            .await?;

        let container_id = container.id;

        self.docker
            .start_container::<&str>(&container_name, None)
            .await?;

        // log

        let mut logs = self
            .docker
            .logs::<String>(
                &container_id,
                Some(bollard::container::LogsOptions::<String> {
                    stdout: true,
                    stderr: true,
                    follow: false,
                    ..Default::default()
                }),
            )
            .fuse();

        let mut std_output = String::new();
        let mut std_error = String::new();
        let timeout = tokio::time::Duration::from_secs(5);
        let start_time = tokio::time::Instant::now();

        tokio::select! {
            res = async {
                while let Some(log) = logs.next().await {
                    match log {
                        Ok(container::LogOutput::StdOut { message }) => {
                            std_output.push_str(std::str::from_utf8(&message)?);
                        }
                        Ok(container::LogOutput::StdErr { message }) => {
                            std_error.push_str(std::str::from_utf8(&message)?);
                        },
                        Err(e) => {
                            return Err::<(), Box<dyn std::error::Error>>(e.into());
                        }
                        _ => {}
                    }
                }
                Ok(())
            } => res?,
            _ = tokio::time::sleep(timeout) => {},
        }

        let run_time = start_time.elapsed();

        // stop and remove container

        // let inspect_result = self
        //     .docker
        //     .inspect_container(&container_name, None)
        //     .await
        //     .unwrap();
        // println!("Real Container ID: {:?}", inspect_result.id);

        // println!("container_id: {}", container_id);

        self.docker
            .stop_container(
                &container_name,
                Some(bollard::container::StopContainerOptions { t: 0 }),
            )
            .await?;

        self.docker
            .remove_container(
                &container_name,
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;

        Ok(RunResult {
            std_output,
            std_error,
            time: run_time,
        })
    }

    // run docker hello-world
    pub async fn hello(&self) -> Result<RunResult, Box<dyn std::error::Error>> {
        // create and start container
        let container_config = container::Config {
            image: Some("hello-world"),
            tty: Some(false),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container::<&str, &str>(None, container_config)
            .await
            .unwrap();

        let container_id = container.id;

        let timer = tokio::time::Instant::now();

        self.docker
            .start_container::<&str>(&container_id, None)
            .await
            .unwrap();

        // log

        let mut logs = self
            .docker
            .logs::<String>(
                &container_id,
                Some(bollard::container::LogsOptions::<String> {
                    stdout: true,
                    stderr: true,
                    follow: false,
                    ..Default::default()
                }),
            )
            .fuse();

        let mut std_output = String::new();
        let mut std_error = String::new();

        while let Some(log) = logs.next().await {
            match log {
                Ok(container::LogOutput::StdOut { message }) => {
                    std_output.push_str(std::str::from_utf8(&message).unwrap());
                }
                Ok(container::LogOutput::StdErr { message }) => {
                    std_error.push_str(std::str::from_utf8(&message).unwrap());
                }
                Err(e) => {
                    return Err(e.into());
                }
                _ => {}
            }
        }

        let time = timer.elapsed();

        // stop and remove container

        self.docker
            .stop_container(
                &container_id,
                Some(bollard::container::StopContainerOptions { t: 0 }),
            )
            .await
            .unwrap();

        self.docker
            .remove_container(
                &container_id,
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        Ok(RunResult {
            std_output,
            std_error,
            time,
        })
    }

    pub async fn python3(
        &self,
        args: Vec<impl AsRef<str>>,
        host_mount_dir_path: impl AsRef<str>,
        container_code_file_name: impl AsRef<str>,
        container_output_file_name: impl AsRef<str>,
    ) -> Result<RunResult, Box<dyn std::error::Error>> {
        let container_code_file_path = format!("/sandbox/{}", container_code_file_name.as_ref());
        let container_output_file_path =
            format!("/sandbox/{}", container_output_file_name.as_ref());

        let cmd = [
            "python3",
            &container_code_file_path,
            ">",
            &container_output_file_path,
        ]
        .join(" ")
        .to_string();

        let container_config = container::Config {
            image: Some("python:latest"),
            cmd: Some(vec!["sh", "-c", &cmd]),
            tty: Some(true),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            host_config: Some(bollard::models::HostConfig {
                binds: Some(vec![format!(
                    "{}:/sandbox:rw",
                    host_mount_dir_path.as_ref()
                )]),
                init: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container::<&str, &str>(None, container_config)
            .await
            .unwrap();

        let container_id = container.id;

        let timer = tokio::time::Instant::now();

        self.docker
            .start_container::<&str>(&container_id, None)
            .await
            .unwrap();

        // log

        let mut logs = self
            .docker
            .logs::<String>(
                &container_id,
                Some(bollard::container::LogsOptions::<String> {
                    stdout: true,
                    stderr: true,
                    follow: false,
                    ..Default::default()
                }),
            )
            .fuse();

        let mut std_output = String::new();
        let mut std_error = String::new();

        while let Some(log) = logs.next().await {
            match log {
                Ok(container::LogOutput::StdErr { message }) => {
                    std_error.push_str(std::str::from_utf8(&message).unwrap());
                }
                Err(e) => {
                    return Err(e.into());
                }
                _ => {}
            }
        }

        let time = timer.elapsed();

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // stop and remove container

        self.docker
            .stop_container(
                &container_id,
                Some(bollard::container::StopContainerOptions { t: 0 }),
            )
            .await
            .unwrap();

        self.docker
            .remove_container(
                &container_id,
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        Ok(RunResult {
            std_output,
            std_error,
            time,
        })
    }

    pub async fn rm_container(
        &self,
        id: impl AsRef<str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.docker
            .remove_container(
                id.as_ref(),
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn rm_images(&self, id: impl AsRef<str>) -> Result<(), Box<dyn std::error::Error>> {
        self.docker.remove_image(id.as_ref(), None, None).await?;

        Ok(())
    }
}
