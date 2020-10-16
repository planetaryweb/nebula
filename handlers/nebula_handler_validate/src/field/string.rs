use super::{Validator, ValidationError};
use regex::Regex;
use serde::{Serialize, Deserialize};
use serde::de::{self, Deserializer, Visitor};
use serde::ser::Serializer;
use std::convert::From;
use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests {
	use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn string_validator_can_de_serialize() {
        let validator = StringValidator {
            min_len: Some(5),
            max_len: Some(8),
            regex: Some(Regex::new("^foo$").unwrap()),
        };

        // serde_test::Token does not have a variant for usize
        // JSON, at least, uses u64
        assert_tokens(&validator, &[
            Token::Struct { name: "StringValidator", len: 3 },
            Token::Str("min"),
            Token::Some,
            Token::U64(5),
            Token::Str("max"),
            Token::Some,
            Token::U64(8),
            Token::Str("regex"),
            Token::Some,
            Token::Str("^foo$"),
            Token::StructEnd,
        ]);

        let validator = StringValidator {
            min_len: None,
            max_len: None,
            regex: None,
        };

        // serde_test::Token does not have a variant for usize
        // JSON, at least, uses u64
        assert_tokens(&validator, &[
            Token::Struct { name: "StringValidator", len: 3 },
            Token::Str("min"),
            Token::None,
            Token::Str("max"),
            Token::None,
            Token::Str("regex"),
            Token::None,
            Token::StructEnd,
        ]);
    }

    #[test]
    fn string_validator_enforces_minimum_length() {
        let text = "this is some text";
        let mut validator = StringValidator {
            min_len: Some(text.len() + 1),
            max_len: None,
            regex: None,
        };

        let err = validator.validate_text(text)
            .expect_err("validating text with less than minimum length should fail");

        match err {
            StringError::TooShort(min) => assert_eq!(min, validator.min_len.unwrap()),
            err => panic!("expected StringError::TooShort, got {:?}", err),
        }

        validator.min_len = Some(text.len());

        validator.validate_text(text)
            .expect("text length == min should validate");
    }

    #[test]
    fn string_validator_enforces_maximum_length() {
        let text = "this is some text";
        let mut validator = StringValidator {
            min_len: None,
            max_len: Some(text.len() - 1),
            regex: None,
        };

        let err = validator.validate_text(text)
            .expect_err("validating text with more than maximum length should fail");

        match err {
            StringError::TooLong(max) => assert_eq!(max, validator.max_len.unwrap()),
            err => panic!("expected StringError::TooLong, got {:?}", err),
        }

        validator.max_len = Some(text.len());

        validator.validate_text(text)
            .expect("text length == max should validate");
    }

    #[test]
    fn string_validator_enforces_regex() {
        let valid = "foobar bar baz foo baz barfoobaz";
        let invalid = "foobar baz fail foo";
        let validator = StringValidator {
            min_len: None,
            max_len: None,
            regex: Some(Regex::new("^(foo|bar|baz|\\s)+$").unwrap()),
        };

        validator.validate_text(valid)
            .expect("valid text should validate");

        let err = validator.validate_text(invalid)
            .expect_err("invalid text should not validate");

        match err {
            StringError::Invalid => {},
            err => panic!("expected StringError::Invalid, got {:?}", err),
        }
    }
}

struct RegexVisitor;

impl<'de> Visitor<'de> for RegexVisitor {
    type Value = Option<Regex>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a valid regular expression")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E> where E: de::Error {
        Some(Regex::new(s).map_err(|e| de::Error::custom(e.to_string()))).transpose()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_string(Self)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> where E: de::Error {
        Ok(None)
    }
}

fn serialize_regex<S>(regex: &Option<Regex>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    match regex {
        None => serializer.serialize_none(),
        Some(regex) => serializer.serialize_some(&regex.to_string()),
    }
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Option<Regex>, D::Error> where D: Deserializer<'de> {
    deserializer.deserialize_option(RegexVisitor)
}

#[derive(Debug)]
pub(crate) enum StringError {
    TooShort(usize),
    TooLong(usize),
    Invalid,
    Validation(ValidationError),
}

impl From<ValidationError> for StringError {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort(min) => write!(f, "value must be at least {} characters long", min),
            Self::TooLong(max) => write!(f, "value must be no more than {} characters long", max),
            Self::Invalid => write!(f, "value is invalid"),
            Self::Validation(err) => write!(f, "{}", err),
        }
    }
}

impl Error for StringError {}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct StringValidator {
    #[serde(rename = "min")]
    pub min_len: Option<usize>,
    #[serde(rename = "max")]
    pub max_len: Option<usize>,
    #[serde(serialize_with = "serialize_regex", deserialize_with = "deserialize_regex")]
    pub regex: Option<Regex>,
}

#[cfg(test)]
impl std::cmp::PartialEq for StringValidator {
    fn eq(&self, other: &Self) -> bool {
        self.min_len == other.min_len &&
            self.max_len == other.max_len &&
            match &self.regex {
                None => other.regex.is_none(),
                Some(lregex) => match &other.regex {
                    None => false,
                    Some(rregex) => lregex.to_string() == rregex.to_string(),
                }
            }
    }
}

impl Validator for StringValidator {
    type Error = StringError;

    fn validate_text(&self, text: &str) -> Result<(), Self::Error> {
        if let Some(min) = self.min_len {
            if text.len() < min {
                return Err(Self::Error::TooShort(min));
            }
        }

        if let Some(max) = self.max_len {
            if text.len() > max {
                return Err(Self::Error::TooLong(max));
            }
        }

        if let Some(rgx) = &self.regex {
            if !rgx.is_match(text) {
                return Err(Self::Error::Invalid);
            }
        }

        Ok(())
    }
}
