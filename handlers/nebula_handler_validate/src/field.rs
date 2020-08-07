use nebula_form::{Form, Field, FormFile as File};
use regex::Regex;
use std::collections::HashSet;

#[cfg(test)]
mod tests {
	use super::*;
    use super::regexes::{EMAIL_HTML5, EMAIL_RFC_5322};
    use lazy_static::lazy_static;

    lazy_static! {
        // These example email addresses came from
        // <https://en.wikipedia.org/wiki/International_email> and serve as a decent set of odd but
        // technically valid email addresses.
        static ref ASCII_EMAILS: Vec<&'static str> = vec![
            "Abc@example.com",
            "Abc.123@example.com",
            "user+mailbox/department=shipping@example.com",
            "!#$%&'*+-/=?^_`.{|}~@example.com",
        ];

        static ref ASCII_QUOTED_EMAILS: Vec<&'static str> = vec![
            "\"Abc@def\"@example.com",
            "\"John\\\"Smith\"@example.com",
            "\"Fred Bloggs\"@example.com",
            "\"Joe.\\\\Blow\"@example.com",
        ];

        static ref UTF8_EMAILS: Vec<&'static str> = vec![
            "иван.сергеев@пример.рф",
            "用户@例子.广告",
            "अजय@डाटा.भारत",
            "квіточка@пошта.укр",
            "θσερ@εχαμπλε.ψομ",
            "Dörte@Sörensen.example.com",
            "коля@пример.рф",
        ];
    }

	#[test]
    fn html5_email_regex_works_on_ascii() {
        for email in ASCII_EMAILS.iter() {
            assert!(EMAIL_HTML5.is_match(*email), *email);
        }
    }

	#[test]
    fn html5_email_regex_does_not_work_on_quoted_ascii() {
        for email in ASCII_QUOTED_EMAILS.iter() {
            assert!(!EMAIL_HTML5.is_match(*email), *email);
        }
    }

	#[test]
    fn html5_email_regex_does_not_work_on_unicode() {
        for email in UTF8_EMAILS.iter() {
            assert!(!EMAIL_HTML5.is_match(*email), *email);
        }
    }

	#[test]
    fn rfc_5322_email_regex_works_on_ascii() {
        for email in ASCII_EMAILS.iter() {
            assert!(EMAIL_RFC_5322.is_match(*email), *email);
        }
    }

	#[test]
    fn rfc_5322_email_regex_works_on_quoted_ascii() {
        for email in ASCII_QUOTED_EMAILS.iter() {
            assert!(EMAIL_RFC_5322.is_match(*email), *email);
        }
    }

	#[test]
    fn rfc_5322_email_regex_works_on_unicode() {
        for email in UTF8_EMAILS.iter() {
            assert!(!EMAIL_RFC_5322.is_match(*email), *email);
        }
    }
}

pub(crate) mod regexes {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        /// The HTML5 spec regex for the `email` input type, as according to
        /// <http://emailregex.com/>.
        pub(crate) static ref EMAIL_HTML5: Regex = Regex::new(r#"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$"#).unwrap();
        /// The RFC 5322 spec regex for emails, as according to <http://emailregex.com>. Note that
        /// <https://www.regular-expressions.info/email.html> points out that not all email
        /// software can actually handle addresses that match this regex.
        pub(crate) static ref EMAIL_RFC_5322: Regex = Regex::new(r#"^(?:[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[ \x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?|\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-zA-Z0-9-]*[a-zA-Z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$"#).unwrap();
        /// According to the [Mozilla Developer Network](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/color),
        /// the HTML5 color input type always uses lowercase hexadecimal notation without alpha.
        pub(crate) static ref COLOR: Regex = Regex::new("^#[a-f0-9]{6}$").unwrap();
    }
}

pub enum ValidationError {
    FileTooBig(u64),
    InvalidContentType(String, String),
    InvalidValue(String, String),
    NotImplementedText,
    NotImplementedFile,
}

pub trait Validator {
    /// Validate text from a textual form field.
    fn validate_text(&self, text: &str) -> Result<(), ValidationError> {
        Err(ValidationError::NotImplementedText)
    }

    /// Validate a file submitted from a form.
    fn validate_file(&self, file: &File) -> Result<(), ValidationError> {
        Err(ValidationError::NotImplementedFile)
    }

