use super::{join_iter, Validator, ValidationError};
use nebula_rpc::config::{Config, ConfigError, ConfigExt};
use lazy_static::lazy_static;
use regex::Regex;
use std::cmp::Ord;
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    use ordered_float::NotNan;

    #[test]
    fn number_validator_all_types_compile() {
        let _ = NumberValidator::<i8> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<i16> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<i32> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<i64> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<u8> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<u16> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<u32> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<u64> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<NotNan<f32>> { min: None, max: None, valid_list: None };
        let _ = NumberValidator::<NotNan<f64>> { min: None, max: None, valid_list: None };
    }

    // BEGIN NUMBER (INT) VALIDATION TESTS

    const INT_MIN: i32 = -5;
    const INT_MAX: i32 = 25;

    fn get_int_validator() -> NumberValidator<i32> {
        NumberValidator {
            min: Some(INT_MIN),
            max: Some(INT_MAX),
            valid_list: None,
        }
    }

    #[test]
    fn int_non_numeric_string_does_not_validate() {
        let validator = get_int_validator();
        let err = validator.validate_text("three")
            .expect_err("number as word should not validate");
        match err {
            NumberError::<i32>::NotANumber(_) => {},
            err => panic!("invalid error, expected NotANumber: {}", err),
        }
        validator.validate_text("")
            .expect_err("empty string should not validate");
        match err {
            NumberError::<i32>::NotANumber(_) => {},
            err => panic!("invalid error, expected NotANumber: {}", err),
        }
        validator.validate_text("abc123")
            .expect_err("string starting with letters should not validate");
        match err {
            NumberError::<i32>::NotANumber(_) => {},
            err => panic!("invalid error, expected NotANumber: {}", err),
        }
        validator.validate_text("123abc")
            .expect_err("string ending with letters should not validate");
        match err {
            NumberError::<i32>::NotANumber(_) => {},
            err => panic!("invalid error, expected NotANumber: {}", err),
        }
        validator.validate_text("  123  ")
            .expect_err("string padded with spaces should not validate");
        match err {
            NumberError::<i32>::NotANumber(_) => {},
            err => panic!("invalid error, expected NotANumber: {}", err),
        }
    }

    #[test]
    fn int_too_large_negative_is_too_small() {
        let mut validator = get_int_validator();
        let err = validator.validate_text("-500")
            .expect_err("too negative of a number should not validate");
        match err {
            NumberError::<i32>::TooSmall(_) => {},
            err => panic!("invalid error, expected TooSmall: {}", err),
        }
        validator.min = Some(-501);
        validator.validate_text("-500")
            .expect("not too negative of a number should validate");
    }

    #[test]
    fn int_too_large_positive_is_too_big() {
        let mut validator = get_int_validator();
        let err = validator.validate_text("500")
            .expect_err("too positive of a number should not validate");
        match err {
            NumberError::<i32>::TooLarge(_) => {},
            err => panic!("invalid error, expected TooLarge: {}", err),
        }
        validator.max = Some(501);
        validator.validate_text("500")
            .expect("not too positive of a number should validate");
    }

    #[test]
    fn int_number_within_range_is_valid() {
        let validator = get_int_validator();
        validator.validate_text(&(INT_MAX + INT_MIN).to_string())
            .expect("number between max and min should validate");
    }

    #[test]
    fn int_number_within_range_not_in_set_is_invalid() {
        let mut validator = get_int_validator();
        let mut valid_list = BTreeSet::new();
        valid_list.insert(7);
        validator.valid_list = Some(valid_list);
        validator.min = Some(2);
        validator.max = Some(5);

        let err = validator.validate_text("4")
            .expect_err("number within range and not in valid list should not validate");
        match err {
            NumberError::<i32>::NotInSet(_) => {},
            err => panic!("invalid error, expected NotInSet: {}", err),
        }
    }

    #[test]
    fn int_number_not_within_range_but_in_set_is_valid() {
        let mut validator = get_int_validator();
        let mut valid_list = BTreeSet::new();
        valid_list.insert(7);
        validator.valid_list = Some(valid_list);
        validator.min = Some(2);
        validator.max = Some(5);

        validator.validate_text("7")
            .expect("number not within range but in valid list should validate");
    }
}

pub trait NumberType: FromStr + fmt::Debug + fmt::Display + Ord + Copy {}
impl<T> NumberType for T where T: FromStr + fmt::Debug + fmt::Display + Ord + Copy {}

pub trait ErrorTrait: fmt::Debug + fmt::Display {}
impl<T> ErrorTrait for T where T: fmt::Debug + fmt::Display {}

#[derive(Debug)]
pub(crate) enum NumberError<T> where T: NumberType {
    NotANumber(String),
    ParseFailure(String),
    TooSmall(T),
    TooLarge(T),
    NotInSet(String),
    Validation(ValidationError),
}

impl<T> From<ValidationError> for NumberError<T> where T: NumberType {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl<T> fmt::Display for NumberError<T> where T: NumberType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotANumber(val) => write!(f, "{} is not a number", val),
            Self::ParseFailure(err) => write!(f, "parsing number failed: {}", err),
            Self::TooSmall(min) => write!(f, "value is below minimum: {}", min),
            Self::TooLarge(max) => write!(f, "value is above maximum: {}", max),
            Self::NotInSet(set_list) => write!(f, "value is not among allowed values: {}", set_list),
            Self::Validation(err) => write!(f, "{}", err),
        }
    }
}

impl<T> Error for NumberError<T> where T: NumberType {}

lazy_static! {
    static ref NUMBER_REGEX: Regex = Regex::new(r#"\d+"#).unwrap();
}

pub(crate) struct NumberValidator<T> where T: NumberType {
    pub min: Option<T>,
    pub max: Option<T>,
    pub valid_list: Option<BTreeSet<T>>,
}

impl<T> NumberValidator<T> where T: NumberType {
    const FIELD_MIN: &'static str = "min";
    const FIELD_MAX: &'static str = "max";
    const FIELD_VALID_LIST: &'static str = "valid-list";
}

impl<T> TryFrom<Config> for NumberValidator<T> where T: NumberType, <T as FromStr>::Err: ErrorTrait {
    type Error = ConfigError;
    fn try_from(config: Config) -> Result<Self, ConfigError> {
        let min = config.get_path_single(Self::FIELD_MIN)?;
        let max = config.get_path_single(Self::FIELD_MAX)?;
        let valid_list = config.get_path_list(Self::FIELD_VALID_LIST)?;
        Ok(Self { min, max, valid_list })
    }
}

impl<T> Validator for NumberValidator<T> where T: NumberType, <T as FromStr>::Err: ErrorTrait {
    type Error = NumberError<T>;
    fn validate_text(&self, text: &str) -> Result<(), NumberError<T>> {
        if !NUMBER_REGEX.is_match(text) {
            return Err(NumberError::<T>::NotANumber(text.to_string()));
        }

        let num: T = text.parse().map_err(|err| NumberError::<T>::ParseFailure(format!("{:?}", err)))?;

        match &self.valid_list {
            Some(list) => {
                if !list.contains(&num) {
                    return Err(NumberError::<T>::NotInSet(join_iter(&mut list.iter(), ", ")));
                }
            },
            None => {
                if let Some(min) = &self.min {
                    if num < *min {
                        return Err(NumberError::<T>::TooSmall(*min));
                    }
                }

                if let Some(max) = &self.max {
                    if num > *max {
                        return Err(NumberError::<T>::TooLarge(*max));
                    }
                }
            }
        }

        Ok(())
    }
}
