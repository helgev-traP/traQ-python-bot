use reqwest::Response;

#[derive(Clone)]
pub struct TraqApi {
    url_api_prefix: String,
    http_client: reqwest::Client,
}

/// constructor
impl TraqApi {
    pub fn new(host: impl AsRef<str>, bot_token: impl AsRef<str>) -> Self {
        let authorization_value = format!("{} {}", "Bearer", bot_token.as_ref());

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&authorization_value).unwrap(),
        );

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            url_api_prefix: format!("https://{}/api/v3", host.as_ref()),
            http_client,
        }
    }
}

/// apis
impl TraqApi {
    pub async fn send_message(
        &self,
        channel_id: impl AsRef<str>,
        message: impl AsRef<str>,
        embed: bool,
    ) -> Result<Response, reqwest::Error> {
        let url = format!(
            "{}/channels/{}/messages",
            self.url_api_prefix,
            channel_id.as_ref()
        );

        let body = serde_json::json!({
            "content": message.as_ref(),
            "embed": embed,
        });

        self.http_client.post(&url).json(&body).send().await
    }
}
