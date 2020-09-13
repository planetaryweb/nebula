use crate::config::{Config, Value as ConfigValue};
use crate::rpc;
use bytes::Bytes;
use http::header::{HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue, ToStrError};
use http::status::InvalidStatusCode;
use nebula_form::{Field, Form, FormFile};
use nebula_status::{Status, StatusCode, StatusData};
use std::collections::HashMap;
use std::fmt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc;

    // These functions should be kept in sync to ensure that the normal version
    // and the RPC type represent the same information. As long as these stay
    // in sync, all tests and functions will be correct.

    fn get_form_file() -> FormFile {
        FormFile {
            filename: "some form file.txt".to_string(),
            content_type: "text/plain".to_string(),
            bytes: b"text content\nstuff".to_vec().into(),
        }
    }

    fn get_rpc_form_file() -> rpc::File {
        rpc::File {
            name: "some form file.txt".to_string(),
            content_type: "text/plain".to_string(),
            content: b"text content\nstuff".to_vec(),
        }
    }

    fn get_file_field() -> Field {
        Field::File(get_form_file())
    }

    fn get_rpc_file_field() -> rpc::Field {
        rpc::Field {
            value: Some(rpc::field::Value::File(get_rpc_form_file())),
        }
    }

    fn get_text_field() -> Field {
        Field::Text("username@example.com".to_string())
    }

    fn get_rpc_text_field() -> rpc::Field {
        rpc::Field {
            value: Some(rpc::field::Value::Text("username@example.com".to_string())),
        }
    }

    fn get_form() -> Form {
        let mut form = Form::new();
        form.insert("some_file", get_file_field());
        form.insert("email", get_text_field());
        form
    }

    fn get_rpc_form() -> rpc::Form {
        let mut fields = HashMap::new();
        fields.insert("some_file".to_string(), get_rpc_file_field());
        fields.insert("email".to_string(), get_rpc_text_field());
        rpc::Form { fields }
    }

    fn get_config() -> Config {
        let mut inner = Config::new();
        inner.insert("baz".to_string(), ConfigValue::Leaf("quux".to_string()));
        let mut config = Config::new();
        config.insert(
            "top-level".to_string(),
            ConfigValue::Leaf("foobar".to_string()),
        );
        config.insert("bar".to_string(), ConfigValue::Node(inner));
        config
    }

    fn get_rpc_config() -> rpc::Config {
        let mut inner = HashMap::new();
        inner.insert(
            "baz".to_string(),
            rpc::ConfigValue {
                value: Some(rpc::config_value::Value::Leaf("quux".to_string())),
            },
        );
        let inner = rpc::Config { config: inner };
        let mut config = HashMap::new();
        config.insert(
            "top-level".to_string(),
            rpc::ConfigValue {
                value: Some(rpc::config_value::Value::Leaf("foobar".to_string())),
            },
        );
        config.insert(
            "bar".to_string(),
            rpc::ConfigValue {
                value: Some(rpc::config_value::Value::Node(inner)),
            },
        );
        rpc::Config { config }
    }

    fn get_status() -> Status<Bytes> {
        let mut status = Status::with_data(
            StatusCode::IM_A_TEAPOT,
            b"{ success: true }".to_vec().into(),
        );
        status.headers_mut().insert(
            "X-App-Test",
            "lorem ipsum"
                .parse()
                .expect("should be a valid header value"),
        );
        status
    }

    fn get_rpc_status() -> rpc::Status {
        let mut headers = HashMap::new();
        // http::HeaderMap lowercases header names on insert, match that here
        headers.insert(
            "x-app-test".to_string(),
            rpc::Headers {
                headers: vec!["lorem ipsum".to_string()],
            },
        );
        rpc::Status {
            code: 418u32,
            headers,
            body: b"{ success: true }".to_vec(),
        }
    }

    #[test]
    fn form_file_from_rpc() {
        let rpc_file = get_rpc_form_file();
        let file = FormFile::from_rpc(rpc_file).expect("Conversion should not fail");
        let expected = get_form_file();

        assert_eq!(file, expected);
    }

    #[test]
    fn form_file_into_rpc() {
        let file = get_form_file();
        let rpc_file = file.into_rpc().expect("Conversion should not fail");
        let expected = get_rpc_form_file();

        assert_eq!(rpc_file, expected);
    }

    #[test]
    fn file_field_from_rpc() {
        let rpc_field = get_rpc_file_field();
        let field = Field::from_rpc(rpc_field).expect("conversion should not fail");
        let expected = get_file_field();
        assert_eq!(field, expected);
    }

    #[test]
    fn file_field_into_rpc() {
        let field = get_file_field();
        let rpc_field = field.into_rpc().expect("conversion should not fail");
        let expected = get_rpc_file_field();
        assert_eq!(rpc_field, expected);
    }

    #[test]
    fn text_field_from_rpc() {
        let rpc_field = get_rpc_text_field();
        let field = Field::from_rpc(rpc_field).expect("conversion should not fail");
        let expected = get_text_field();
        assert_eq!(field, expected);
    }

    #[test]
    fn text_field_into_rpc() {
        let field = get_text_field();
        let rpc_field = field.into_rpc().expect("conversion should not fail");
        let expected = get_rpc_text_field();
        assert_eq!(rpc_field, expected);
    }

    #[test]
    fn form_from_rpc() {
        let rpc_form = get_rpc_form();
        let form = Form::from_rpc(rpc_form).expect("conversion should not fail");
        let expected = get_form();
        assert_eq!(form, expected);
    }

    #[test]
    fn form_into_rpc() {
        let form = get_form();
        let rpc_form = form.into_rpc().expect("conversion should not fail");
        let expected = get_rpc_form();
        assert_eq!(rpc_form, expected);
    }

    #[test]
    fn config_from_rpc() {
        let rpc_config = get_rpc_config();
        let config = Config::from_rpc(rpc_config).expect("conversion should not fail");
        let expected = get_config();
        assert_eq!(config, expected);
    }

    #[test]
    fn config_into_rpc() {
        let config = get_config();
        let rpc_config = config.into_rpc().expect("conversion should not fail");
        let expected = get_rpc_config();
        assert_eq!(rpc_config, expected);
    }

    #[test]
    fn status_from_rpc() {
        let rpc_status = get_rpc_status();
        let status = Status::<Bytes>::from_rpc(rpc_status).expect("conversion should not fail");
        let expected = get_status();
        assert_eq!(status, expected);
    }

    #[test]
    fn status_into_rpc() {
        let status = get_status();
        let rpc_status = status.into_rpc().expect("conversion should not fail");
        let expected = get_rpc_status();
        assert_eq!(rpc_status, expected);
    }
}

