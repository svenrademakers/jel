// use crate::http_request_handler_trait::HttpRequestHandlerTrait;
// use async_trait::async_trait;

// pub struct RedirectServer {
//     redirect_location: String,
// }

// impl RedirectServer {
//     pub fn new(hostname: String) -> Self {
//         let redirect_location = format!("http://{}", hostname);
//         RedirectServer { redirect_location }
//     }
// }
 
// #[async_trait]
// impl HttpRequestHandlerTrait for RedirectServer {
//     async fn on_request(
//         &self,
//         _: http::Request<&[u8]>,
//     ) -> std::io::Result<http::Response<Vec<u8>>> {
//         let response = http::Response::builder()
//             .status(http::StatusCode::MOVED_PERMANENTLY)
//             .header("Location", self.redirect_location.clone())
//             .body(Vec::new())
//             .expect("building of redirect http response must succeed");
//         Ok(response)
//     }
// }
