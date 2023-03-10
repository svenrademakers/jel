use actix_limitation::Status;
use actix_service::{Service, Transform};
use actix_web::{
    body::EitherBody,
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{self, ready, Ready};

#[derive(Debug, Clone, Default)]
pub struct RonaldoAuthentication;

impl<S, B> Transform<S, ServiceRequest> for RonaldoAuthentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationService { service }))
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticationService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        if !allow_list(request.uri().path()) {
            return Box::pin(future::ready(
                Ok(ServiceResponse::new(
                    request.request().clone(),
                    HttpResponse::Unauthorized().finish(),
                ))
                .map(ServiceResponse::map_into_right_body),
            ));
        }
        let res = self.service.call(request);
        Box::pin(async move {
            // forwarded responses map to "left" body
            res.await.map(ServiceResponse::map_into_left_body)
        })
    }
}

fn allow_list(path: &str) -> bool {
    path == "/favicon.ico"
        || path == "/login.html"
        || path == "/dologin"
        || path.to_string().starts_with("/.well-known")
}
