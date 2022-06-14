use std::fmt::Display;

use async_trait::async_trait;
use hyper::Body;

#[async_trait]
pub trait RequestHandler: Send + Sync + Display {
    async fn invoke(&self, request: http::Request<Body>) -> std::io::Result<http::Response<Body>>;
    fn path() -> &'static str
    where
        Self: Sized;
}

pub type BoxedServiceHandler =
    Box<dyn Fn(http::Request<Body>) -> std::io::Result<http::Response<Body>>>;

#[macro_export]
macro_rules! service_function {
    ($name: ident, $func: expr, $uri: literal) => {
        #[derive(Debug)]
        pub struct ServiceFunction$ident {
            closure: BoxedServiceHandler,
        }

        impl ServiceFunction$ident {
            pub fn new(closure: BoxedServiceHandler) -> Self {
                Self { closure }
            }
        }

        #[async_trait]
        impl RequestHandler for ServiceFunction$ident {
            async fn invoke(
                &self,
                request: http::Request<Body>,
            ) -> std::io::Result<http::Response<Body>> {
                self.closure(request)
            }

            fn path() -> &'static str {
                $uri
            }
        }

        impl Display for ServiceFunction$ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", stringify!($name))
            }
        }

        ServiceFunction$ident::new($func)
    };
}
