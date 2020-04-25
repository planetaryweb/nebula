use bytes::Bytes;
use http::header::{self, HeaderMap, HeaderValue};
#[cfg(feature = "server-warp")]
use http::response::Builder;
pub use http::StatusCode;
#[cfg(feature = "server-warp")]
use hyper::Body;
/// This crate implements a standalone datatype for HTTP status codes. `Status`
/// allows you to specify a status code by name and associate custom data and
/// headers with it, then convert that `Status` into a server response.
///
/// Currently, the only automatic conversion that is supported is for Warp.
///
use std::fmt::Debug;
#[cfg(feature = "server-warp")]
use warp::{
    reject::{self, Reject, Rejection},
    reply::{Reply, Response},
    Filter,
};

#[cfg(test)]
mod tests {
    use super::*;
    // To test:
    #[test]
    fn new_status_contains_correct_code() {
        assert_eq!(
            Status::new(&StatusCode::IM_A_TEAPOT).code(),
            &StatusCode::IM_A_TEAPOT
        );
    }

    #[test]
    fn new_status_contains_correct_specified_message() {
        assert_eq!(
            Status::with_message(&StatusCode::IM_A_TEAPOT, String::from("foobar")).message(),
            Some("foobar")
        );
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
    fn server_error_does_not_contain_error_message() {
        let server_msg = "foobar";
        let status =
            Status::with_message(&StatusCode::INTERNAL_SERVER_ERROR, String::from(server_msg));
        let client_msg = status.to_string();
        assert!(!client_msg.contains(server_msg));
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn status_rejection_is_a_status() {
        assert!(Status::rejection_is_status(reject::custom(Status::new(
            &StatusCode::IM_A_TEAPOT
        ))));
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn non_status_rejection_is_not_status() {
        assert!(!Status::rejection_is_status(warp::reject::not_found()));
    }

    // - 5xx status does not reveal error message to client
    // - Correctly implements Warp's error type
}

/// An enumerated list of possible errors returned by this crate and related data.
pub enum Error {
    #[cfg(feature = "server-warp")]
    /// The Rejection you attempted to recover is not an instance of Status.
    NotStatus(Rejection),
}

pub trait StatusData: Into<Bytes> + Clone + Debug + Send + Sync {}
impl<T: Into<Bytes> + Clone + Debug + Send + Sync> StatusData for T {}

/// An HTTP status code bundled with associated data.
///
///
// TODO: Genericize the data member into anything that can be converted into bytes?
#[derive(Clone, Debug)]
pub struct Status<T = String>
where
    T: StatusData,
{
    c: &'static StatusCode,
    data: Option<T>,
    data_bytes: Option<Bytes>,
    h: HeaderMap<HeaderValue>,
}

impl Status {
    pub fn new(code: &'static StatusCode) -> Status {
        Status {
            c: code,
            data: None,
            data_bytes: None,
            h: HeaderMap::new(),
        }
    }

    /// Create a new Status with an associated message.
    pub fn with_message(code: &'static StatusCode, msg: String) -> Status {
        let mut status = Status::with_data(code, msg);
        status.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime::TEXT_PLAIN_UTF_8.as_ref()).unwrap(),
        );
        status
    }

    /// Create a new Status with associated arbitrary data.
    pub fn with_data<T: StatusData>(code: &'static StatusCode, data: T) -> Status<T> {
        Status {
            c: code,
            data: Some(data.clone()),
            data_bytes: Some(data.into()),
            h: HeaderMap::new(),
        }
    }
}

impl<T: StatusData> Status<T> {
    /// Gain a reference to this Status' status code.
    pub fn code(&self) -> &StatusCode {
        &self.c
    }

    /// Attempts to parse the bytes contained within the Status as a &str.
    fn data_as_message(&self) -> Option<&str> {
        // If there is data and it can successfully be parsed as a string,
        // return the parsed string. Otherwise, return None, ignoring any
        // errors while parsing.
        match self.data_bytes.as_ref() {
            None => None,
            Some(data) => std::str::from_utf8(data.as_ref()).ok(),
        }
    }

    /// Attempts to parse the data contained in this Status as a &str.
    ///
    /// The `Content-Type` header is used to help determine if the data is meant
    /// to be parsed as text or not.
    pub fn message(&self) -> Option<&str> {
        if self.h.contains_key(header::CONTENT_TYPE) {
            match self.h.get(header::CONTENT_TYPE).unwrap().to_str() {
                Err(_) => None,
                Ok(content_type) => match content_type.parse::<mime::Mime>().ok() {
                    None => None,
                    Some(mime_type) => match mime_type.type_() {
                        mime::TEXT => self.data_as_message(),
                        _ => {
                            if mime_type == mime::APPLICATION_JSON {
                                self.data_as_message()
                            } else {
                                None
                            }
                        }
                    },
                },
            }
        } else {
            None
        }
    }

    /// Gain an immutable view into the headers map.
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.h
    }

    /// Gain a mutable reference to the headers map.
    pub fn headers_mut(&mut self) -> &mut HeaderMap<HeaderValue> {
        &mut self.h
    }

    #[cfg(feature = "server-warp")]
    /// Returns `true` if the warp Rejection is an instance of Status.
    pub fn rejection_is_status(err: &Rejection) -> bool {
        err.find::<Self>().is_some()
    }

    #[cfg(feature = "server-warp")]
    /// Attempts to recover the Rejection as an instnace of Status. Returns
    /// Error::NotStatus if the Rejection does not implement Status.
    // TODO: Example usage
    pub fn recover(err: Rejection) -> std::result::Result<impl Reply, Error> {
        err.find::<Self>()
            .map(|stat| stat.clone())
            .ok_or(Error::NotStatus(err))
    }
}

impl<T: StatusData> std::fmt::Display for Status<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.message() {
            None => write!(f, "{}", self.code()),
            Some(msg) => {
                if self.code().as_u16() < 500 {
                    write!(f, "{}\n{}", self.code(), msg)
                } else {
                    write!(f, "{}", self.code())
                }
            }
        }
    }
}

impl<T: StatusData> std::error::Error for Status<T> {}

impl<T: StatusData> From<Status<T>> for Result<Status<T>, Status<T>> {
    fn from(s: Status<T>) -> Result<Status<T>, Status<T>> {
        if s.code().as_u16() < 400 {
            Ok(s)
        } else {
            Err(s)
        }
    }
}

#[cfg(feature = "server-warp")]
impl<T: StatusData> From<Status<T>> for Response {
    fn from(s: Status<T>) -> Response {
        let mut build = Builder::new().status(s.code());

        for (key, val) in s.headers().iter() {
            build = build.header(key, val)
        }

        // Unwrapping will cause a panic on error, however I am fairly certain
        // that nothing will cause building the response to error. The StatusCode
        // and HeaderName/HeaderValue types are taken directly from the same crate
        // that implements this Builder. Further, creating the hyper Body should
        // not error either.
        match s.data_bytes {
            None => build.body(Body::empty()),
            Some(m) => build.body(Body::from(m)),
        }
        .unwrap()
    }
}

#[cfg(feature = "server-warp")]
impl<T: StatusData> Reject for Status<T> {}

#[cfg(feature = "server-warp")]
impl<T: StatusData> Reply for Status<T> {
    fn into_response(self) -> Response {
        self.into()
    }
}
