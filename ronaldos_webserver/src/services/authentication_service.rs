use actix_limitation::Status;
use actix_service::{Service, Transform};
use actix_web::{
    body::EitherBody,
    dev::{ServiceRequest, ServiceResponse},
    http, Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{self, ready, Ready},
    sync::Arc,
};

use crate::middleware::{PermissionResult, SessionMananger};

pub struct RonaldoAuthentication {
    session_mananger: Option<SessionMananger>,
}

impl RonaldoAuthentication {
    pub fn new(session_mananger: Option<SessionMananger>) -> Self {
        Self { session_mananger }
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
        ready(Ok(AuthenticationService {
            service,
            session_mananger: self.session_mananger.clone(),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticationService<S> {
    service: S,
    session_mananger: Option<SessionMananger>,
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
        // if no session manager is defined, just pass through the request
        let Some(ref authenticator) = self.session_mananger else {
                let res = self.service.call(request);
                return Box::pin(async move {
                    // forwarded responses map to "left" body
                    res.await.map(ServiceResponse::map_into_left_body)
                })
        };

        match authenticator.has_permission(request.request()) {
            crate::middleware::PermissionResult::Ok => {
                let res = self.service.call(request);
                Box::pin(async move {
                    // forwarded responses map to "left" body
                    res.await.map(ServiceResponse::map_into_left_body)
                })
            }
            PermissionResult::AuthenticationNeeded => {
                return Box::pin(future::ready(
                    Ok(ServiceResponse::new(
                        request.request().clone(),
                        HttpResponse::TemporaryRedirect()
                            .insert_header((http::header::LOCATION, "/login.html"))
                            .finish(),
                    ))
                    .map(ServiceResponse::map_into_right_body),
                ))
            }
            PermissionResult::Denied => {
                return Box::pin(future::ready(
                    Ok(ServiceResponse::new(
                        request.request().clone(),
                        HttpResponse::Forbidden().finish(),
                    ))
                    .map(ServiceResponse::map_into_right_body),
                ))
            }
            PermissionResult::Ok => todo!(),
        }
    }
}