#[derive(Debug)]
pub enum Error {
    HeaderNameToStr(ToStrError),
    HeaderNameFromStr(InvalidHeaderName),
    HeaderValueToStr(ToStrError),
    HeaderValueFromStr(InvalidHeaderValue),
    InvalidStatusCode(InvalidStatusCode),
    UnexpectedNone(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderNameToStr(err) => {
                writeln!(f, "Could not convert HeaderName into string: {}", err)
            }
            Self::HeaderNameFromStr(err) => {
                writeln!(f, "Could not convert string into HeaderName: {}", err)
            }
            Self::HeaderValueToStr(err) => {
                writeln!(f, "Could not convert HeaderValue into string: {}", err)
            }
            Self::HeaderValueFromStr(err) => {
                writeln!(f, "Could not convert string into HeaderValue: {}", err)
            }
            Self::InvalidStatusCode(err) => writeln!(f, "Invalid HTTP status code: {}", err),
            Self::UnexpectedNone(field) => writeln!(f, "Missing field: {}", field),
        }
    }
}

impl From<Error> for String {
    fn from(err: Error) -> String {
        err.to_string()
    }
}

impl From<InvalidStatusCode> for Error {
    fn from(err: InvalidStatusCode) -> Self {
        Error::InvalidStatusCode(err)
    }
}

impl From<InvalidHeaderName> for Error {
    fn from(err: InvalidHeaderName) -> Self {
        Error::HeaderNameFromStr(err)
    }
}

impl From<InvalidHeaderValue> for Error {
    fn from(err: InvalidHeaderValue) -> Self {
        Error::HeaderValueFromStr(err)
    }
}

pub trait IntoRPC {
    type RPCType: prost::Message;
    fn into_rpc(self) -> Result<Self::RPCType, Error>;
}

pub trait FromRPC: Sized {
    type RPCType: prost::Message;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error>;
}

impl FromRPC for FormFile {
    type RPCType = rpc::File;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error> {
        let file = FormFile {
            filename: other.name,
            content_type: other.content_type,
            bytes: other.content.into(),
        };
        Ok(file)
    }
}

impl IntoRPC for FormFile {
    type RPCType = rpc::File;
    fn into_rpc(self) -> Result<Self::RPCType, Error> {
        let file = Self::RPCType {
            name: self.filename,
            content_type: self.content_type,
            content: self.bytes.into_iter().collect(),
        };
        Ok(file)
    }
}

impl FromRPC for Field {
    type RPCType = rpc::Field;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error> {
        let field = match other
            .value
            .ok_or_else(|| Error::UnexpectedNone("Field.value"))?
        {
            rpc::field::Value::Text(text) => Field::Text(text),
            rpc::field::Value::File(file) => Field::File(FormFile::from_rpc(file)?),
        };

        Ok(field)
    }
}

impl IntoRPC for Field {
    type RPCType = rpc::Field;
    fn into_rpc(self) -> Result<Self::RPCType, Error> {
        let field = Self::RPCType {
            value: Some(match self {
                Field::File(file) => rpc::field::Value::File(file.into_rpc()?),
                Field::Text(text) => rpc::field::Value::Text(text),
            }),
        };
        Ok(field)
    }
}

