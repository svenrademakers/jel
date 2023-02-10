use super::{as_json_response, lookup_content_type};
use crate::middleware::LocalStreamStore;
use actix_service::{Transform, Service};
use actix_web::{dev::{ServiceRequest, ServiceResponse}, Error, body::EitherBody, HttpResponse, http::{StatusCode, header}, error::HttpError, post};
use futures_util::future::LocalBoxFuture;
use std::{
    fmt::Display,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    }, future::{Ready, ready},
};

pub struct StreamsService{
    stream_store: Arc<LocalStreamStore>,
    base_url: String,
    dev_mode: bool,
}

impl<S, B> Transform<S, ServiceRequest> for StreamsService
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = StreamServiceResponse<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(StreamServiceResponse {service, 
            stream_store: self.stream_store,
            base_url: self.base_url,
            dev_mode: self.dev_mode
        }))
    }
}

#[derive(Debug, Clone)]
pub struct StreamServiceResponse<S>{
    service: S,
    stream_store: Arc<LocalStreamStore>,
    base_url: String,
    dev_mode: bool,
}

impl<S> StreamServiceResponse<S> {
    async fn test(&self) -> HttpResponse {
        static GTEST_VID : &str = "https://commondatastorage.googleapis.com/gtv-videos-bucket/CastVideos/hls/DesigningForGoogleCast.m3u8";
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let number = COUNTER.fetch_add(1, Ordering::Relaxed);
        let test_description = format!("this is a test {}", number);

        self.stream_store
            .register(
                test_description,
                vec![PathBuf::from(GTEST_VID), PathBuf::from("test1.m3u8")],
                chrono::Utc::now(),
            )
            .await
            .unwrap();

        HttpResponse::Ok().finish()
    }

}

#[post("/")]
fn preflight_response() -> HttpResponse {
    HttpResponse::build(StatusCode::NO_CONTENT)
        .append_header((
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                "Content-Length, Content-Type, Range",
                ))
        .append_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
        .append_header((header::ACCESS_CONTROL_MAX_AGE, "1728000"))
        .append_header((header::CONTENT_TYPE, "text/plain charset=UTF-8"))
        .append_header((header::CONTENT_LENGTH, "0"))
        .finish()
}

impl<S, B> Service<ServiceRequest> for StreamServiceResponse<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        Box::pin(async move {req.})
//        if request.method() == Method::OPTIONS {
//            return Ok(self.preflight_response());
//        }
//
//        let cursor = request.uri().path()[1..].find('/').unwrap() + 2;
//        match &request.uri().path()[cursor..] {
//            "test" if self.dev_mode => self.test().await,
//            "all" => as_json_response(
//                &self
//                    .stream_store
//                    .get_available_streams(&self.base_url)
//                    .await,
//            ),
//            file => {
//                let data = self.stream_store.get_segment(file).await.unwrap();
//                let mut response = HttpResponse::Ok()
//                    .append_header((header::CACHE_CONTROL, "no-cache"))
//                    .append_header((header::ACCEPT_ENCODING, "identity"))
//                    .append_header((header::ACCEPT_RANGES, "bytes"))
//                    .append_header((header::ACCESS_CONTROL_ALLOW_HEADERS, "*"))
//                    .header(
//                        header::ACCESS_CONTROL_ALLOW_METHODS,
//                        "POST, GET, OPTIONS",
//                    )
//                    .append_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
//                    .append_header((header::ACCESS_CONTROL_MAX_AGE, "1728000"))
//                    .header(
//                        header::ACCESS_CONTROL_EXPOSE_HEADERS,
//                        "Content-Length",
//                    )
//                    .append_header((header::CONTENT_LENGTH, data.len()));
//
//                if let Some(content_type) = lookup_content_type(file.as_ref()) {
//                    response = response.append_header((header::CONTENT_TYPE, content_type));
//                }
//
//                Ok(response.body(data.into()).unwrap())
//            }
//            _ => Ok(http::Response::builder()
//                .status(http::StatusCode::NOT_FOUND)
//                .body(Body::empty())
//                .unwrap()),
//        }
    }
}