    /// Validate any given field. Defaults to calling the appropriate `validate_*` method based on
    /// the field type.
    fn validate(&self, field: &Field) -> Result<(), ValidationError> {
        match field {
            Field::Text(text) => self.validate_text(text),
            Field::File(file) => self.validate_file(file),
        }
    }
}

pub(crate) struct EnumOptions {
    pub case_sensitive: bool,
    pub valid_values: Vec<String>,
}

impl Validator for EnumOptions {
    fn validate_text(&self, text: &str) -> Result<(), ValidationError> {
        if self.case_sensitive && self.valid_values.iter().any(|s| s == text) {
            Ok(())
        } else if !self.case_sensitive && self.valid_values.iter().any(|s| s.eq_ignore_ascii_case(text)) {
            Ok(())
        } else {
            Err(ValidationError::InvalidValue(text.to_string(), format!("{:?}", self.valid_values)))
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EmailType {
    HTML5,
    RFC_5322,
}

impl Default for EmailType {
    fn default() -> Self {
        Self::HTML5
    }
}

impl Display for EmailType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::HTML5 => write!(f, "values accepted by the HTML5 email field"),
            Self::RFC_5322 => write!(f, "any valid email address"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmailOptions {
    pub domain_whitelist: Option<HashSet<String>>,
    pub domain_blacklist: Option<HashSet<String>>,
    #[serde(default, alias = "type")]
    pub regex_type: EmailType,
}

impl Validator for EmailOptions {
    fn validate_text(&self, text: &str) -> Result<(), ValidationError> {
        let regex = match self.regex_type {
            EmailType::HTML5 => &regexes::EMAIL_HTML5,
            EmailType::RFC_5322 => &regexes::EMAIL_RFC_5322,
        };

        if !regex.is_match(text) {
            return Err(ValidationError::InvalidValue(
                    text.to_string(),
                    self.regex_type.to_string(),
            ))
        }

        // The regular expressions enforce at least one @ regardless of which one is used,
        // so there should always be at least one result.
        let domain = text.rsplit('@').first().unwrap();
        match self.domain_whitelist {
            Some(wset) => {
                if !wset.iter().any(|s| s.eq_ignore_ascii_case(domain)) {
                    return Err(ValidationError::InvalidValue(
                        text.to_string(),
                        format!("allowed domains: {}", wset.join(", ")),
                    ));
                }
            },
            None => match self.domain_blacklist {
                Some(bset) => {
                    if wset.iter().any(|s| s.eq_ignore_ascii_case(domain)) {
                        return Err(ValidationError::InvalidValue(
                            text.to_string(),
                            format!("banned domains: {}", wset.join(", ")),
                        ));
                    }
                },
                None => {}
            }
        }

        Ok(())
    }
}

pub(crate) struct FileOptions {
    pub content_types: Option<HashSet<String>>,
    pub max_size: Option<u64>, // Bytes
}

impl Validator for FileOptions {
    fn validate_file(&self, file: &File) -> Result<(), ValidationError> {
        match self.max_size {
            Some(size) => {
                if file.bytes.len() > size {
                    return Err(ValidationError::FileTooBig(self.max_size));
                }
            },
            None => {},
        }

        match self.content_types {
            Some(type_set) => {
                if !type_set.contains(file.content_type.to_lowercase()) {
                    return Err(ValidationError::InvalidContentType(file.content_type.to_lowercase(), type_set.join(", ")));
                }
            },
            None => {},
        }

        Ok(())
    }
}

pub(crate) struct NumberOptions<T> where T: PartialOrd {
    pub min: Option<T>,
    pub max: Option<T>,
    pub valid_list: Option<HashSet<T>>,
}

pub(crate) struct PhoneOptions {
    pub valid_area_codes: Option<Vec<String>>,
}

pub(crate) struct StringOptions {
    pub min_len: usize,
    pub max_len: usize,
    pub regex: Regex,
}

pub(crate) struct UrlOptions {
    pub valid_domains: Option<Vec<String>>,
}

pub(crate) enum Type {
    /// The HTML5 color input type only allows lowercase hexadecimal values without
    /// alpha.
    Color,
    Int(NumberOptions<i64>),
    Float(NumberOptions<f64>),
    Enum(EnumOptions),
    String(StringOptions),
    File(FileOptions),
    Email(EmailOptions),
    Date,
    DateTime,
    Month,
    /// Generally corresponds to the HTML `password` input type.
    Hidden(StringOptions),
    Telephone(PhoneOptions),
    Time,
    Url(UrlOptions),
    Week,
    List(Box<Type>),
}

struct FieldValidation {
    pub required: bool,
    pub typ: Option<Type>,
}
