use crate::middleware::LocalStreamStore;
use actix_web::{
    get,
    http::{self, header, StatusCode},
    post,
    web::{self, service},
    Error, HttpRequest, HttpResponse, Responder,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

const STREAM_SCOPE: &'static str = "/streams";

pub fn stream_service_config(
    cfg: &mut web::ServiceConfig,
    stream_store: web::Data<LocalStreamStore>,
) {
    cfg.service(
        web::scope(STREAM_SCOPE)
            .app_data(stream_store)
            .route(
                "/",
                web::method(http::Method::OPTIONS).to(preflight_response),
            )
            .route("/test", web::get().to(insert_video_stub))
            .route("/all", web::get().to(get_all_streams))
            .service(get_segment),
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

#[get("/{segment_path}")]
async fn get_segment(file: web::Path<String>, store: web::Data<LocalStreamStore>) -> HttpResponse {
    let data = store.get_segment(file.into_inner()).await.unwrap();
    HttpResponse::Ok()
        .append_header((header::CACHE_CONTROL, "no-cache"))
        .append_header((header::ACCEPT_ENCODING, "identity"))
        .append_header((header::ACCEPT_RANGES, "bytes"))
        .append_header((header::ACCESS_CONTROL_ALLOW_HEADERS, "*"))
        .append_header((header::ACCESS_CONTROL_ALLOW_METHODS, "POST, GET, OPTIONS"))
        .append_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
        .append_header((header::ACCESS_CONTROL_MAX_AGE, "1728000"))
        .append_header((header::ACCESS_CONTROL_EXPOSE_HEADERS, "Content-Length"))
        .append_header((header::CONTENT_LENGTH, data.len()))
        .body(data)
    // if let Some(content_type) = lookup_content_type(file.as_ref()) {
    //     response = response.append_header((header::CONTENT_TYPE, content_type));
    // }
}
