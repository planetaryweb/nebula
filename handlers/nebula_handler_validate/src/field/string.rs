use super::{Validator, ValidationError};
use nebula_rpc::config::{Config, ConfigError, ConfigExt};
use regex::Regex;
use std::convert::{From, TryFrom};
use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests {
	use super::*;

    #[test]
    fn string_validator_enforces_minimum_length() {
        let text = "this is some text";
        let mut validator = StringValidator {
            min_len: Some(text.len() + 1),
            max_len: None,
            regex: None,
        };

        let err = validator.do_validate(text)
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

        let err = validator.do_validate(text)
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

        let err = validator.do_validate(invalid)
            .expect_err("invalid text should not validate");

        match err {
            StringError::Invalid => {},
            err => panic!("expected StringError::Invalid, got {:?}", err),
        }
    }
}

#[derive(Debug)]
pub(crate) enum StringError {
    TooShort(usize),
    TooLong(usize),
    Invalid,
}

impl From<StringError> for ValidationError {
    fn from(err: StringError) -> Self {
        Self::InvalidInput(err.to_string())
    }
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort(min) => write!(f, "value must be at least {} characters long", min),
            Self::TooLong(max) => write!(f, "value must be no more than {} characters long", max),
            Self::Invalid => write!(f, "value is invalid"),
        }
    }
}

impl Error for StringError {}

#[derive(Debug)]
pub struct StringValidator {
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    pub regex: Option<Regex>,
}

impl StringValidator {
    const FIELD_MIN_LENGTH: &'static str = "min";
    const FIELD_MAX_LENGTH: &'static str = "max";
    const FIELD_REGEX: &'static str = "regex";

    fn do_validate(&self, text: &str) -> Result<(), StringError> {
        if let Some(min) = self.min_len {
            if text.len() < min {
                return Err(StringError::TooShort(min));
            }
        }

        if let Some(max) = self.max_len {
            if text.len() > max {
                return Err(StringError::TooLong(max));
            }
        }

        if let Some(rgx) = &self.regex {
            if !rgx.is_match(text) {
                return Err(StringError::Invalid);
            }
        }

        Ok(())
    }
}

impl TryFrom<Config> for StringValidator {
    type Error = ConfigError;
    fn try_from(config: Config) -> Result<Self, ConfigError> {
        let min_len = config.get_path_single(Self::FIELD_MIN_LENGTH)?;
        let max_len = config.get_path_single(Self::FIELD_MAX_LENGTH)?;
        let regex = config.get_path_single::<String>(Self::FIELD_REGEX)?
            .map(|s| Regex::new(&s)).transpose()
            .map_err(|err| ConfigError::Parse(err.to_string()))?;
        Ok(Self { min_len, max_len, regex })
    }
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
    fn validate_text(&self, text: &str) -> crate::Result {
        self.do_validate(text).map_err(Into::into)
    }

    fn try_from_config(config: Config) -> Result<Self, ConfigError> where Self: Sized {
        Self::try_from(config)
    }
}
