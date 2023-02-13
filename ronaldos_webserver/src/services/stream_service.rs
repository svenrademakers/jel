use crate::middleware::LocalStreamStore;
use actix_web::{
    http::{self, header, StatusCode},
    post, web, Error, HttpResponse, Responder,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const STREAM_SCOPE: &'static str = "/streams";

pub fn stream_service_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope(STREAM_SCOPE)
            .route(
                "/",
                web::method(http::Method::OPTIONS).to(preflight_response),
            )
            .route("/test", web::get().to(insert_video_stub))
            .route("/all", web::get().to(get_all_streams)),
    );
}

async fn insert_video_stub(
    store: web::Data<LocalStreamStore>,
    cfg: web::Data<ronaldos_config::Config>,
) -> HttpResponse {
    if !cfg.verbose() {
        return HttpResponse::MethodNotAllowed().into();
    }
    static GTEST_VID : &str = "https://commondatastorage.googleapis.com/gtv-videos-bucket/CastVideos/hls/DesigningForGoogleCast.m3u8";
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let number = COUNTER.fetch_add(1, Ordering::Relaxed);
    let test_description = format!("this is a test#{}", number);

    store
        .register(
            test_description,
            vec![PathBuf::from(GTEST_VID), PathBuf::from("test1.m3u8")],
            chrono::Utc::now(),
        )
        .await
        .unwrap();

    HttpResponse::Ok().into()
}

async fn get_all_streams(store: web::Data<LocalStreamStore>) -> impl Responder {
    web::Json(store.get_available_streams(STREAM_SCOPE).await)
}

async fn preflight_response() -> HttpResponse {
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
