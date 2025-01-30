use futures::{future::BoxFuture, FutureExt};
use std::sync::Arc;

use traq_python_bot::{
    event::{Event, Message, MessageBody, MessageCreatedUpdated},
    event_loop::EventLoop,
    traq_api::TraqApi,
};

mod docker_manager;
mod parse;
use docker_manager::DockerManager;

#[tokio::main]
async fn main() {
    if let Err(e) = server_main().await {
        eprintln!(
            "\n\n
            ========================================
            Server stopped with error:\n{}",
            e
        );
    };

    // remove all containers and images
    todo!()
}

async fn server_main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;

    let host = std::env::var("TRAQ_HOST")?;
    let bot_id = std::env::var("TRAQ_BOT_ID")?;
    let token = std::env::var("TRAQ_BOT_TOKEN")?;

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

    let docker_manager = DockerManager::builder("docker/tar").docker_files(
        "python",
        "./docker/python",
        "Dockerfile",
    );

    // create event loop

    let mut event_loop = EventLoop::build_from_host_and_token(host.to_string(), token).await;
    let stats = Stats {
        parser,
        docker: docker_manager.build(),
    };
    event_loop.run(stats, event_loop_fn).await;

    Ok(())
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
