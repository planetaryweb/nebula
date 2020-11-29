use super::{ConfigError, Validator, ValidationError};
use nebula_rpc::{Config, config::ConfigExt};
use serde::{Serialize, Deserialize};
use std::cmp::PartialEq;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::error::Error;
use std::str::FromStr;
use std::fmt;

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

    const EMAIL_IN_WHITELIST:  &'static str = "username@allowed.com";
    const EMAIL_IN_BLACKLIST:  &'static str = "username@disallowed.com";
    const EMAIL_IN_BOTH_LISTS: &'static str = "username@domain.com";
    const EMAIL_NOT_IN_LIST:   &'static str = "username@example.com";
    const EMAIL_VALID_DOMAIN_INVALID_USER: &'static str = "user@invalid@domain.com";

    fn get_email_validator() -> EmailValidator {
        let mut domain_whitelist = HashSet::new();
        domain_whitelist.insert("allowed.com".to_string());
        domain_whitelist.insert("domain.com".to_string());
        let mut domain_blacklist = HashSet::new();
        domain_blacklist.insert("disallowed.com".to_string());
        domain_blacklist.insert("domain.com".to_string());
        EmailValidator {
            domain_whitelist: Some(domain_whitelist),
            domain_blacklist: Some(domain_blacklist),
            regex_type: Default::default(),
        }
    }

    #[test]
    fn email_in_whitelist_validates() {
        let mut validator = get_email_validator();
        validator.domain_blacklist = None;
        validator.validate_text(EMAIL_IN_BOTH_LISTS)
            .expect("whitelisted email should validate");
        validator.validate_text(EMAIL_IN_WHITELIST)
            .expect("whitelisted email should validate");
    }

    #[test]
    fn email_not_in_whitelist_does_not_validate() {
        let mut validator = get_email_validator();
        validator.domain_blacklist = None;
        let err = validator.validate_text(EMAIL_NOT_IN_LIST)
            .expect_err("non-whitelisted email should not validate");
        match err {
            EmailError::DomainNotWhitelisted(_) => {},
            err => panic!("invalid error, expected DomainNotWhitelisted: {}", err),
        }
    }

    #[test]
    fn email_not_in_blacklist_validates() {
        let mut validator = get_email_validator();
        validator.domain_whitelist = None;
        validator.validate_text(EMAIL_NOT_IN_LIST)
            .expect("non-blacklisted email should validate");
    }

    #[test]
    fn email_in_blacklist_does_not_validate() {
        let mut validator = get_email_validator();
        validator.domain_whitelist = None;
        let err = validator.validate_text(EMAIL_IN_BLACKLIST)
            .expect_err("blacklisted email should not validate");
        match err {
            EmailError::DomainBlacklisted(_) => {},
            err => panic!("invalid error, expected DomainBlacklisted: {}", err),
        }
        let err = validator.validate_text(EMAIL_IN_BOTH_LISTS)
            .expect_err("blacklisted email should not validate");
        match err {
            EmailError::DomainBlacklisted(_) => {},
            err => panic!("invalid error, expected DomainBlacklisted: {}", err),
        }
    }

    #[test]
    fn email_whitelist_takes_precedence() {
        let mut validator = get_email_validator();
        validator.validate_text(EMAIL_IN_BOTH_LISTS)
            .expect("white-and-blacklisted email should validate");
        validator.validate_text(EMAIL_IN_WHITELIST)
            .expect("whitelisted email should validate");
        let err = validator.validate_text(EMAIL_NOT_IN_LIST)
            .expect_err("non-whitelisted email should not validate");
        match err {
            EmailError::DomainNotWhitelisted(_) => {},
            err => panic!("invalid error, expected DomainNotWhitelisted: {}", err),
        }
    }

    #[test]
    fn whitelisted_domain_invalid_username_does_not_validate() {
        let mut validator = get_email_validator();
        let err = validator.validate_text(EMAIL_VALID_DOMAIN_INVALID_USER)
            .expect_err("invalid username with valid domain should not validate");
        match err {
            EmailError::NotValidEmail(_, _) => {},
            err => panic!("invalid error, expected NotValidEmail: {}", err),
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
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EmailType {
    Html5,
    Rfc5322,
}

impl FromStr for EmailType {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html5" => Ok(EmailType::Html5),
            "rfc5322" => Ok(EmailType::Rfc5322),
            _ => Err(ConfigError::Parse(format!("email type must be one of 'html5' or 'rfc5322': got {}", s))),
        }
    }
}

