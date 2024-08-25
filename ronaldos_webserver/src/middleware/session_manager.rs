use actix_web::{http, HttpRequest};
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use tracing::debug;
use ronaldos_config::Login;
use std::ops::Add;

const SESSION_ID_KEY: &str = "Session_id";

#[derive(Debug, PartialEq)]
pub enum PermissionResult {
    Denied,
    AuthenticationNeeded,
    Ok,
}
fn allow_list(path: &str) -> bool {
    path == "/favicon.ico"
        || path == "/login.html"
        || path == "/dologin"
        || path.to_string().starts_with("/.well-known")
}

#[derive(Debug, Clone)]
pub struct SessionMananger {
    encoded: String,
}

impl SessionMananger {
    pub fn new(login: &Login) -> Self {
        let session_id: String = format!("username={}&password={}", login.username, login.password);
        debug!("raw session id: {}", session_id);
        let mut encoded = String::new();
        URL_SAFE.encode_string(session_id, &mut encoded);
        debug!("session cookie: {}", encoded);
        SessionMananger { encoded }
    }

    /// This function checks if an request is allowed to be handled by checking
    /// if the cookie of the request contains the correct username and password.
    /// login requests are always allowed
    ///
    /// # Return
    /// * None if request is allowed
    /// * Some(response) containing a response why the request is not allowed
    pub fn has_permission(&self, request: &HttpRequest) -> PermissionResult {
        if allow_list(request.path()) {
            return PermissionResult::Ok;
        }

        let result = request
            .headers()
            .get("Cookie")
            .map(http::header::HeaderValue::to_str)
            .map(Result::ok)
            .flatten()
            .map(|str| {
                let mut s = String::new();
                URL_SAFE.encode_string(str, &mut s);
                if s == self.encoded {
                    PermissionResult::Ok
                } else {
                    PermissionResult::AuthenticationNeeded
                }
            })
            .unwrap_or(PermissionResult::Denied);

        if result == PermissionResult::Denied && request.path() == "/" {
            return PermissionResult::AuthenticationNeeded;
        }
        result
    }

    fn create_session(&self, data: &[u8]) -> Option<String> {
        let utf8_body = std::str::from_utf8(data).unwrap();
        let encoded = URL_SAFE.encode(utf8_body);
        debug!("encoded session: {}", encoded);
        if encoded == self.encoded {
            return Some(encoded);
        }
        None
    }
}

fn cookie_header(session: String) -> http::header::HeaderMap {
    let expiration = chrono::Local::now().add(chrono::Duration::days(31));
    let mut map = http::header::HeaderMap::new();
    map.append(
        http::header::EXPIRES,
        expiration.to_rfc2822().parse().unwrap(),
    );
    map.append(
        http::header::SET_COOKIE,
        format!("{}={}; Secure; HttpOnly", SESSION_ID_KEY, session)
            .parse()
            .unwrap(),
    );
    map
}

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
