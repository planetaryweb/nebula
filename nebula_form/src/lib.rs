use bytes::Bytes;
#[cfg(feature = "server-warp")]
use futures::stream::Stream;
#[cfg(feature = "server-warp")]
use futures::{StreamExt, TryStreamExt};
#[cfg(feature = "server-warp")]
use bytes::Buf;
#[cfg(feature = "server-warp")]
use nebula_status::{Status, StatusCode};
use std::collections::HashMap;
#[cfg(feature = "server-warp")]
use std::error::Error;
#[cfg(feature = "server-warp")]
use std::fmt::{self, Display, Formatter};
use std::str;
use urlencoding;
#[cfg(feature = "server-warp")]
use warp::filters::multipart::{FormData, Part};
#[cfg(feature = "server-warp")]
use warp::reject::{Reject, Rejection};
#[cfg(feature = "server-warp")]
use warp::Filter;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "server-warp")]
    use futures::executor::block_on;
    use std::collections::HashMap;

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
        expected.extend_from_slice(
            b"\r\nContent-Disposition: form-data; name=\"baz\"; filename=\"baz.txt\"",
        );
        expected.extend_from_slice(b"\r\nContent-type: text/plain");
        expected.extend_from_slice(b"\r\n\r\nBaz is a text file with this content.");
        expected.extend_from_slice(b"\r\n");

        let mut hash = HashMap::new();
        hash.insert(
            String::from("baz"),
            Field::File(FormFile {
                filename: String::from("baz.txt"),
                content_type: String::from("text/plain"),
                bytes: Bytes::from_static(b"Baz is a text file with this content."),
            }),
        );

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
    fn field_with_text_into_text() {
        let content = "some random text stuff";
        let field = Field::Text(String::from(content));
        assert_eq!(field.into_text(), Some(String::from(content)));
    }

    #[test]
    fn field_with_text_into_file_is_none() {
        let content = "some random text stuff";
        let field = Field::Text(String::from(content));
        assert_eq!(field.into_file(), None);
    }

    #[test]
    fn field_with_file_into_file() {
        let file = FormFile {
            filename: String::from("file.txt"),
            content_type: String::from("text/plain"),
            bytes: b"this is the content of the file."[..].into(),
        };

        let field = Field::File(file.clone());
        assert_eq!(field.into_file(), Some(file));
    }

    #[test]
    fn field_with_file_into_text_is_none() {
        let file = FormFile {
            filename: String::from("file.txt"),
            content_type: String::from("text/plain"),
            bytes: b"this is the content of the file."[..].into(),
        };

        let field = Field::File(file);
        assert_eq!(field.into_text(), None);
    }

    #[test]
    fn field_with_text_as_text() {
        let content = "some random text stuff";
        let field = Field::Text(String::from(content));
        assert_eq!(field.as_text(), Some(content));
    }

    #[test]
    fn field_with_text_as_file_is_none() {
        let content = "some random text stuff";
        let field = Field::Text(String::from(content));
        assert_eq!(field.as_file(), None);
    }

    #[test]
    fn field_with_file_as_file() {
        let file = FormFile {
            filename: String::from("file.txt"),
            content_type: String::from("text/plain"),
            bytes: b"this is the content of the file."[..].into(),
        };

        let field = Field::File(file.clone());
        assert_eq!(field.as_file(), Some(&file));
    }

    #[test]
    fn field_with_file_as_text_is_none() {
        let file = FormFile {
            filename: String::from("file.txt"),
            content_type: String::from("text/plain"),
            bytes: b"this is the content of the file."[..].into(),
        };

        let field = Field::File(file);
        assert_eq!(field.as_text(), None);
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

        assert!(result
            .as_slice()
            .windows(foo_bytes.len())
            .any(|win| win == foo_bytes.as_slice()));

        assert!(result
            .as_slice()
            .windows(bar_bytes.len())
            .any(|win| win == bar_bytes.as_slice()));

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

        assert!(result
            .as_slice()
            .windows(foo_bytes.len())
            .any(|win| win == foo_bytes.as_slice()));

        assert!(result
            .as_slice()
            .windows(baz_bytes.len())
            .any(|win| win == baz_bytes.as_slice()));

        assert_eq!(&result[(result.len() - end.len())..], end.as_slice());

        assert_eq!(result.len(), foo_bytes.len() + baz_bytes.len() + end.len());
    }

    #[cfg(feature = "server-warp")]
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

    #[cfg(feature = "server-warp")]
    fn mock_warp_request(boundary: &str, body: &[u8]) -> Form {
        let filter = warp::filters::multipart::form().map(|data| Form::try_from_formdata(data));

        let result = warp::test::request()
            .method("POST")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .header("Content-Length", format!("{}", body.len()))
            .body(body)
            .filter(&filter);

        let temp = block_on(result);
        block_on(temp.unwrap()).unwrap()
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn multipart_try_from_no_files() {
        let (boundary, form) = mock_form(false);
        let body = form.to_multipart_bytes(boundary.as_bytes());

        let result = mock_warp_request(&boundary, &body);

        assert_eq!(form, result);
    }

    #[test]
    #[cfg(feature = "server-warp")]
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
                Field::Text(val) => {
                    assert!(qstr.contains(&String::from(format!("{}={}", key, val))))
                }
                Field::File(_) => assert!(false),
            }
        }

        let fields_len: usize = fields
            .iter()
            .map(|(key, val)| {
                key.len()
                    + match val {
                        Field::Text(val) => val.len(),
                        Field::File(_) => panic!("there should not be a Form::Field here"),
                    }
            })
            .sum();

        // fields_len is the length of just the key and value strings
        // fields.len() is the number of key-value pairs (and thus "=")
        // fields.len() - 1 is the number of "&" between the pairs
        assert_eq!(qstr.len(), fields_len + fields.len() + fields.len() - 1)
    }

    #[test]
    #[cfg(feature = "server-warp")]
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
    #[cfg(feature = "server-warp")]
    fn wrap_form_multipart_no_file() {
        let (boundary, multipart) = mock_form(false);
        let filter = form_filter();
        let req = warp::test::request()
            .method("POST")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(multipart.to_multipart_bytes(boundary.as_bytes()))
            .filter(&filter);
        assert_eq!(block_on(req).unwrap(), multipart);
    }

    #[test]
    #[cfg(feature = "server-warp")]
    fn wrap_form_multipart_with_file() {
        let (boundary, multipart) = mock_form(true);
        let filter = form_filter();
        let req = warp::test::request()
            .method("POST")
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(multipart.to_multipart_bytes(boundary.as_bytes()))
            .filter(&filter);
        assert_eq!(block_on(req).unwrap(), multipart);
    }

    #[test]
    fn test_field_as_fromstr() {
        let field = Field::Text("12".to_string());
        let num = field.contents_as()
            .expect("Number conversion should not fail");

        assert_eq!(12u16, num);
    }

    #[test]
    fn test_file_field_is_not_text_with_fromstr() {
        let field = Field::File(
            FormFile {
                filename: "test.txt".to_string(),
                content_type: "text/plain".to_string(),
                bytes: b"12".as_ref().into(),
            }
        );

        let err = field.contents_as::<u16, _>()
            .expect_err("Converting text *file* to number should fail");
        
        if let Error::NotText = err {
            assert!(true);
        } else {
            panic!("Unexpected error: {:?}", err);
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ParseField(String),
    ParseForm(String),
    NotText,
    NotFile,
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
    #[cfg(feature = "server-warp")]
    /// A helper function that coalesces a `Buf` `Stream` into the bytes being
    /// streamed.
    ///
    /// Requires `features = "server-warp"`.
    async fn buf_to_bytes(
        strm: impl Stream<Item = Result<impl Buf, warp::Error>>,
    ) -> Result<Bytes, warp::Error> {
        Ok(Bytes::from(
            strm.try_fold(Vec::new(), |mut vec, data| {
                vec.extend_from_slice(data.bytes());
                async move { Ok(vec) }
            })
            .await?,
        ))
    }

    #[cfg(feature = "server-warp")]
    /// Attempts to create a `Field` instance from the provided `Part`.
    ///
    /// Requires `features = "server-warp"`.
    pub async fn try_from_async(part: Part) -> Result<(String, Self), Status<String>> {
        let name = part.name().to_string();
        let filename = part.filename().map(|f| f.to_string());
        let content_type = part.content_type().map(|c| c.to_string());

        let content = Self::buf_to_bytes(part.stream())
            .await
            .map_err(|e| Status::with_message(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let filename = match filename {
            None => {
                return String::from_utf8(content.to_vec())
                    .map(|s| (name, Field::Text(s)))
                    .map_err(|e| {
                        Status::with_message(StatusCode::UNSUPPORTED_MEDIA_TYPE, e.to_string())
                    })
            }
            Some(f) => f,
        };

        let content_type = content_type.ok_or(Status::with_message(
            StatusCode::BAD_REQUEST,
            "form field has filename but no content type".to_string(),
        ))?;

        let field = Field::File(FormFile {
            filename,
            content_type,
            bytes: content,
        });

        Ok((name, field))
    }

    /// Returns an Option containing the text of the field as an owned value,
    /// if it is not a File.
    pub fn into_text(self) -> Option<String> {
        match self {
            Field::Text(txt) => Some(txt),
            Field::File(_) => None,
        }
    }

    /// Returns an Option containing the file from the field as an owned value,
    /// if it is a File.
    pub fn into_file(self) -> Option<FormFile> {
        match self {
            Field::Text(_) => None,
            Field::File(f) => Some(f),
        }
    }

    /// Returns an Option containing the text of the field, if it is not a
    /// File.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Field::Text(txt) => Some(&txt),
            Field::File(_) => None,
        }
    }

    /// Returns an Option containing the file from the field, if it is a File.
    pub fn as_file(&self) -> Option<&FormFile> {
        match self {
            Field::Text(_) => None,
            Field::File(f) => Some(&f),
        }
    }

    /// Attmpts to return the field contents as an instance of type T if the field is
    /// Field::Text, or Ok(None) for Field::File.
    pub fn contents_as<T, E>(&self) -> Result<T, Error> where E: std::fmt::Display + Sized, T: FromStr<Err=E> {
        let txt = self.as_text()
            .ok_or(Error::NotText)?;
        txt.parse()
            .map_err(|e: E| Error::ParseField(e.to_string()))
    }
}

/// Represents the entire contents of a submitted form.
#[derive(Debug, Default, PartialEq)]
pub struct Form(HashMap<String, Field>);

impl IntoIterator for Form {
    type Item = (String, Field);
    type IntoIter = <HashMap<String, Field> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let Form(fields) = self;
        fields.into_iter()
    }
}

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
    pub fn extend(&mut self, iter: impl Iterator<Item = (String, Field)>) {
        for (name, field) in iter {
            self.insert(&name, field);
        }
    }

    /// Append the contents of a map to the current `Form`, converting the
    /// `String` values to a `Field::Text`. Fields that already exist will
    /// be overwritten.
    pub fn extend_from_strings(&mut self, iter: impl Iterator<Item = (String, String)>) {
        self.extend(iter.map(|(k, v)| (k, Field::Text(v))));
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
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Field)> {
        self.0.iter()
    }

    /// Returns an iterator over every text field in the form.
    pub fn iter_text(&self) -> impl Iterator<Item = (&String, &Field)> {
        self.iter().filter(|(_name, field)| match field {
            Field::Text(_) => true,
            Field::File(_) => false,
        })
    }

    /// Returns an iterator over every file field in the form.
    pub fn iter_files(&self) -> impl Iterator<Item = (&String, &Field)> {
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
                Field::File(_) => return Err(format!("Cannot include field {} as text", name)),
                Field::Text(txt) => {
                    let enc_key = urlencoding::encode(name);
                    let enc_val = urlencoding::encode(&txt);
                    builder.push(format!("{}={}", enc_key, enc_val));
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
                }
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

    #[cfg(feature = "server-warp")]
    /// Attempts to consume a Warp `FormData` stream and return a `Form` built
    /// from its contents.
    ///
    /// Requires `features = "server-warp"`.
    async fn try_from_formdata(mut data: FormData) -> Result<Self, Status<String>> {
        let mut form = Form::new();

        while let Some(part) = data.next().await {
            match part {
                Err(err) => {
                    return Err(Status::with_message(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        err.to_string(),
                    ))
                }
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
#[cfg(feature = "server-warp")]
///
/// Requires `features = "server-warp"`.
struct RejectionWrapper {
    msg: String,
}

#[cfg(feature = "server-warp")]
impl Display for RejectionWrapper {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[cfg(feature = "server-warp")]
impl Error for RejectionWrapper {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[cfg(feature = "server-warp")]
impl Reject for RejectionWrapper {}

#[cfg(feature = "server-warp")]
/// Returns a `Filter` that reads a form as either a URL-encoded request body
/// or a `multipart/form-data` body, parses it as necessary, and returns a
/// `Form` object.
///
/// Requires `features = "server-warp"`.
pub fn form_filter() -> impl Filter<Extract = (Form,), Error = Rejection> + Clone {
    warp::filters::body::form()
        .map(|f: HashMap<String, String>| Form::from(f))
        .or(
            warp::filters::multipart::form().and_then(|f: FormData| async move {
                Form::try_from_formdata(f)
                    .await
                    .map_err(|e| warp::reject::custom(e))
            }),
        )
        .unify()
}
