use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use log::debug;
use ronaldos_config::Login;
use serde::Deserialize;
use std::time::Duration;
use std::{fmt::Display, ops::Add};
const SESSION_ID_KEY: &str = "Session_id";

#[derive(Deserialize)]
pub enum Users {
    Admin,
    Viewer(u32),
}

#[derive(Debug, Clone)]
pub struct SessionMananger {
    encoded: String,
}

//impl SessionMananger {
//    pub fn new(login: &Login) -> Self {
//        let session_id: String = format!("username={}&password={}", login.username, login.password);
//        debug!("raw session id: {}", session_id);
//        let mut encoded = String::new();
//        URL_SAFE.encode_string(session_id, &mut encoded);
//        debug!("session cookie: {}", encoded);
//        SessionMananger { encoded }
//    }
//}
//    /// This function checks if an request is allowed to be handled by checking
//    /// if the cookie of the request contains the correct username and password.
//    /// login requests are always allowed
//    ///
//    /// # Return
//    /// * None if request is allowed
//    /// * Some(response) containing a response why the request is not allowed
//    pub async fn has_permission(
//        &self,
//        request: &http::Request<Body>,
//    ) -> Result<Option<http::HeaderMap>, http::Response<Body>> {
//        if request.uri().path() == "/login.html"
//            || request.uri().path() == "/dologin"
//            || request.uri().path().to_string().starts_with("/.well-known")
//        {
//            return Ok(None);
//        }
//
//        if let Some(query) = request.uri().query() {
//            debug!("has query {}", query);
//
//            match self.create_session(query.trim().as_bytes()) {
//                Some(session) => {
//                    return Ok(Some(cookie_header(session)));
//                }
//                None => {
//                    debug!("query no good");
//                    return Err(denied_response(request.uri()));
//                }
//            }
//        }
//
//        let valid_session = request
//            .headers()
//            .get("Cookie")
//            .map(http::HeaderValue::to_str)
//            .map(Result::ok)
//            .flatten()
//            .map(|c| self.cookie_contains_valid_session(c))
//            .unwrap_or_default();
//
//        match valid_session {
//            true => Ok(None),
//            false => Err(denied_response(request.uri())),
//        }
//    }
//
//    fn create_session(&self, data: &[u8]) -> Option<String> {
//        let utf8_body = std::str::from_utf8(data).unwrap();
//        let encoded = URL_SAFE.encode(utf8_body);
//        debug!("encoded session: {}", encoded);
//        if encoded == self.encoded {
//            return Some(encoded);
//        }
//        None
//    }
//}
//
//fn cookie_header(session: String) -> HeaderMap {
//    let expiration = chrono::Local::now().add(Duration::days(31));
//    let mut map: HeaderMap = HeaderMap::new();
//    map.append(
//        http::header::EXPIRES,
//        expiration.to_rfc2822().parse().unwrap(),
//    );
//    map.append(
//        http::header::SET_COOKIE,
//        format!("{}={}; Secure; HttpOnly", SESSION_ID_KEY, session)
//            .parse()
//            .unwrap(),
//    );
//    map
//}
//
//fn denied_response(uri: &http::Uri) -> http::Response<Body> {
//    match uri.path().ends_with("html") || uri.path().eq("/") {
//        true => http::Response::builder()
//            .status(http::StatusCode::TEMPORARY_REDIRECT)
//            .header("Location", "/login.html")
//            .body(Body::empty())
//            .unwrap(),
//        false => http::Response::builder()
//            .status(http::StatusCode::FORBIDDEN)
//            .body(Body::empty())
//            .unwrap(),
//    }
//}
//
//fn redirect_ok_response(session: &str) -> http::Response<Body> {
//    let expiration = chrono::Local::now().add(Duration::days(31));
//    http::Response::builder()
//        .header(
//            http::header::SET_COOKIE,
//            format!(
//                "{}={}; Secure; HttpOnly; SameSite=Strict",
//                SESSION_ID_KEY, session
//            ),
//        )
//        .header(http::header::LOCATION, "/index.html")
//        .header(http::header::EXPIRES, expiration.to_rfc2822())
//        .status(http::StatusCode::SEE_OTHER)
//        .body(Body::empty())
//        .unwrap()
//}
//
//#[async_trait]
//impl RequestHandler for SessionMananger {
//    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>> {
//        match request
//            .headers()
//            .get(http::header::CONTENT_TYPE)
//            .map(http::HeaderValue::to_str)
//            .map(Result::ok)
//            .flatten()
//        {
//            Some(x) if x.contains("multipart/form-data") => Ok(http::Response::builder()
//                .status(http::StatusCode::NOT_IMPLEMENTED)
//                .body(Body::empty())
//                .unwrap()),
//            _ => {
//                let body = hyper::body::to_bytes(request).await.unwrap();
//                let response;
//                if let Some(session) = self.create_session(&body) {
//                    response = redirect_ok_response(&session);
//                } else {
//                    debug!("authentication failed.");
//                    response = http::Response::builder()
//                        .status(http::StatusCode::FORBIDDEN)
//                        .body(Body::from("password or username not correct!"))
//                        .unwrap()
//                }
//                return Ok(response);
//            }
//        }
//    }
//
//    fn path() -> &'static str {
//        "dologin"
//    }
//}
//
//impl Display for SessionMananger {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        write!(f, "SessionMananger")
//    }
//}
