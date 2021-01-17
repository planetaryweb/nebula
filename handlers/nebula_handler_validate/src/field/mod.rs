pub mod email;
pub mod enums;
pub mod file;
pub mod number;
pub mod phone;
pub mod string;
pub mod url;

use nebula_rpc::config::ConfigError;
use ordered_float::NotNan;
use crate::{Validator, ValidationError};

use email::EmailValidator;
use enums::EnumValidator;
use file::FileValidator;
use number::NumberValidator;
use phone::PhoneValidator;
use string::StringValidator;
use self::url::UrlValidator;

pub enum Type {
    /// The HTML5 color input type only allows lowercase hexadecimal values without
    /// alpha.
    //Color,
    Int(NumberValidator<i64>),
    Float(NumberValidator<NotNan<f64>>),
    Enum(EnumValidator),
    String(StringValidator),
    File(FileValidator),
    Email(EmailValidator),
    //Date,
    //DateTime,
    //Month,
    /// Generally corresponds to the HTML `password` input type.
    Hidden(StringValidator),
    Telephone(PhoneValidator),
    //Time,
    Url(UrlValidator),
    //Week,
    //List(Box<Type>),
}

impl<'a> From<&'a Type> for &'a dyn Validator {
    fn from(other: &'a Type) -> Self {
        match other {
            Type::Int(int_validator) => int_validator,
            Type::Float(float_validator) => float_validator,
            Type::Enum(enum_validator) => enum_validator,
            Type::String(str_validator) => str_validator,
            Type::Email(email_validator) => email_validator,
            Type::Hidden(hidden_validator) => hidden_validator,
            Type::Telephone(phone_validator) => phone_validator,
            Type::Url(url_validator) => url_validator,
            Type::File(file_validator) => file_validator,
        }
    }
}

impl Validator for Type {
    fn validate_text(&self, text: &str) -> crate::Result {
        <&dyn Validator>::from(self).validate_text(text)
    }

    fn validate_file(&self, file: &nebula_form::FormFile) -> crate::Result {
        <&dyn Validator>::from(self).validate_file(file)
    }

    fn try_from_config(config: nebula_rpc::Config) -> Result<Self, ConfigError> where Self: Sized {
        todo!()
    }
}

pub struct FieldValidator {
    pub required: bool,
    pub typ: Option<Type>,
}

impl FieldValidator {
}

impl Validator for FieldValidator {

    fn validate_text(&self, text: &str) -> Result<(), ValidationError> {
        if self.required && text.len() == 0 {
            return Err(ValidationError::FieldRequired);
        }

        if let Some(typ) = &self.typ {
            typ.validate_text(text)
        } else {
            Ok(())
        }
    }

    fn validate_file(&self, file: &nebula_form::FormFile) -> Result<(), ValidationError> {
        if let Some(typ) = &self.typ {
            typ.validate_file(file)
        } else {
            Ok(())
        }
    }

    fn try_from_config(config: nebula_rpc::Config) -> Result<Self, ConfigError> where Self: Sized {
        todo!()
    }
}
