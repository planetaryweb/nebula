use std::collections::HashMap;
#[cfg(feature = "warp")]
use std::error::Error;
#[cfg(feature = "warp")]
use std::fmt::{self, Display, Formatter};
use std::str;
#[cfg(feature = "warp")]
use bytes::Buf;
use bytes::Bytes;
#[cfg(feature = "warp")]
use futures::{StreamExt, TryStreamExt};
#[cfg(feature = "warp")]
use futures::stream::Stream;
#[cfg(feature = "warp")]
use warp::Filter;
#[cfg(feature = "warp")]
use warp::filters::multipart::{FormData, Part};
#[cfg(feature = "warp")]
use warp::reject::{Reject, Rejection};
use urlencoding;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use futures::executor::block_on;

    fn get_foo(boundary: &[u8]) -> (Vec<u8>, HashMap<String, String>) {
        let mut expected = Vec::new();
        expected.extend_from_slice(b"--");
        expected.extend_from_slice(boundary);
        expected.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"foo\"");
        expected.extend_from_slice(b"\r\n\r\nThe contents of foo.");
        expected.extend_from_slice(b"\r\n");

        let mut hash = HashMap::new();
        hash.insert(String::from("foo"), String::from("The contents of foo."));

        (expected, hash)
    }

    fn get_bar(boundary: &[u8]) -> (Vec<u8>, HashMap<String, String>) {
        let mut expected = Vec::new();
        expected.extend_from_slice(b"--");
        expected.extend_from_slice(boundary);
        expected.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"bar\"");
        expected.extend_from_slice(b"\r\n\r\nBar has content too!");
        expected.extend_from_slice(b"\r\n");

        let mut hash = HashMap::new();
        hash.insert(String::from("bar"), String::from("Bar has content too!"));

        (expected, hash)
    }

    fn get_baz(boundary: &[u8]) -> (Vec<u8>, HashMap<String, Field>) {
        let mut expected = Vec::new();
        expected.extend_from_slice(b"--");
        expected.extend_from_slice(boundary);
        expected.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"baz\"; filename=\"baz.txt\"");
        expected.extend_from_slice(b"\r\nContent-type: text/plain");
        expected.extend_from_slice(b"\r\n\r\nBaz is a text file with this content.");
        expected.extend_from_slice(b"\r\n");

        let mut hash = HashMap::new();
        hash.insert(String::from("baz"), Field::File(FormFile {
            filename: String::from("baz.txt"),
            content_type: String::from("text/plain"),
            bytes: Bytes::from_static(b"Baz is a text file with this content."),
        }));

        (expected, hash)
    }

    fn get_end(boundary: &[u8]) -> Vec<u8> {
        let mut expected = Vec::new();
        expected.clear();
        expected.extend_from_slice(b"--");
        expected.extend_from_slice(boundary);
        expected.extend_from_slice(b"--");
        expected
    }

    #[test]
    fn form_as_multipart_no_files() {
        let boundary = b"--ultrasupercoolboundary--";

        let (foo_bytes, foo_map) = get_foo(boundary);
        let (bar_bytes, bar_map) = get_bar(boundary);
        let end = get_end(boundary);

        let mut form = Form::new();
        form.extend_from_strings(foo_map.into_iter());
        form.extend_from_strings(bar_map.into_iter());

        let result = form.to_multipart_bytes(boundary);

        assert!(result.as_slice().windows(foo_bytes.len()).any(|win| win == foo_bytes.as_slice()));

        assert!(result.as_slice().windows(bar_bytes.len()).any(|win| win == bar_bytes.as_slice()));

        assert_eq!(&result[(result.len() - end.len())..], end.as_slice());

        assert_eq!(result.len(), foo_bytes.len() + bar_bytes.len() + end.len());
    }

    #[test]
    fn multipart_as_bytes_files() {
        let boundary = b"--ultrasupercoolboundary--";

        let (foo_bytes, foo_map) = get_foo(boundary);
        let (baz_bytes, baz_map) = get_baz(boundary);
        let end = get_end(boundary);

        let mut form = Form::with_capacity(foo_map.len() + baz_map.len());

        form.extend_from_strings(foo_map.into_iter());
        form.extend(baz_map.into_iter());

        let result = form.to_multipart_bytes(boundary);

        assert!(result.as_slice().windows(foo_bytes.len()).any(|win| win == foo_bytes.as_slice()));

        assert!(result.as_slice().windows(baz_bytes.len()).any(|win| win == baz_bytes.as_slice()));

        assert_eq!(&result[(result.len() - end.len())..], end.as_slice());

        assert_eq!(result.len(), foo_bytes.len() + baz_bytes.len() + end.len());
    }

    fn mock_form(with_files: bool) -> (String, Form) {
        let boundary = "------mockboundaryvalue";

        let (_, foo_map) = get_foo(boundary.as_bytes());
        let (_, bar_map) = get_bar(boundary.as_bytes());

        let mut form = Form::with_capacity(foo_map.len() + bar_map.len());
        form.extend_from_strings(foo_map.into_iter());
        form.extend_from_strings(bar_map.into_iter());

        let files = if with_files {
            let (_, baz_map) = get_baz(boundary.as_bytes());
            baz_map
        } else {
            HashMap::new()
        };

        form.extend(files.into_iter());

        (String::from(boundary), form)
    }

    #[cfg(feature = "warp")]
    fn mock_warp_request(boundary: &str, body: &[u8]) -> Form {
        let filter = warp::filters::multipart::form()
                         .map(|data| Form::try_from_formdata(data));

        let result = warp::test::request()
            .method("POST")
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .header("Content-Length", format!("{}", body.len()))
            .body(body)
            .filter(&filter);

        let temp = block_on(result);
        block_on(temp.unwrap()).unwrap()
    }

    #[test]
    fn multipart_try_from_no_files() {
        let (boundary, form) = mock_form(false);
        let body = form.to_multipart_bytes(boundary.as_bytes());

        let result = mock_warp_request(&boundary, &body);

        assert_eq!(form, result);
    }

    #[test]
    fn multipart_try_from_files() {
        let (boundary, form) = mock_form(true);
        let body = form.to_multipart_bytes(boundary.as_bytes());

        let result = mock_warp_request(&boundary, &body);

        assert_eq!(form, result);
    }

    #[test]
    fn form_fields_to_query_string() {
        let mut fields = Form::new();
        fields.insert("foo", Field::Text(String::from("bar")));
        fields.insert("bar", Field::Text(String::from("baz")));
        fields.insert("baz", Field::Text(String::from("12")));

        let qstr: String = fields.to_url_encoded().unwrap();

        for (key, val) in fields.iter() {
            match val {
                Field::Text(val) => assert!(qstr.contains(&String::from(format!("{}={}", key, val)))),
                Field::File(_) => assert!(false),
            }
        }

        let fields_len: usize = fields.iter()
            .map(|(key, val)| key.len() + match val {
                Field::Text(val) => val.len(),
                Field::File(_) => panic!("there should not be a Form::Field here"),
            })
            .sum();

        // fields_len is the length of just the key and value strings
        // fields.len() is the number of key-value pairs (and thus "=")
        // fields.len() - 1 is the number of "&" between the pairs
        assert_eq!(qstr.len(), fields_len + fields.len() + fields.len() - 1)
    }

    #[test]
    #[cfg(feature = "warp")]
    fn wrap_form_form_fields() {
        let (_, urlenc_form) = mock_form(false);
        let filter = form_filter();
        let req = warp::test::request()
            .method("POST")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(urlenc_form.to_url_encoded().unwrap().as_bytes())
            .filter(&filter);
        assert_eq!(block_on(req).unwrap(), urlenc_form);
    }

    #[test]
    #[cfg(feature = "warp")]
    fn wrap_form_multipart_no_file() {
        let (boundary, multipart) = mock_form(false);
        let filter = form_filter();
        let req = warp::test::request()
            .method("POST")
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .body(multipart.to_multipart_bytes(boundary.as_bytes()))
            .filter(&filter);
        assert_eq!(block_on(req).unwrap(), multipart);
    }

    #[test]
    #[cfg(feature = "warp")]
    fn wrap_form_multipart_with_file() {
        let (boundary, multipart) = mock_form(true);
        let filter = form_filter();
        let req = warp::test::request()
            .method("POST")
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .body(multipart.to_multipart_bytes(boundary.as_bytes()))
            .filter(&filter);
        assert_eq!(block_on(req).unwrap(), multipart);
    }

    #[test]
    #[cfg(feature = "warp")]
    fn wrap_form_multipart_failure() {
        // Not sure how to test this...
    }
}

