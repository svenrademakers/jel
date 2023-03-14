use actix_web::{web, HttpResponse, Responder};

use crate::middleware::FootballApi;

pub fn fixture_service_config(cfg: &mut web::ServiceConfig, football_api: web::Data<FootballApi>) {
    cfg.service(
        web::resource("fixtures")
            .app_data(football_api)
            .route(web::get().to(get_all_fixtures)),
    );
}

async fn get_all_fixtures(football_info: web::Data<FootballApi>) -> impl Responder {
    let mut data = Vec::new();
    football_info.fixtures(&mut data).await.unwrap();
    HttpResponse::Ok().body(data)
}
