use std::collections::HashMap;

use bollard::{container, Docker};
use futures::StreamExt;


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
    pub fn builder(tar_dir: impl Into<String>) -> DockerManagerBuilder {
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
        name: impl Into<String>,
        dockerfile_dir: impl Into<String>,
        dockerfile: impl Into<String>,
    ) -> Self {
        self.docker_files.insert(
            name.into(),
            DockerFiles {
                dockerfile_dir: dockerfile_dir.into(),
                dockerfile: dockerfile.into(),
            },
        );
        self
    }

    pub fn build(&self) -> DockerManager {
        // build images and hold image ids


        todo!()
    }
}

pub struct DockerManager {
    docker: Docker,
    docker_files: HashMap<String, DockerFiles>,
}

struct DockerImage {
    image_id: String,
}

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
        // create and start container
        println!(
            "args: {:?}",
            args.iter().map(|arg| arg.as_ref()).collect::<Vec<_>>()
        );

        let container_config = container::Config {
            image: Some(name.as_ref()),
            cmd: Some(args.iter().map(|arg| arg.as_ref()).collect()),
            tty: Some(false),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let container = self
            .docker
            .create_container::<&str, &str>(None, container_config)
            .await?;

        let container_id = container.id;

        self.docker
            .start_container::<&str>(&container_id, None)
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

        let inspect_result = self
            .docker
            .inspect_container(&container_id, None)
            .await
            .unwrap();
        println!("Real Container ID: {:?}", inspect_result.id);

        println!("container_id: {}", container_id);

        self.docker
            .stop_container(
                &container_id,
                Some(bollard::container::StopContainerOptions { t: 0 }),
            )
            .await?;

        self.docker
            .remove_container(
                &container_id,
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

    pub async fn rm_container(&self, id: impl AsRef<str>) -> Result<(), Box<dyn std::error::Error>> {
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
