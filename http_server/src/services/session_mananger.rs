use async_trait::async_trait;
use hyper::Body;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;
use super::RequestHandler;
const SESSION_ID_KEY: &str = "Session_id";

#[derive(Debug, Clone)]
pub struct SessionMananger {
    session_ids: Arc<RwLock<HashSet<u64>>>,
}

impl SessionMananger {
    pub fn new(session_ids: Arc<RwLock<HashSet<u64>>>) -> Self {
        SessionMananger { session_ids }
    }

    /// validate that the request is allowed to take place.
    /// returns `Some(session_id)` if session_id is present and exists in the list
    async fn is_valid_session(&self, cookies: Option<&str>) -> Option<u64> {
        for cookie in cookies?.split_terminator(";") {
            if let Some((key, value)) = cookie.split_once("=") {
                if key.eq(SESSION_ID_KEY) {
                    let session_id = value.parse::<u64>().unwrap();
                    return self.session_ids.read().await.get(&session_id).cloned();
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

    fn login(&self, data: &[u8]) -> bool {
        let utf8_body = std::str::from_utf8(data).unwrap();
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

        username_ok && password_ok
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
                if self.login(&body) {
                    Ok(http::Response::builder()
                        .header(http::header::SET_COOKIE, format!("SESSION_ID_KEY=12345;"))
                        .status(http::StatusCode::OK)
                        .body(Body::empty())
                        .unwrap())
                } else {
                    Ok(http::Response::builder()
                        .status(http::StatusCode::FORBIDDEN)
                        .body(Body::empty())
                        .unwrap())
                }
            }
        }
    }
}
