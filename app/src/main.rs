use std::{collections::HashMap, sync::Arc};

use bollard::{
    container::{Config, LogOutput},
    image::BuildImageOptions,
    Docker,
};
use futures::{future::BoxFuture, FutureExt, StreamExt};
use tokio::io::AsyncReadExt;
use traq_python_bot::{
    create_tar_archive,
    event::{Event, Message, MessageBody, MessageCreatedUpdated},
    event_loop::EventLoop,
    traq_api::TraqApi,
};

mod parse;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    let host = std::env::var("TRAQ_HOST").unwrap();
    let bot_id = std::env::var("TRAQ_BOT_ID").unwrap();
    let token = std::env::var("TRAQ_BOT_TOKEN").unwrap();

    // create parser

    let mut parser = parse::Parser::new();
    parser.add("ping", format!(r"(?s)^@{} +-ping$", regex::escape(&bot_id)));
    parser.add(
        "docker-hello",
        format!(r"(?s)^@{} +-docker-hello$", regex::escape(&bot_id)),
    );
    parser.add(
        "rm-all-containers",
        format!(r"(?s)^@{} +-rm-all-containers$", regex::escape(&bot_id)),
    );
    parser.add(
        "rm-all-images",
        format!(r"(?s)^@{} +-rm-all-images$", regex::escape(&bot_id)),
    );

    parser.add(
        "python",
        format!(
            r"(?s)^@{}(?<arg>.*)\n+```(?:python)?\n(?<code>.*?)\n```$",
            regex::escape(&bot_id)
        ),
    );

    // prepare docker

    let mut docker_manager = DockerManager::new("docker/tar");
    docker_manager.add_docker_files("python", "./docker/python", "Dockerfile");

    // create event loop

    let mut event_loop = EventLoop::build_from_host_and_token(host.to_string(), token).await;
    let stats = Stats {
        parser,
        docker: docker_manager,
    };
    event_loop.run(stats, event_loop_fn).await;
}

struct Stats {
    parser: parse::Parser,
    docker: DockerManager,
}

fn event_loop_fn(
    message: tokio_tungstenite::tungstenite::Message,
    api: TraqApi,
    stats: Arc<Stats>,
) -> BoxFuture<'static, ()> {
    async move {
        match message {
            tokio_tungstenite::tungstenite::Message::Text(utf8_bytes) => {
                let event = Event::from_json(&utf8_bytes.to_string()).unwrap();

                if let Event::Message {
                    body: Message::MessageCreated(MessageCreatedUpdated { message, .. }),
                    ..
                } = event
                {
                    let MessageBody { plain_text, .. } = message;

                    let Some((pattern_name, captures)) = &stats.parser.parse(&plain_text) else {
                        api.send_message(&message.channel_id, ":question:", false)
                            .await
                            .unwrap();
                        return;
                    };

                    match pattern_name.as_str() {
                        "ping" => {
                            api.send_message(&message.channel_id, "pong", false)
                                .await
                                .unwrap();
                        }
                        "docker-hello" => {
                            let result = stats.docker.hello().await.unwrap();

                            let mut output = format!(
                                "time: {}ms\nstdout:\n```\n{}\n```",
                                result.time.as_millis(),
                                result.std_output
                            );

                            if !result.std_error.is_empty() {
                                output.push_str(&format!(
                                    "\nstderr:\n```\n{}\n```",
                                    result.std_error
                                ));
                            }

                            api.send_message(&message.channel_id, &output, false)
                                .await
                                .unwrap();
                        }
                        "rm-all-containers" => {
                            api.send_message(&message.channel_id, "not implemented.", false)
                                .await
                                .unwrap();
                        }
                        "rm-all-images" => {
                            api.send_message(&message.channel_id, "not implemented.", false)
                                .await
                                .unwrap();
                        }
                        "python" => {
                            let code = captures["code"].to_string();
                            let args = captures["arg"]
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>();

                            let result = stats
                                .docker
                                .run_image(
                                    "python:3.10-slim",
                                    // vec![vec![code], args].into_iter().flatten().collect(),
                                    Vec::<&str>::new(),
                                )
                                .await
                                .unwrap();

                            let mut output = format!(
                                "time: {}ms\nstdout:\n```\n{}\n```",
                                result.time.as_millis(),
                                result.std_output
                            );

                            if !result.std_error.is_empty() {
                                output.push_str(&format!(
                                    "\nstderr:\n```\n{}\n```",
                                    result.std_error
                                ));
                            }

                            api.send_message(&message.channel_id, &output, false)
                                .await
                                .unwrap();
                        }
                        _ => {
                            todo!()
                        }
                    }
                }
            }
            tokio_tungstenite::tungstenite::Message::Close(close_frame) => todo!(),
            _ => (),
        }
    }
    .boxed()
}

struct DockerManager {
    // docker client
    docker: Docker,
    // configs
    tar_dir: String,
    //
    docker_files: HashMap<String, DockerFiles>,
}

struct DockerFiles {
    dockerfile_dir: String,
    dockerfile: String,
}

struct DockerImage {
    image_id: String,
}

struct DockerContainer<'a> {
    docker: &'a Docker,
    container_id: String,
}

struct RunResult {
    std_output: String,
    std_error: String,
    time: tokio::time::Duration,
}

impl DockerManager {
    fn new(tar_dir: impl Into<String>) -> DockerManager {
        Self {
            docker: Docker::connect_with_local_defaults().unwrap(),
            tar_dir: tar_dir.into(),
            docker_files: HashMap::new(),
        }
    }

    fn add_docker_files(
        &mut self,
        name: impl Into<String>,
        dockerfile_dir: impl Into<String>,
        dockerfile: impl Into<String>,
    ) {
        self.docker_files.insert(
            name.into(),
            DockerFiles {
                dockerfile_dir: dockerfile_dir.into(),
                dockerfile: dockerfile.into(),
            },
        );
    }

    // Build docker image from dockerfile and execute f with DockerContainer.
    // Then remove the container after f is executed.
    async fn run_image(
        &self,
        name: impl AsRef<str> + Into<String>,
        args: Vec<impl AsRef<str>>,
    ) -> Result<RunResult, Box<dyn std::error::Error>> {
        // create and start container
        println!(
            "args: {:?}",
            args.iter().map(|arg| arg.as_ref()).collect::<Vec<_>>()
        );

        let container_config = Config {
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
                        Ok(LogOutput::StdOut { message }) => {
                            std_output.push_str(std::str::from_utf8(&message)?);
                        }
                        Ok(LogOutput::StdErr { message }) => {
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
    async fn hello(&self) -> Result<RunResult, Box<dyn std::error::Error>> {
        // create and start container
        let container_config = Config {
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
                Ok(LogOutput::StdOut { message }) => {
                    std_output.push_str(std::str::from_utf8(&message).unwrap());
                }
                Ok(LogOutput::StdErr { message }) => {
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

    async fn rm_container(&self, id: impl AsRef<str>) -> Result<(), Box<dyn std::error::Error>> {
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

    async fn rm_images(&self, id: impl AsRef<str>) -> Result<(), Box<dyn std::error::Error>> {
        self.docker.remove_image(id.as_ref(), None, None).await?;

        Ok(())
    }
}
