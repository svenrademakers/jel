use actix_service::{Service, Transform};
use actix_web::{
    body::EitherBody,
    dev::{ServiceRequest, ServiceResponse},
    Error,
};
use futures_util::future::LocalBoxFuture;
use log::debug;
use ronaldos_config::Login;
use std::future::{ready, Ready};

const SESSION_ID_KEY: &str = "Session_id";

#[derive(Debug, Clone)]
pub struct RonaldoAuthentication {
    login: Login,
}

impl RonaldoAuthentication {
    pub fn new(login: Login) -> Self {
        Self { login }
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
        ready(Ok(AuthenticationService::new(service, self.login)))
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticationService<S> {
    service: S,
    encoded: String,
}

impl<S> AuthenticationService<S> {
    pub fn new(service: S, login: Login) -> Self {
        let session_id: String = format!("username={}&password={}", login.username, login.password);
        debug!("raw session id: {}", session_id);
        let encoded = base64::encode_config(session_id, base64::URL_SAFE);
        debug!("session cookie: {}", &encoded);
        AuthenticationService { service, encoded }
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
    ) -> Result<Option<http::HeaderMap>, http::Response<Body>> {
        if request.uri().path() == "/login.html"
            || request.uri().path() == "/dologin"
            || request.uri().path().to_string().starts_with("/.well-known")
        {
            return Ok(None);
        }

        if let Some(query) = request.uri().query() {
            debug!("has query {}", query);

            match self.create_session(query.trim().as_bytes()) {
                Some(session) => {
                    return Ok(Some(cookie_header(session)));
                }
                None => {
                    debug!("query no good");
                    return Err(denied_response(request.uri()));
                }
            }
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
            true => Ok(None),
            false => Err(denied_response(request.uri())),
        }
    }

    fn create_session(&self, data: &[u8]) -> Option<String> {
        let utf8_body = std::str::from_utf8(data).unwrap();
        let encoded = base64::encode_config(utf8_body, base64::URL_SAFE);
        debug!("encoded session: {}", encoded);
        if encoded == self.encoded {
            return Some(encoded);
        }
        None
    }
}

fn cookie_header(session: String) -> HeaderMap {
    let expiration = chrono::Local::now().add(Duration::days(31));
    let mut map: HeaderMap = HeaderMap::new();
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

fn redirect_ok_response(session: &str) -> http::Response<Body> {
    let expiration = chrono::Local::now().add(Duration::days(31));
    http::Response::builder()
        .header(
            http::header::SET_COOKIE,
            format!(
                "{}={}; Secure; HttpOnly; SameSite=Strict",
                SESSION_ID_KEY, session
            ),
        )
        .header(http::header::LOCATION, "/index.html")
        .header(http::header::EXPIRES, expiration.to_rfc2822())
        .status(http::StatusCode::SEE_OTHER)
        .body(Body::empty())
        .unwrap()
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
        todo!()
    }
}
