/// This crate implements a standalone datatype for HTTP status codes. `Status`
/// allows you to specify a status code by name and associate custom text and
/// headers with it, then convert that `Status` into a server response.
///
/// Currently, the only automatic conversion that is supported is for Warp.
///
use std::convert::Infallible;
pub use http::StatusCode;
use http::header::{self, HeaderMap, HeaderName, HeaderValue};
use http::response::{Builder};
#[cfg(feature = "server-warp")]
use hyper::Body;
#[cfg(feature = "server-warp")]
use warp::{
    reject::{self, Reject, Rejection},
    reply::{Reply, Response},
    Filter
};

#[cfg(test)]
mod tests {
    use super::*;
    // To test:
    #[test]
    fn new_status_contains_correct_code() {
        assert_eq!(Status::new(&StatusCode::IM_A_TEAPOT).code(), &StatusCode::IM_A_TEAPOT);
    }

    #[test]
    fn new_status_contains_correct_specified_message() {
        assert_eq!(Status::with_message(&StatusCode::IM_A_TEAPOT, String::from("foobar")).message(), Some("foobar"));
    }

    #[test]
    fn new_status_does_not_contain_message() {
        assert_eq!(Status::new(&StatusCode::IM_A_TEAPOT).message(), None);
    }

    #[test]
    fn new_status_has_empty_headers() {
        assert!(Status::new(&StatusCode::IM_A_TEAPOT).headers().is_empty());
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn status_rejection_is_a_status() {
        assert!(Status::rejection_is_status(reject::custom(Status::new(&StatusCode::IM_A_TEAPOT))));
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn non_status_rejection_is_not_status() {
        assert!(!Status::rejection_is_status(warp::reject::not_found()));
    }

    // - 5xx status does not reveal error message to client
    // - Correctly implements Warp's error type
}

pub enum Error {
    #[cfg(feature = "server-warp")]
    NotStatus(Rejection)
}

/// An HTTP status code bundled with an associated message.
///
///
#[derive(Clone, Debug)]
pub struct Status {
    c: &'static StatusCode,
    msg: Option<String>,
    h: HeaderMap<HeaderValue>,
}

impl Status {
    pub fn new(code: &'static StatusCode) -> Status {
        Status { c: code, msg: None, h: HeaderMap::new() }
    }

    pub fn with_message(code: &'static StatusCode, msg: String) -> Status {
        Status { c: code, msg: Some(msg), h: HeaderMap::new() }
    }

    pub fn code(&self) -> &StatusCode {
        &self.c
    }

    pub fn message(&self) -> Option<&str> {
        self.msg.as_ref().map(|x| x.as_str())
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.h
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.h
    }

    #[cfg(feature = "server-warp")]
    pub fn rejection_is_status(err: Rejection) -> bool {
        err.find::<Self>().is_some()
    }

    #[cfg(feature = "server-warp")]
    pub fn recover(err: Rejection) -> Result<impl Reply, Error> {
        err.find::<Self>().map(|stat| stat.clone()).ok_or(Error::NotStatus(err))
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.message() {
            None => write!(f, "{}", self.code()),
            Some(msg) => {
                if self.code().as_u16() < 500 {
                    write!(f, "{}\n{}", self.code(), msg)
                } else {
                    write!(f, "{}", self.code())
                }
            },
        }
    }
}

impl std::error::Error for Status {}

#[cfg(feature = "server-warp")]
impl From<Status> for Response {
    fn from(s: Status) -> Response {
        let mut build = Builder::new()
            .status(s.code());

        for (key, val) in s.headers().iter() {
            build = build.header(key, val)
        }

        // Unwrapping will cause a panic on error, however I am fairly certain
        // that nothing will cause building the response to error. The StatusCode
        // and HeaderName/HeaderValue types are taken directly from the same crate
        // that implements this Builder. Further, creating the hyper Body should
        // not error either.
        match s.msg {
            None => build.body(Body::empty()),
            Some(m) => build.body(Body::from(m)),
        }.unwrap()
    }
}

#[cfg(feature = "server-warp")]
impl Reject for Status {}

#[cfg(feature = "server-warp")]
impl Reply for Status {
    fn into_response(self) -> Response {
        self.into()
    }
}
