use futures::{
    stream::{SplitSink, SplitStream},
    StreamExt,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::traq_api::TraqApi;

pub struct TraqServerConnecterBuilder {
    /// The host of the Traq server.
    pub host: String,
    /// Bot token from Bot--Console
    pub bot_token: String,
}

impl TraqServerConnecterBuilder {
    pub async fn build(self) -> TraqServerConnecter {
        let wss_url = format!("wss://{}/api/v3/bots/ws", &self.host);
        let authorization_value = format!("{} {}", "Bearer", self.bot_token);

        // connect websocket

        let ws_request = http::Request::builder()
            .method("GET")
            .header("Host", &self.host)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-Websocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .uri(wss_url)
            .header("Authorization", &authorization_value)
            .body(())
            .unwrap();

        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_request).await.unwrap();

        let (write, read) = ws_stream.split();

        TraqServerConnecter {
            ws_read: read,
            ws_write: write,
            http_client: TraqApi::new(self.host, self.bot_token),
        }
    }
}

pub struct TraqServerConnecter {
    pub(crate) ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    pub(crate) ws_write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,

    pub(crate) http_client: TraqApi,
}
