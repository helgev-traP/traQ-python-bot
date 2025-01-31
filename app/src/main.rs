use futures::{future::BoxFuture, FutureExt};
use std::{error::Error, sync::Arc};

use traq_python_bot::{
    event::{Event, Message, MessageBody, MessageCreatedUpdated},
    event_loop::EventLoop,
    traq_api::TraqApi,
};

mod docker_manager;
use docker_manager::DockerManager;
mod err;
mod parse;
use err::ServerError;

#[tokio::main]
async fn main() {
    if let Err(e) = server_main().await {
        eprintln!(
            "\n
========================================\n
Server stopped with error:\n
{}\n",
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
    let sandbox_dir = std::env::var("SANDBOX_DIR")?;

    println!("Starting server...");

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

    println!("Parser created.");

    // prepare docker

    // let docker_manager = DockerManager::builder("docker/tar", &sandbox_dir).docker_files(
    //     "python",
    //     "./docker/python",
    //     "Dockerfile",
    // );

    let docker_manager = DockerManager::builder("docker/tar", &sandbox_dir);

    let docker = docker_manager.build().await?;

    let stats = Stats {
        sandbox_dir,
        parser,
        docker,
    };

    println!("Docker prepared.");

    // create event loop

    let mut event_loop = EventLoop::build_from_host_and_token(host.to_string(), token).await;
    println!("Start event loop.");
    event_loop.run(stats, event_loop_fn).await;
    Ok(())
}

struct Stats {
    sandbox_dir: String,
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

                    println!("Received:\n{}", plain_text);

                    let Some((pattern_name, captures)) = &stats.parser.parse(&plain_text) else {
                        println!("Send:\n:question:");
                        api.send_message(&message.channel_id, ":question:", false)
                            .await
                            .unwrap();
                        return;
                    };

                    let response = match pattern_name.as_str() {
                        "ping" => "pong".to_owned(),
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

                            output
                        }
                        "rm-all-containers" => "not implemented.".to_owned(),
                        "rm-all-images" => "not implemented.".to_owned(),
                        "python" => {
                            let code = captures["code"].to_string();
                            let args = captures["arg"]
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>();

                            // let result = stats
                            //     .docker
                            //     .python(code, args, api.clone())
                            //     .await;

                            // let mut output = format!(
                            //     "time: {}ms\nstdout:\n```\n{}\n```",
                            //     result.time.as_millis(),
                            //     result.std_output
                            // );

                            // if !result.std_error.is_empty() {
                            //     output.push_str(&format!(
                            //         "\nstderr:\n```\n{}\n```",
                            //         result.std_error
                            //     ));
                            // }

                            python(code, args, &stats.sandbox_dir, &stats.docker)
                                .await
                                .unwrap()
                        }
                        _ => {
                            panic!(
                                "Unknown pattern name: {} | event loop do not match all patterns.",
                                pattern_name
                            );
                        }
                    };

                    println!("Send:\n{}", response);
                    api.send_message(&message.channel_id, &response, false)
                        .await
                        .unwrap();
                }
            }
            tokio_tungstenite::tungstenite::Message::Close(close_frame) => todo!(),
            _ => (),
        }
    }
    .boxed()
}

async fn python(
    code: String,
    args: Vec<String>,
    sandbox_dir: &str,
    docker: &DockerManager,
) -> Result<String, Box<dyn Error>> {
    // prepare sandbox directory
    let sandbox_dir = format!("{}/python-{}", sandbox_dir, uuid::Uuid::now_v7());
    tokio::fs::create_dir_all(&sandbox_dir).await?;

    // prepare sandbox input and output files
    let input_file = format!("{}/python-code.py", &sandbox_dir);

    // write code to input file
    tokio::fs::write(&input_file, code).await?;

    // run docker
    let result = docker
        .python3(args, &sandbox_dir, "python-code.py", "output.txt")
        .await?;

    println!("result: {:?}", result);

    // read output file
    println!("output file: {}/output.txt", sandbox_dir);
    let output = tokio::fs::read_to_string(&format!("{}/output.txt", sandbox_dir)).await?;

    // remove input and output files
    tokio::fs::remove_file(&input_file).await?;
    tokio::fs::remove_file(&format!("{}/output.txt", sandbox_dir)).await?;

    // remove sandbox directory
    tokio::fs::remove_dir_all(&sandbox_dir).await?;

    Ok(
        format!(
            "container time: {}ms\nstdout:\n```\n{}\n```",
            result.time.as_millis(),
            output
        )
    )
}
