use crate::middleware::LocalStreamStore;
use actix_web::{
    get,
    http::{self, header, StatusCode},
    web, HttpResponse, Responder,
};
use std::ffi::OsStr;
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

#[get("{file:.*}")]
async fn get_segment(file: web::Path<String>, store: web::Data<LocalStreamStore>) -> HttpResponse {
    let file = file.into_inner();
    let data = store.get_segment(&file).await.unwrap();
    let content_type = match lookup_content_type(&file) {
        Some(content_type) => content_type,
        None => {
            return HttpResponse::NotImplemented().into();
        }
    };

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
        .append_header((header::CONTENT_TYPE, content_type))
        .body(data)
}

fn lookup_content_type<T: Into<PathBuf>>(path: T) -> Option<&'static str> {
    match path.into().extension().and_then(OsStr::to_str) {
        Some("m3u8") => Some("application/x-mpegURL"),
        Some("mp4") => Some("video/mp4"),
        Some("ts") => Some("video/mp2t"),
        _ => None,
    }
}
