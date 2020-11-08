use super::{ValidationError, Validator};
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
            let err = enum_validator.validate_text(item)
                .expect_err(&format!("text should not match (case insensitive): {}", item));
            match err {
                EnumError::InvalidOption(_) => {}, // InvalidOption should be returned
                err => panic!("Invalid error: {}", err),
            }
        }
        for item in NO_MATCH_CASE_INSENSITIVE.iter() {
            let err = enum_validator.validate_text(item)
                .expect_err(&format!("text should not match (regardless of case): {}", item));
            match err {
                EnumError::InvalidOption(_) => {}, // InvalidOption should be returned
                err => panic!("Invalid error: {}", err),
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
            let err = enum_validator.validate_text(item)
                .expect_err(&format!("text should not match (regardless of case): {}", item));
            match err {
                EnumError::InvalidOption(_) => {}, // InvalidOption should be returned
                err => panic!("Invalid error: {}", err),
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum EnumError {
    InvalidOption(String),
    Validation(ValidationError),
}

impl From<ValidationError> for EnumError {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl fmt::Display for EnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOption(allowed) =>
                write!(f, r#"string not found among valid values: {}"#, allowed),
            Self::Validation(err) =>
                write!(f, "{}", err),
        }
    }
}

impl Error for EnumError {}

pub(crate) struct EnumValidator {
    pub case_sensitive: bool,
    pub valid_values: Vec<String>,
}

impl Validator for EnumValidator {
    type Error = EnumError;
    fn validate_text(&self, text: &str) -> Result<(), Self::Error> {
        if self.case_sensitive && self.valid_values.iter().any(|s| s == text) {
            Ok(())
        } else if !self.case_sensitive && self.valid_values.iter().any(|s| s.eq_ignore_ascii_case(text)) {
            Ok(())
        } else {
            Err(EnumError::InvalidOption(format!("{:?}", self.valid_values)))
        }
    }
}