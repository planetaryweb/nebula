pub mod email;
pub mod enums;
pub mod file;
pub mod number;
pub mod phone;
pub mod string;
pub mod url;

use nebula_form::{Field, FormFile as File};
use nebula_rpc::config::{Config, ConfigError};
use ordered_float::NotNan;
use serde::de::{self, MapAccess};
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;

use email::EmailValidator;
use enums::EnumValidator;
use file::FileValidator;
use number::NumberValidator;
use phone::PhoneValidator;
use string::StringValidator;
use self::url::UrlValidator;

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

pub(crate) mod regexes {
    use lazy_static::lazy_static;
    use regex::Regex;
    lazy_static! {
        /// According to the [Mozilla Developer Network](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/color),
        /// the HTML5 color input type always uses lowercase hexadecimal notation without alpha.
        pub(crate) static ref COLOR: Regex = Regex::new("^#[a-f0-9]{6}$").unwrap();
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
    NotImplementedText,
    NotImplementedFile,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotImplementedFile => write!(f, "this validator cannot handle files"),
            Self::NotImplementedText => write!(f, "this validator only handles files"),
        }
    }
}

impl Error for ValidationError {}

pub trait Validator: TryFrom<nebula_rpc::Config, Error=ConfigError> {
    type Error: ::std::error::Error + std::convert::From<ValidationError>;
    /// Validate text from a textual form field.
    fn validate_text(&self, text: &str) -> Result<(), <Self as Validator>::Error> {
        Err(<Self as Validator>::Error::from(ValidationError::NotImplementedText))
    }

    /// Validate a file submitted from a form.
    fn validate_file(&self, file: &File) -> Result<(), <Self as Validator>::Error> {
        Err(<Self as Validator>::Error::from(ValidationError::NotImplementedFile))
    }

    /// Validate any given field. Defaults to calling the appropriate `validate_*` method based on
    /// the field type.
    fn validate(&self, field: &Field) -> Result<(), <Self as Validator>::Error> {
        match field {
            Field::Text(text) => self.validate_text(text),
            Field::File(file) => self.validate_file(file),
        }
    }
}

pub(crate) enum Type {
    /// The HTML5 color input type only allows lowercase hexadecimal values without
    /// alpha.
    Color,
    Int(NumberValidator<i64>),
    Float(NumberValidator<NotNan<f64>>),
    Enum(EnumValidator),
    String(StringValidator),
    File(FileValidator),
    Email(EmailValidator),
    Date,
    DateTime,
    Month,
    /// Generally corresponds to the HTML `password` input type.
    Hidden(StringValidator),
    Telephone(PhoneValidator),
    Time,
    Url(UrlValidator),
    Week,
    List(Box<Type>),
}

pub(crate) struct FieldValidation {
    pub required: bool,
    pub typ: Option<Type>,
}
