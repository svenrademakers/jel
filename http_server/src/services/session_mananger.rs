use super::RequestHandler;
use async_trait::async_trait;
use chrono::Duration;
use hyper::Body;
use once_cell::sync::Lazy;
use std::{collections::HashSet, ops::Add};
use tokio::sync::RwLock;
const SESSION_ID_KEY: &str = "Session_id";

static SESSIONS: Lazy<RwLock<HashSet<u64>>> = Lazy::new(|| RwLock::new(HashSet::new()));

#[derive(Debug, Clone, Copy)]
pub struct SessionMananger;

impl SessionMananger {
    pub const fn new() -> Self {
        SessionMananger {}
    }

    /// validate that the request is allowed to take place.
    /// returns `Some(session_id)` if session_id is present and exists in the list
    async fn is_valid_session(&self, cookies: Option<&str>) -> Option<u64> {
        for cookie in cookies?.split_terminator(";") {
            if let Some((key, value)) = cookie.split_once("=") {
                if key.eq(SESSION_ID_KEY) {
                    let session_id = value.parse::<u64>().unwrap();
                    return SESSIONS.read().await.get(&session_id).cloned();
                }
            }
        }
        return None;
    }

    pub async fn has_permission(
        &self,
        request: &http::Request<Body>,
    ) -> Option<http::Response<Body>> {
        if request.uri() == "/login.html" || request.uri() == "/dologin" {
            return None;
        }

        let cookies = request
            .headers()
            .get("Cookie")
            .map(http::HeaderValue::to_str)
            .map(Result::ok)
            .flatten();

        match self.is_valid_session(cookies).await {
            Some(_) => None,
            None => Some(
                http::Response::builder()
                    .status(http::StatusCode::TEMPORARY_REDIRECT)
                    .header("Location", "/login.html")
                    .body(Body::empty())
                    .unwrap(),
            ),
        }
    }

    fn create_session(&self, data: &[u8]) -> Option<u64> {
        let utf8_body = std::str::from_utf8(data).unwrap();
        println!("DATA: {}", utf8_body);
        let mut username_ok = false;
        let mut password_ok = false;
        for var in utf8_body.split("&") {
            if let Some((key, val)) = var.split_once('=') {
                if key == "username" && val == env!("USERNAME") {
                    username_ok = true
                }
                if key == "password" && val == env!("PASSWORD") {
                    password_ok = true
                }
            }
        }

        match username_ok && password_ok {
            true => Some(12345),
            false => None,
        }
    }
}

#[async_trait]
impl RequestHandler for SessionMananger {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
        match request
            .headers()
            .get(http::header::CONTENT_TYPE)
            .map(http::HeaderValue::to_str)
            .map(Result::ok)
            .flatten()
        {
            Some(x) if x.contains("multipart/form-data") => Ok(http::Response::builder()
                .status(http::StatusCode::NOT_IMPLEMENTED)
                .body(Body::empty())
                .unwrap()),
            _ => {
                let body = hyper::body::to_bytes(request).await.unwrap();
                if let Some(session) = self.create_session(&body) {
                    let expiration = chrono::Local::now().add(Duration::days(1));

                    Ok(http::Response::builder()
                        .header(
                            http::header::SET_COOKIE,
                            format!("{}={}; Secure; HttpOnly", SESSION_ID_KEY, session),
                        )
                        .header(http::header::LOCATION, "/index.html")
                        .header(http::header::EXPIRES, expiration.to_rfc2822())
                        .status(http::StatusCode::SEE_OTHER)
                        .body(Body::empty())
                        .unwrap())
                } else {
                    Ok(http::Response::builder()
                        .status(http::StatusCode::FORBIDDEN)
                        .body(Body::from("password or username not correct!"))
                        .unwrap())
                }
            }
        }
    }
}