impl FromRPC for Form {
    type RPCType = rpc::Form;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error> {
        let fields = other
            .fields
            .into_iter()
            .map(|(key, val)| Ok((key, Field::from_rpc(val)?)))
            .collect::<Result<HashMap<String, Field>, Error>>()?;
        let mut form = Form::new();
        form.extend(fields.into_iter());
        Ok(form)
    }
}

impl IntoRPC for Form {
    type RPCType = rpc::Form;
    fn into_rpc(self) -> Result<Self::RPCType, Error> {
        let fields = self
            .into_iter()
            .map(|(key, val)| Ok((key, val.into_rpc()?)))
            .collect::<Result<HashMap<String, rpc::Field>, Error>>()?;
        Ok(rpc::Form { fields })
    }
}

impl FromRPC for Status<Bytes> {
    type RPCType = rpc::Status;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error> {
        let mut status =
            Status::with_data(StatusCode::from_u16(other.code as u16)?, other.body.into());
        for (key, list) in other.headers.into_iter() {
            for val in list.headers.into_iter() {
                let key = key.as_str().parse::<HeaderName>()?;
                let val = val.as_str().parse::<HeaderValue>()?;

                status.headers_mut().insert(key, val);
            }
        }
        Ok(status)
    }
}

impl<T> IntoRPC for Status<T>
where
    T: StatusData,
{
    type RPCType = rpc::Status;
    fn into_rpc(self) -> Result<Self::RPCType, Error> {
        let headers = {
            let mut headers = HashMap::new();
            for (key, val) in self.headers().iter() {
                let h_list = {
                    if !headers.contains_key(key) {
                        headers.insert(key, Vec::new());
                    }
                    // Above code should ensure this is always Some()
                    headers.get_mut(key).unwrap()
                };

                let val = val.to_str().map_err(Error::HeaderValueToStr)?.to_string();
                h_list.push(val);
            }

            headers
                .into_iter()
                .map(|(key, val)| (key.to_string(), rpc::Headers { headers: val }))
                .collect()
        };

        let status = rpc::Status {
            code: self.code().as_u16() as u32,
            headers,
            body: self.bytes().to_vec(),
        };

        Ok(status)
    }
}

impl IntoRPC for ConfigValue {
    type RPCType = rpc::ConfigValue;
    fn into_rpc(self) -> Result<Self::RPCType, Error> {
        let result = match self {
            ConfigValue::Leaf(text) => {
                let value = rpc::config_value::Value::Leaf(text);
                rpc::ConfigValue { value: Some(value) }
            }
            ConfigValue::Node(conf) => {
                let value = rpc::config_value::Value::Node(conf.into_rpc()?);
                rpc::ConfigValue { value: Some(value) }
            }
        };

        Ok(result)
    }
}

impl FromRPC for ConfigValue {
    type RPCType = rpc::ConfigValue;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error> {
        use rpc::config_value::Value as RPCValue;
        let result = match other.value.ok_or_else(|| Error::UnexpectedNone("value"))? {
            RPCValue::Leaf(text) => ConfigValue::Leaf(text),
            RPCValue::Node(conf) => ConfigValue::Node(Config::from_rpc(conf)?),
        };

        Ok(result)
    }
}

impl IntoRPC for Config {
    type RPCType = rpc::Config;
    fn into_rpc(self) -> Result<Self::RPCType, Error> {
        let config = self
            .into_iter()
            .map(|(key, val)| Ok((key, val.into_rpc()?)))
            .collect::<Result<HashMap<_, _>, Error>>()?;
        Ok(Self::RPCType { config })
    }
}

impl FromRPC for Config {
    type RPCType = rpc::Config;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error> {
        let config = other
            .config
            .into_iter()
            .map(|(key, val)| Ok((key, ConfigValue::from_rpc(val)?)))
            .collect::<Result<HashMap<_, _>, Error>>()?;
        Ok(config)
    }
}

impl IntoRPC for (Config, Form) {
    type RPCType = rpc::HandleRequest;
    fn into_rpc(self) -> Result<Self::RPCType, Error> {
        let (config, form) = self;
        let req = rpc::HandleRequest {
            config: Some(config.into_rpc()?),
            form: Some(form.into_rpc()?),
        };
        Ok(req)
    }
}

impl FromRPC for (Config, Form) {
    type RPCType = rpc::HandleRequest;
    fn from_rpc(other: Self::RPCType) -> Result<Self, Error> {
        let config = other
            .config
            .map(Config::from_rpc)
            .ok_or_else(|| Error::UnexpectedNone("config"))??;
        let form = other
            .form
            .map(Form::from_rpc)
            .ok_or_else(|| Error::UnexpectedNone("form"))??;
        Ok((config, form))
    }
}