/// Represents a single file submitted through a form
#[derive(Clone, Debug, PartialEq)]
pub struct FormFile {
    /// The original name of the file
    pub filename: String,
    /// The content type of the file, e.g. `text/plain`.
    pub content_type: String,
    /// The bytes that make up the file's content.
    ///
    /// These bytes should be interpreted based on the file's `content_type`.
    pub bytes: Bytes,
}

/// Represents the contents of a single field of the submitted form.
///
/// A `File` field corresponds to HTML form fields with `type="file"`,
/// while `Text` fields correspond to all others, which can be
/// represented as a string.
#[derive(Clone, Debug, PartialEq)]
pub enum Field {
    Text(String),
    File(FormFile),
}

impl Field {
    #[cfg(feature = "warp")]
    /// A helper function that coalesces a `Buf` `Stream` into the bytes being
    /// streamed.
    ///
    /// Requires `features = "warp"`.
    async fn buf_to_bytes(strm: impl Stream<Item = Result<impl Buf, warp::Error>>) -> Result<Bytes, warp::Error> {
        Ok(Bytes::from(strm.try_fold(Vec::new(), |mut vec, data| {
            vec.extend_from_slice(data.bytes());
            async move { Ok(vec) }
        }).await?))
    }

    #[cfg(feature = "warp")]
    /// Attempts to create a `Field` instance from the provided `Part`.
    ///
    /// Requires `features = "warp"`.
    pub async fn try_from_async(part: Part) -> Result<(String, Self), String> {
        let name = String::from(part.name());
        let filename = part.filename().map(|f| f.to_string());
        let content_type = part.content_type().map(|c| c.to_string());

        let content = match Self::buf_to_bytes(part.stream()).await {
            Ok(content) => content,
            Err(err) => return Err(String::from(err.description())),
        };

        match filename {
            None => {
                match String::from_utf8(content.to_vec()) {
                    Ok(s) => Ok((name, Field::Text(s))),
                    Err(e) => Err(String::from(e.description())),
                }
            },
            Some(filename) => {
                match content_type {
                    None => Err("form field has filename but no content type".to_string()),
                    Some(content_type) => {
                        Ok((name, Field::File(FormFile {
                            filename: String::from(filename),
                            content_type: String::from(content_type),
                            bytes: content,
                        })))
                    }
                }
            }
        }
    }
}

