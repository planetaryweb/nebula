use super::{Validator, ValidationError};
use nebula_rpc::config::{Config, ConfigError};
use lazy_static::lazy_static;
use regex::Regex;
use std::convert::{From, TryFrom};
use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref VALID_PHONE_NUMBERS: Vec<&'static str> = vec![
            // United States
            "+19563638399",
            "+19784207057",
            "+12025550111",
            // United Kingdom
            "+441632960876",
            "+441632960786",
            "+442079460936",
            // Japan
            "+81752299084",
        ];
        static ref INVALID_PHONE_NUMBERS_NO_PREFIX: Vec<&'static str> = vec![
            // United States
            "9563638399",
            "9784207057",
            "2025550111",
            // United Kingdom
            "1632960876",
            "1632960786",
            "2079460936",
            // Japan
            "752299084",
        ];
        static ref INVALID_PHONE_NUMBERS_HAS_PUNC: Vec<&'static str> = vec![
            // United States
            "+1 956-363-8399",
            "+1 (978) 420-7057",
            "+1 202-5550111",
            // United Kingdom
            "+44 1632-960876",
            "+44 1632 960786",
            "+44 20794 60936",
            "+44 020 7946 0499",
            // Japan
            // "75 060 2905",
            // "75 229-9084",
            "+81 75-229-9084",
        ];
        static ref INVALID_PHONE_NUMBERS_HAS_ALPHA: Vec<&'static str> = vec![
            // United States
            "+1956ISALPHA",
            "+1978420WORD",
            // United Kingdom
            "+4416329ALPHA",
            // Japan
            "+8175229WORD",
        ];
    }

    #[test]
    fn test_international_regex() {
        for number in VALID_PHONE_NUMBERS.iter() {
            assert!(GENERIC_PHONE_REGEX.is_match(number), "{} does not match", number);
        }
        for list in vec![ &*INVALID_PHONE_NUMBERS_HAS_ALPHA, &*INVALID_PHONE_NUMBERS_HAS_PUNC, &*INVALID_PHONE_NUMBERS_NO_PREFIX ].iter() {
            for number in list.iter() {
                assert!(!GENERIC_PHONE_REGEX.is_match(number), "{} should not match", number);
            }
        }
    }

    #[test]
    fn test_prefix_regex() {
        for list in vec![ &*VALID_PHONE_NUMBERS, &*INVALID_PHONE_NUMBERS_HAS_ALPHA, &*INVALID_PHONE_NUMBERS_HAS_PUNC ].iter() {
            for number in list.iter() {
                assert!(INTL_PREFIX_REGEX.is_match(number), "{} does not match", number);
            }
        }
        for number in INVALID_PHONE_NUMBERS_NO_PREFIX.iter() {
            assert!(!INTL_PREFIX_REGEX.is_match(number), "{} should not match", number);
        }
    }

    #[test]
    fn valid_phone_number_validates() {
        let validator = PhoneValidator{};
        for number in VALID_PHONE_NUMBERS.iter() {
            validator.validate_text(number)
                .expect("valid phone number should validate");
        }
    }

    #[test]
    fn phone_number_with_alpha_is_invalid() {
        let validator = PhoneValidator{};
        for number in INVALID_PHONE_NUMBERS_HAS_ALPHA.iter() {
            let err = validator.do_validate(number)
                .expect_err("phone number with alpha characters should not validate");
            match err {
                PhoneError::Invalid(_) => {},
                err => panic!("expected PhoneError::Invalid, got {:?}", err),
            }
        }
    }

    #[test]
    fn phone_number_without_prefix_is_invalid() {
        let validator = PhoneValidator{};
        for number in INVALID_PHONE_NUMBERS_NO_PREFIX.iter() {
            let err = validator.do_validate(number)
                .expect_err("phone number without international prefix should not validate");
            match err {
                PhoneError::NoPrefix(_) => {},
                err => panic!("expected PhoneError::NoPrefix, got {:?}", err),
            }
        }
    }

    #[test]
    fn phone_number_with_spaces_or_punc_is_invalid() {
        let validator = PhoneValidator{};
        for number in INVALID_PHONE_NUMBERS_HAS_PUNC.iter() {
            let err = validator.do_validate(number)
                .expect_err("phone number with spaces or punctuation should not validate");
            match err {
                PhoneError::Invalid(_) => {},
                err => panic!("expected PhoneError::Invalid, got {:?}", err),
            }
        }
    }
}

lazy_static! {
    /// Phone regular expression for "Generic International Phone Number" from <http://www.phoneregex.com/>.
    /// Requires all phone numbers to include the international prefix and not contain any spaces,
    /// dashes, parentheses, or anything other than a leading plus and digits.
    static ref GENERIC_PHONE_REGEX: Regex = Regex::new(r#"^\+(9[976]\d|8[987530]\d|6[987]\d|5[90]\d|42\d|3[875]\d|2[98654321]\d|9[8543210]|8[6421]|6[6543210]|5[87654321]|4[987654310]|3[9643210]|2[70]|7|1)\d{1,14}$"#).unwrap();
    /// Regular expression that matches just the international prefix of a phone number. Used
    /// internally to determine if a number did not match because it doesn't have a valid prefix.
    static ref INTL_PREFIX_REGEX: Regex = Regex::new(r#"^\+(9[976]\d|8[987530]\d|6[987]\d|5[90]\d|42\d|3[875]\d|2[98654321]\d|9[8543210]|8[6421]|6[6543210]|5[87654321]|4[987654310]|3[9643210]|2[70]|7|1)"#).unwrap();
}

#[derive(Debug)]
pub(crate) enum PhoneError {
    Invalid(String),
    NoPrefix(String),
}

impl From<PhoneError> for ValidationError {
    fn from(err: PhoneError) -> Self {
        Self::InvalidInput(err.to_string())
    }
}

impl fmt::Display for PhoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(num) => write!(f, "{} appears to be an invalid phone number", num),
            Self::NoPrefix(num) => write!(f, "{} does not appear to have the required international prefix", num),
        }
    }
}

impl Error for PhoneError {}

pub struct PhoneValidator {}

impl PhoneValidator {
    fn do_validate(&self, text: &str) -> Result<(), PhoneError> {
        if !GENERIC_PHONE_REGEX.is_match(text) {
            if INTL_PREFIX_REGEX.is_match(text) {
                return Err(PhoneError::Invalid(text.to_string()))
            } else {
                return Err(PhoneError::NoPrefix(text.to_string()))
            }
        }

        Ok(())
    }
}

impl TryFrom<Config> for PhoneValidator {
    type Error = ConfigError;
    fn try_from(_: Config) -> Result<Self, ConfigError> {
        Ok(Self{})
    }
}

impl Validator for PhoneValidator {
    fn validate_text(&self, text: &str) -> crate::Result {
        self.do_validate(text).map_err(Into::into)
    }

    fn try_from_config(config: Config) -> Result<Self, ConfigError> where Self: Sized {
        Self::try_from(config)
    }
}
