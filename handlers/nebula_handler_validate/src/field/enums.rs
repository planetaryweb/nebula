use super::{ValidationError, Validator};
use nebula_rpc::config::{Config, ConfigError, ConfigExt};
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref NO_MATCH_CASE_INSENSITIVE: Vec<&'static str> = vec![
            "notfound",
            "item2",
            "item", // prefix
            "tem1", // suffix
            "rustacean",
        ];

        // Also not case sensitive
        static ref MATCHES_CASE_INSENSITIVE: Vec<&'static str> = vec![
            "ITEM1",
            "FOObar",
            "EnuMS",
        ];
        
        static ref MATCHES_CASE_SENSITIVE: Vec<&'static str> = vec![
            "item1",
            "fooBAR",
            "ENUMS",
        ];
    }

    fn get_enum_validator(case_sensitive: bool) -> EnumValidator {
        EnumValidator {
            case_sensitive,
            valid_values: vec![
                "item1".to_string(),
                "fooBAR".to_string(),
                "ENUMS".to_string(),
            ],
        }
    }

    #[test]
    fn field_in_enum_validator_case_sensitive() {
        let enum_validator = get_enum_validator(true);
        for item in MATCHES_CASE_SENSITIVE.iter() {
            enum_validator.validate_text(item)
                .expect(&format!("text should match (case sensitive): {}", item));
        }
        for item in MATCHES_CASE_INSENSITIVE.iter() {
            let err = enum_validator.do_validate(item)
                .expect_err(&format!("text should not match (case insensitive): {}", item));
            match err {
                EnumError::InvalidOption{..} => {}, // InvalidOption should be returned
                //err => panic!("Invalid error: {}", err),
            }
        }
        for item in NO_MATCH_CASE_INSENSITIVE.iter() {
            let err = enum_validator.do_validate(item)
                .expect_err(&format!("text should not match (regardless of case): {}", item));
            match err {
                EnumError::InvalidOption{..} => {}, // InvalidOption should be returned
                //err => panic!("Invalid error: {}", err),
            }
        }
    }

    #[test]
    fn field_in_enum_validator_case_insensitive() {
        let enum_validator = get_enum_validator(false);
        for item in MATCHES_CASE_SENSITIVE.iter() {
            enum_validator.validate_text(item)
                .expect(&format!("text should match (case sensitive): {}", item));
        }
        for item in MATCHES_CASE_INSENSITIVE.iter() {
            enum_validator.validate_text(item)
                .expect(&format!("text should match (case insensitive): {}", item));
        }
        for item in NO_MATCH_CASE_INSENSITIVE.iter() {
            let err = enum_validator.do_validate(item)
                .expect_err(&format!("text should not match (regardless of case): {}", item));
            match err {
                EnumError::InvalidOption{..} => {}, // InvalidOption should be returned
                //err => panic!("Invalid error: {}", err),
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum EnumError {
    InvalidOption{ allowed: String },
}

impl From<EnumError> for ValidationError {
    fn from(err: EnumError) -> Self {
        ValidationError::InvalidInput(err.to_string())
    }
}

impl fmt::Display for EnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOption{ allowed } =>
                write!(f, r#"string not found among valid values: {}"#, allowed),
        }
    }
}

impl Error for EnumError {}

pub struct EnumValidator {
    pub case_sensitive: bool,
    pub valid_values: Vec<String>,
}

impl EnumValidator {
    const FIELD_CASE_SENSITIVE: &'static str = "case-sensitive";
    const FIELD_VALID_VALUES: &'static str = "valid-values";

    fn do_validate(&self, text: &str) -> Result<(), EnumError> {
        if self.case_sensitive && self.valid_values.iter().any(|s| s == text) {
            Ok(())
        } else if !self.case_sensitive && self.valid_values.iter().any(|s| s.eq_ignore_ascii_case(text)) {
            Ok(())
        } else {
            Err(EnumError::InvalidOption{ allowed: format!("{:?}", self.valid_values)})
        }
    }
}

impl TryFrom<Config> for EnumValidator {
    type Error = ConfigError;

    fn try_from(other: Config) -> Result<Self, ConfigError> {
        let case_sensitive: bool = other.get_path_single(Self::FIELD_CASE_SENSITIVE)?
            .unwrap_or(true); // Default to case sensitive

        let valid_values: Vec<String> = other.get_path_list(Self::FIELD_VALID_VALUES)?
            .ok_or(ConfigError::Missing(Self::FIELD_VALID_VALUES.to_owned()))?;
        let valid_values = if case_sensitive {
            valid_values
        } else {
            valid_values.into_iter().map(|s| s.to_lowercase()).collect()
        };

        Ok(Self { case_sensitive, valid_values })
    }
}

impl Validator for EnumValidator {
    fn validate_text(&self, text: &str) -> crate::Result {
        self.do_validate(text).map_err(Into::into)
    }

    fn try_from_config(config: Config) -> Result<Self, ConfigError> where Self: Sized {
        Self::try_from(config)
    }
}
