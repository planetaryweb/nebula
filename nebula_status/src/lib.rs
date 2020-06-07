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
};

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "server-warp")]
    use warp::reject;

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
        assert!(Status::<Empty>::rejection_is_status(&reject::custom(Status::new(
            &StatusCode::IM_A_TEAPOT
        ))));
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn non_status_rejection_is_not_status() {
        assert!(!Status::<Empty>::rejection_is_status(&reject::not_found()));
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn rejection_from_status() {
        let data = vec![0u8, 1u8, 2u8, 3u8, 4u8];
        let status = Status::with_data(&StatusCode::IM_A_TEAPOT, data.clone());
        
        let rej = reject::Rejection::from(status.clone());
        let rej_status = rej.find::<Status<Vec<u8>>>().unwrap();

        assert_eq!(status, rej_status);
    }

    // - 5xx status does not reveal error message to client
    // - Correctly implements Warp's error type
}

/// An enumerated list of possible errors returned by this crate and related data.
#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "server-warp")]
    /// The Rejection you attempted to recover is not an instance of Status.
    NotStatus(Rejection),
}

/// A trait alias for marking associated data inside of a container with the 
/// traits necessary to be used.
pub trait StatusInnerData: Clone + Debug + Send + Sync + 'static {}
impl<T: Clone + Debug + Send + Sync + 'static> StatusInnerData for T {}

/// A trait alias for marking associated data with the traits necessary to be
/// used.
pub trait StatusData: Into<Bytes> + StatusInnerData {}
impl<T: Into<Bytes> + StatusInnerData> StatusData for T {}

/// An empty type used by a Status without associated data.
#[derive(Clone, Debug)]
pub struct Empty;

impl Into<Bytes> for Empty {
    fn into(self) -> Bytes {
        Bytes::new()
    }
}

/// An HTTP status code bundled with associated data.
///
/// Code that creates a new instance of Status should set any related response
/// headers before returning it.
// TODO: Genericize the data member into anything that can be converted into bytes?
#[derive(Clone, Debug)]
pub struct Status<T = Empty>
where
    T: StatusData,
{
    c: &'static StatusCode,
    data: Option<T>,
    data_bytes: Option<Bytes>,
    h: HeaderMap<HeaderValue>,
}

impl Status {
    /// Create a new Status without any associated data. This will be converted to
    /// the specified status code with associated headers and no body.
    pub fn new(code: &'static StatusCode) -> Status<Empty> {
        Status {
            c: code,
            data: None,
            data_bytes: None,
            h: HeaderMap::new(),
        }
    }

    /// Create a new Status with associated data of type String. Useful for
    /// returning basic error messages.
    pub fn with_message(code: &'static StatusCode, msg: String) -> Status<String> {
        let mut status = Status::with_data(code, msg);
        status.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime::TEXT_PLAIN_UTF_8.as_ref()).unwrap(),
        );
        status
    }

    /// Create a new Status with associated arbitrary data. Useful for
    /// returning a struct that can be serialized into e.g. JSON.
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

    /// Returns an option containing a reference to the contained data, if
    /// any.
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
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
    /// Attempts to recover the Rejection as an instance of Status. Returns
    /// Error::NotStatus if the Rejection does not implement Status.
    // TODO: Example usage
    pub fn recover(err: &Rejection) -> std::result::Result<Self, Error> {
        err.find::<Self>()
            .map(|stat| stat.clone())
            .ok_or(Error::NotStatus(err))
    }
}

#[cfg(feature = "server-warp")]
impl <T: StatusData> From<Status<T>> for Rejection {
    fn from(status: Status<T>) -> Self {
        reject::custom(status)
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
