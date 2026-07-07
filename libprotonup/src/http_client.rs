#[allow(async_fn_in_trait)]
pub trait HttpSend {
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, reqwest::Error>;
}

pub struct RealSender;

impl HttpSend for RealSender {
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, reqwest::Error> {
        request.send().await
    }
}

#[cfg(test)]
pub struct MockSender {
    pub status: u16,
    pub body: String,
    pub content_type: String,
}

#[cfg(test)]
impl HttpSend for MockSender {
    async fn send(
        &self,
        _request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let response = http::Response::builder()
            .status(self.status)
            .header("Content-Type", &self.content_type)
            .body(self.body.clone())
            .unwrap();
        Ok(response.into())
    }
}
