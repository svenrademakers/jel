use actix_service::{Service, Transform};
use actix_web::{
    body::EitherBody,
    dev::{ServiceRequest, ServiceResponse},
    Error,
};
use futures_util::future::LocalBoxFuture;

#[derive(Debug, Clone)]
pub struct RonaldoAuthentication;

impl RonaldoAuthentication {
    pub fn new() -> Self {
        Self {}
    }
}

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
    encoded: String,
}

impl<S> AuthenticationService<S> {
    pub fn new(service: S) ->{
        let session_id: String = format!("username={}&password={}", login.username, login.password);
        debug!("raw session id: {}", session_id);
        let encoded = base64::encode_config(session_id, base64::URL_SAFE);
        debug!("session cookie: {}", &encoded);
        AuthenticationService { service, encoded }
    }
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

    fn call(&self, _: ServiceRequest) -> Self::Future {
        todo!()
    }
}
