#![warn(clippy::all)]
#![warn(clippy::correctness)]
#![warn(clippy::style)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::pedantic)]
#![allow(dead_code)]
#![allow(unused_imports)]

use ::async_trait::async_trait;
use bytes::Bytes;
use nebula_form::Form;
use ::nebula_rpc::server::Handler as RPCHandler;
use nebula_rpc::config::{Config, ConfigError};
use ::nebula_form::{Field, FormFile as File};
use nebula_status::Status;

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use ::std::result::Result as StdResult;


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn join_iter_works() {
        let mut set = HashSet::new();
        set.insert("foo".to_string());
        set.insert("bar".to_string());
        set.insert("baz".to_string());
        set.insert("quux".to_string());
        // HashSet iterator is arbitrary order, so the best way to tell
        // if the string is correct is to test the length
        assert_eq!(join_iter(&mut set.iter(), ", ").len(), "foo, bar, baz, quux".len());
    }
}

fn join_iter<T>(collection: &mut dyn Iterator<Item=&T>, sep: &str) -> String where T: fmt::Display + std::cmp::Eq {
    let mut s = collection.fold(String::new(), |mut acc, elem| {
        acc.push_str(&format!("{}", elem));
        acc.push_str(sep);
        acc
    });
    // Remove the last instance of the separator
    s.truncate(s.len() - sep.len());
    s
}

#[derive(Debug)]
pub enum ValidationError {
    FieldRequired,
    InvalidInput(String),
    NotImplementedText,
    NotImplementedFile,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldRequired => write!(f, "this field is required"),
            Self::NotImplementedFile => write!(f, "this validator cannot handle files"),
            Self::NotImplementedText => write!(f, "this validator only handles files"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
        }
    }
}

impl Error for ValidationError {}

type Result = StdResult<(), ValidationError>;

pub trait Validator: Send + Sync {
    /// Using this instead of requiring TryFrom to make the trait object-safe
    fn try_from_config(config: Config) -> StdResult<Self, ConfigError> where Self: Sized;

    /// Validate text from a textual form field.
    fn validate_text(&self, _text: &str) -> Result {
        Err(ValidationError::NotImplementedText)
    }

    /// Validate a file submitted from a form.
    fn validate_file(&self, _file: &File) -> Result {
        Err(ValidationError::NotImplementedFile)
    }

    /// Validate any given field. Defaults to calling the appropriate `validate_*` method based on
    /// the field type.
    fn validate(&self, field: &Field) -> Result {
        match field {
            Field::Text(text) => self.validate_text(text),
            Field::File(file) => self.validate_file(file),
        }
    }
}

mod field;

pub struct Handler {
    fields: BTreeMap<String, Box<dyn Validator>>,
}

/*
#[async_trait]
impl RPCHandler for Handler {
    async fn handle(&self, config: Config, form: Form) -> Status<Bytes> {

    }

    async fn validate(&self, config: Config) -> Status<Bytes> {

    }
}
*/