/// Represents the entire contents of a submitted form.
#[derive(Debug, PartialEq)]
pub struct Form(HashMap<String, Field>);

impl Form {
    /// Creates a new empty form instance.
    pub fn new() -> Form {
        Form(HashMap::new())
    }

    /// Creates a new empty form instance with the given capacity.
    pub fn with_capacity(cap: usize) -> Form {
        Form(HashMap::with_capacity(cap))
    }

    /// Adds a new `Field` to the `Form`. Returns the previous `Field`, if
    /// there was one.
    pub fn insert(&mut self, name: &str, field: Field) -> Option<Field> {
        self.0.insert(String::from(name), field)
    }

    /// If a `Field` exists with the given `name`, it is removed and returned.
    /// Otherwise, nothing happens and `None` is returned.
    pub fn remove(&mut self, name: &str) -> Option<Field> {
        self.0.remove(name)
    }

    /// Empties the contents of the `Form`.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Returns a reference to the field contents, if the field exists.
    pub fn get(&self, name: &str) -> Option<&Field> {
        self.0.get(name)
    }

    /// Append the contents of a map to the current `Form`. Fields that already
    /// exist will be overwritten.
    pub fn extend(&mut self, iter: impl Iterator<Item=(String, Field)>) {
        for (name, field) in iter {
            self.insert(&name, field);
        }
    }

    /// Append the contents of a map to the current `Form`, converting the
    /// `String` values to a `Field::Text`. Fields that already exist will
    /// be overwritten.
    pub fn extend_from_strings(&mut self, iter: impl Iterator<Item=(String, String)>) {
        self.extend(
            iter.map(|(k, v)| (k, Field::Text(v)))
        );
    }

    // Information getters

    /// Indicates whether this `Form` contains a field with the given name.
    pub fn contains_field(&self, field: &str) -> bool {
        self.0.contains_key(field)
    }

    /// Indicates whether the `Form` is empty (i.e., has no fields).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the length of the `Form` (i.e., the number of fields).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    // Iteration

    /// Returns an iterator over every field in the form.
    pub fn iter(&self) -> impl Iterator<Item=(&String, &Field)> {
        self.0.iter()
    }

