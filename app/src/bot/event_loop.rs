use std::os::linux::raw::stat;

use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::{
    traq_api::TraqApi,
    traq_server_connecter::{TraqServerConnecter, TraqServerConnecterBuilder},
};

pub struct EventLoopBuilder {
    pub connecter: TraqServerConnecterBuilder,
}

impl EventLoopBuilder {
    pub async fn build(self) -> EventLoop {
        EventLoop {
            connecter: self.connecter.build().await,
        }
    }
}

pub struct EventLoop {
    connecter: TraqServerConnecter,
}

impl EventLoop {
    pub async fn build_from_host_and_token(
        host: impl Into<String>,
        token: impl Into<String>,
    ) -> EventLoop {
        let connecter = TraqServerConnecterBuilder {
            host: host.into(),
            bot_token: token.into(),
        };

        EventLoopBuilder { connecter }.build().await
    }

    pub async fn run<Stats, F, Fut>(&mut self, stats: Stats, event_loop: F)
    where
        Stats: Send + Sync,
        F: Fn(Message, TraqApi, std::sync::Arc<Stats>) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let stats = std::sync::Arc::new(stats);

        let ws_read = &mut self.connecter.ws_read;
        let ws_write = &mut self.connecter.ws_write;
        let http_client = &self.connecter.http_client;

        while let Some(message) = ws_read.next().await {
            match message {
                Ok(message) => {
                    if let Message::Ping(_) = message {
                        ws_write
                            .send(Message::Pong(Default::default()))
                            .await
                            .unwrap();
                        continue;
                    } else {
                        (event_loop)(message, http_client.clone(), stats.clone()).await;
                    }
                }
                Err(e) => {
                    // todo: implement error handling
                    println!("Error: {:?}", e);
                    break;
                }
            }
        }
    }
}
