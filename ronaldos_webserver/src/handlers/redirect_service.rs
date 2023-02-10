use std::future::{ready, Ready};

use actix_web::{
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http, Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

pub struct RedirectScheme {
    enabled: bool,
}

impl RedirectScheme {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RedirectScheme
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RedirectService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RedirectService {
            service,
            enabled: self.enabled,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct RedirectService<S> {
    service: S,
    enabled: bool,
}

impl<S, B> Service<ServiceRequest> for RedirectService<S>
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
        if self.enabled && req.connection_info().scheme() == "http" {
            let host = req.connection_info().host().to_owned();
            let uri = req.uri().to_owned();
            let url = format!("https://{}{}", host, uri);
            let response = req.into_response(
                HttpResponse::MovedPermanently()
                    .append_header((http::header::LOCATION, url))
                    .finish()
                    .map_into_right_body(),
            );
            return Box::pin(async { Ok(response) });
        }
        let res = self.service.call(req);

        Box::pin(async move {
            // forwarded responses map to "left" body
            res.await.map(ServiceResponse::map_into_left_body)
        })
    }
}