impl Default for EmailType {
    fn default() -> Self {
        Self::Html5
    }
}

impl fmt::Display for EmailType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Html5 => write!(f, "values accepted by the HTML5 email field"),
            Self::Rfc5322 => write!(f, "email addresses defined by RFC 5322"),
        }
    }
}

#[derive(Debug)]
pub enum EmailError {
    DomainBlacklisted(String),
    DomainNotWhitelisted(String),
    NotValidEmail(EmailType, String),
    Validation(ValidationError),
}

impl From<ValidationError> for EmailError {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl fmt::Display for EmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DomainBlacklisted(domain) => write!(f, "{} has been blacklisted", domain),
            Self::DomainNotWhitelisted(domain) => write!(f, "{} is not whitelisted", domain),
            Self::NotValidEmail(typ, email) => write!(f, "{} does not match {}", email, typ),
            Self::Validation(err) => write!(f, "{}", err),
        }
    }
}

impl Error for EmailError {}

pub(crate) struct EmailValidator {
    pub domain_whitelist: Option<HashSet<String>>,
    pub domain_blacklist: Option<HashSet<String>>,
    pub regex_type: EmailType,
}

impl EmailValidator {
    const FIELD_DOMAIN_BLACKLIST: &'static str = "domain-blacklist";
    const FIELD_DOMAIN_WHITELIST: &'static str = "domain-whitelist";
    const FIELD_REGEX_TYPE: &'static str = "type";
}

impl TryFrom<Config> for EmailValidator {
    type Error = ConfigError;
    fn try_from(config: Config) -> Result<Self, ConfigError> {
        let domain_blacklist = config.get_path_list(Self::FIELD_DOMAIN_BLACKLIST)?;
        let domain_whitelist = config.get_path_list(Self::FIELD_DOMAIN_WHITELIST)?;
        let regex_type = config.get_path_single(Self::FIELD_REGEX_TYPE)?
            .ok_or(ConfigError::Missing(Self::FIELD_REGEX_TYPE.to_string()))?;

        let result = EmailValidator {
            domain_whitelist,
            domain_blacklist,
            regex_type,
        };

        Ok(result)
    }
}

impl Validator for EmailValidator {
    type Error = EmailError;
    fn validate_text(&self, text: &str) -> Result<(), EmailError> {
        let regex = match self.regex_type {
            EmailType::Html5 => &*regexes::EMAIL_HTML5,
            EmailType::Rfc5322 => &*regexes::EMAIL_RFC_5322,
        };

        if !regex.is_match(text) {
            return Err(EmailError::NotValidEmail(
                self.regex_type,
                text.to_string(),
            ));
        }

        // The regular expressions enforce at least one @ regardless of which one is used,
        // so there should always be at least one result.
        let domain = text.rsplit('@').next().unwrap();
        match &self.domain_whitelist {
            Some(wset) => {
                if !wset.iter().any(|s| s.eq_ignore_ascii_case(domain)) {
                    return Err(EmailError::DomainNotWhitelisted(
                        domain.to_string(),
                    ));
                }
            },
            None => match &self.domain_blacklist {
                Some(bset) => {
                    if bset.iter().any(|s| s.eq_ignore_ascii_case(domain)) {
                        return Err(EmailError::DomainBlacklisted(
                            domain.to_string(),
                        ));
                    }
                },
                None => {}
            }
        }

        Ok(())
    }
}
