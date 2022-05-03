use super::RequestHandler;
use async_trait::async_trait;
use chrono::Duration;
use hyper::Body;
use log::debug;
use std::{fmt::Display, ops::Add};
const SESSION_ID_KEY: &str = "Session_id";

#[derive(Debug, Clone)]
pub struct SessionMananger {
    encoded: String,
}

impl SessionMananger {
    pub fn new() -> Self {
        const SESSION_ID: &str = concat!(
            "username=",
            env!("USERNAME"),
            "&",
            "password=",
            env!("PASSWORD")
        );
        let encoded = base64::encode_config(SESSION_ID, base64::URL_SAFE);
        debug!("session cookie: {}", &encoded);

        SessionMananger { encoded }
    }

    /// validate that the request is allowed to take place.
    /// returns `Some(session_id)` if session_id is present and exists in the list
    fn cookie_contains_valid_session(&self, cookies: &str) -> bool {
        debug!("cookie: {}", cookies);
        for cookie in cookies.split_terminator(";") {
            if let Some((key, value)) = cookie.split_once("=") {
                if key.eq(SESSION_ID_KEY) {
                    return value == self.encoded;
                }
            }
        }
        return false;
    }

    /// This function checks if an request is allowed to be handled by checking
    /// if the cookie of the request contains the correct username and password.
    /// login requests are always allowed
    ///
    /// # Return
    /// * None if request is allowed
    /// * Some(response) containing a response why the request is not allowed
    pub async fn has_permission(
        &self,
        request: &http::Request<Body>,
    ) -> Option<http::Response<Body>> {
        if request.uri() == "/login.html" || request.uri() == "/dologin" {
            return None;
        }

        let valid_session = request
            .headers()
            .get("Cookie")
            .map(http::HeaderValue::to_str)
            .map(Result::ok)
            .flatten()
            .map(|c| self.cookie_contains_valid_session(c))
            .unwrap_or_default();

        match valid_session {
            true => None,
            false => Some(denied_response(request.uri())),
        }
    }

    fn create_session(&self, data: &[u8]) -> Option<String> {
        let utf8_body = std::str::from_utf8(data).unwrap();
        let encoded = base64::encode_config(utf8_body, base64::URL_SAFE);

        if encoded == self.encoded {
            return Some(encoded);
        }
        None
    }
}

fn denied_response(uri: &http::Uri) -> http::Response<Body> {
    match uri.path().ends_with("html") || uri.path().eq("/") {
        true => http::Response::builder()
            .status(http::StatusCode::TEMPORARY_REDIRECT)
            .header("Location", "/login.html")
            .body(Body::empty())
            .unwrap(),
        false => http::Response::builder()
            .status(http::StatusCode::FORBIDDEN)
            .body(Body::empty())
            .unwrap(),
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
                    let expiration = chrono::Local::now().add(Duration::days(31));

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
                    debug!("authentication failed ");
                    Ok(http::Response::builder()
                        .status(http::StatusCode::FORBIDDEN)
                        .body(Body::from("password or username not correct!"))
                        .unwrap())
                }
            }
        }
    }
}

impl Display for SessionMananger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionMananger")
    }
}