    /// Returns an iterator over every text field in the form.
    pub fn iter_text(&self) -> impl Iterator<Item=(&String, &Field)> {
        self.iter().filter(|(_name, field)| match field {
            Field::Text(_) => true,
            Field::File(_) => false,
        })
    }

    /// Returns an iterator over every file field in the form.
    pub fn iter_files(&self) -> impl Iterator<Item=(&String, &Field)> {
        self.iter().filter(|(_name, field)| match field {
            Field::Text(_) => false,
            Field::File(_) => true,
        })
    }

    // # Conversions
    /// Returns the `Form` to a URL encoded format, suitable for `GET` requests
    /// or the body of a `Content-Type: application/x-www-form-urlencoded` `POST` request.
    pub fn to_url_encoded(&self) -> Result<String, String> {
        let mut builder = Vec::new();

        for (name, val) in &self.0 {
            match val {
                Field::File(_) => return Err(String::from(format!("Cannot include field {} as text", name))),
                Field::Text(txt) => {
                    let enc_key = urlencoding::encode(name);
                    let enc_val = urlencoding::encode(&txt);
                    builder.push(String::from(format!("{}={}", enc_key, enc_val)));
                }
            }
        }

        Ok(builder.join("&"))
    }

    /// Returns the `Form` in multipart format, i.e. the format suitable for
    /// the body of a request with `Content-Type: multipart/form-data`.
    pub fn to_multipart_bytes(&self, boundary: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();

        for (name, field) in self.iter() {
            buf.extend_from_slice(b"--");
            buf.extend_from_slice(boundary);
            buf.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"");
            buf.extend_from_slice(name.as_bytes());

            match field {
                Field::Text(txt) => {
                    buf.extend_from_slice(b"\"\r\n\r\n");
                    buf.extend_from_slice(txt.as_bytes());
                },
                Field::File(file) => {
                    buf.extend_from_slice(b"\"; filename=\"");
                    buf.extend_from_slice(file.filename.as_bytes());
                    buf.extend_from_slice(b"\"\r\nContent-type: ");
                    buf.extend_from_slice(file.content_type.as_bytes());
                    buf.extend_from_slice(b"\r\n\r\n");
                    buf.extend_from_slice(&file.bytes);
                }
            }

            buf.extend_from_slice(b"\r\n");
        }

        buf.extend_from_slice(b"--");
        buf.extend_from_slice(boundary);
        buf.extend_from_slice(b"--");

        buf
    }

    #[cfg(feature = "warp")]
    /// Attempts to consume a Warp `FormData` stream and return a `Form` built
    /// from its contents.
    ///
    /// Requires `features = "warp"`.
    async fn try_from_formdata(mut data: FormData) -> Result<Self, String> {
        let mut form = Form::new();

        while let Some(part) = data.next().await {
            match part {
                Err(err) => return Err(String::from(err.description())),
                Ok(part) => {
                    let (name, field) = Field::try_from_async(part).await?;
                    form.insert(&name, field)
                }
            };
        }

        Ok(form)
    }
}

impl From<HashMap<String, String>> for Form {
    fn from(map: HashMap<String, String>) -> Self {
        let mut form = Form::with_capacity(map.capacity());
        for (key, val) in map.iter() {
            form.insert(key, Field::Text(val.clone()));
        }
        form
    }
}

#[derive(Debug)]
#[cfg(feature = "warp")]
///
/// Requires `features = "warp"`.
struct RejectionWrapper {
    msg: String,
}

#[cfg(feature = "warp")]
impl Display for RejectionWrapper {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[cfg(feature = "warp")]
impl Error for RejectionWrapper {
    fn description(&self) -> &str {
        &self.msg
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[cfg(feature = "warp")]
impl Reject for RejectionWrapper {}

#[cfg(feature = "warp")]
/// Returns a `Filter` that reads a form as either a URL-encoded request body
/// or a `multipart/form-data` body, parses it as necessary, and returns a
/// `Form` object.
///
/// Requires `features = "warp"`.
pub fn form_filter() -> impl Filter<Extract = (Form,), Error = Rejection> {
    warp::filters::body::form().map(|f: HashMap<String, String>| Form::from(f))
        .or(
            warp::filters::multipart::form().and_then(|f: FormData| async move {
                    match Form::try_from_formdata(f).await {
                        Ok(form) => Ok(form),
                        Err(err) => Err(warp::reject::custom(RejectionWrapper{msg: err.to_string()}))
                    }
                }
            )
        ).unify()
}
