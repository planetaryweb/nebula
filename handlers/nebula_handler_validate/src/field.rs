use bytes::Bytes;
use lazy_static::lazy_static;
use nebula_form::{Field, FormFile as File};
use ordered_float::NotNan;
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

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

    // BEGIN ENUM VALIDATION TESTING

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

    // END ENUM VALIDATION TESTING

    // BEGIN EMAIL VALIDATION TESTING

    const EMAIL_IN_WHITELIST:  &str = "username@allowed.com";
    const EMAIL_IN_BLACKLIST:  &str = "username@disallowed.com";
    const EMAIL_IN_BOTH_LISTS: &str = "username@domain.com";
    const EMAIL_NOT_IN_LIST:   &str = "username@example.com";
    const EMAIL_VALID_DOMAIN_INVALID_USER: &str = "user@invalid@domain.com";

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

    // END EMAIL VALIDATION TESTING

    // BEGIN FILE VALIDATION TESTING
    
    fn get_file_validator() -> FileValidator {
        let mut content_types = HashSet::new();
        content_types.insert("text/plain".to_string());
        content_types.insert("application/json".to_string());
        let content_types = Some(content_types);
        // Ensure that the valid file is always valid length *and*
        // cover the edge case of being *just* the max size.
        let max_size = Some(get_valid_file().bytes.len());
        FileValidator {
            content_types,
            max_size,
        }
    }

    fn get_valid_file() -> File {
        File {
            filename: "valid_file.txt".to_string(),
            content_type: "text/plain".to_string(),
            bytes: Bytes::from_static(b"Hello, world!"),
        }
    }

    fn get_invalid_file_wrong_content_type() -> File {
        File {
            filename: "short_enough_but_bad_content_type".to_string(),
            content_type: "application/rtf".to_string(),
            // Note: Keep this field longer than in `get_valid_file()`
            bytes: Bytes::from_static(b"5"),
        }
    }

    fn get_invalid_file_too_big() -> File {
        File {
            filename: "im_too_large.json".to_string(),
            content_type: "application/json".to_string(),
            bytes: Bytes::from_static(br#"{ "foo": "this string is too long to be valid." }"#),
        }
    }

    #[test]
    fn file_over_max_size_does_not_validate() {
        let mut validator = get_file_validator();
        let file = get_invalid_file_too_big();
        validator.content_types = None;
        let err = validator.validate_file(&file)
            .expect_err("file that is too big should not validate");
        match err {
            FileError::TooBig(_) => {},
            err => panic!("invalid error, expected TooBig: {}", err),
        }
    }

    #[test]
    fn file_not_in_content_types_does_not_validate() {
        let mut validator = get_file_validator();
        let file = get_invalid_file_wrong_content_type();
        validator.max_size = None;
        let err = validator.validate_file(&file)
            .expect_err("file that is too big should not validate");
        match err {
            FileError::InvalidContentType(_) => {},
            err => panic!("invalid error, expected InvalidContentType: {}", err),
        }
    }

    #[test]
    fn valid_file_validates() {
        let validator = get_file_validator();
        let file = get_valid_file();
        validator.validate_file(&file)
            .expect("valid file should validate");
    }

    // END FILE VALIDATION TESTS
    
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
        let mut validator = get_int_validator();
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

    // END NUMBER (INT) VALIDATION TESTS
    
    // BEGIN NUMBER (FLOAT) VALIDATION TESTS
    


    // END NUMBER (FLOAT) VALIDATION TESTS

    // BEGIN PHONE VALIDATION TESTS

    // END PHONE VALIDATION TESTS

    // BEGIN STRING VALIDATION TESTS

    // END STRING VALIDATION TESTS
    
    // BEGIN URL VALIDATION TESTS

    // END URL VALIDATION TESTS
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

pub trait Validator {
    type Error: ::std::error::Error + std::convert::From<ValidationError>;
    /// Validate text from a textual form field.
    fn validate_text(&self, text: &str) -> Result<(), Self::Error> {
        Err(Self::Error::from(ValidationError::NotImplementedText))
    }

    /// Validate a file submitted from a form.
    fn validate_file(&self, file: &File) -> Result<(), Self::Error> {
        Err(Self::Error::from(ValidationError::NotImplementedFile))
    }

    /// Validate any given field. Defaults to calling the appropriate `validate_*` method based on
    /// the field type.
    fn validate(&self, field: &Field) -> Result<(), Self::Error> {
        match field {
            Field::Text(text) => self.validate_text(text),
            Field::File(file) => self.validate_file(file),
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

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EmailType {
    Html5,
    Rfc5322,
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

#[derive(Serialize, Deserialize)]
pub(crate) struct EmailValidator {
    pub domain_whitelist: Option<HashSet<String>>,
    pub domain_blacklist: Option<HashSet<String>>,
    #[serde(default, alias = "type")]
    pub regex_type: EmailType,
}

impl Validator for EmailValidator {
    type Error = EmailError;
    fn validate_text(&self, text: &str) -> Result<(), Self::Error> {
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

#[derive(Debug)]
pub(crate) enum FileError {
    InvalidContentType(String),
    TooBig(usize),
    Validation(ValidationError),
}

impl From<ValidationError> for FileError {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContentType(content_list) => write!(f, "content type is not among allowed types: {}", content_list),
            Self::TooBig(max_size) => write!(f, "file is larger than {} byte maximum", max_size),
            Self::Validation(err) => write!(f, "{}", err),
        }
    }
}

impl Error for FileError {}

pub(crate) struct FileValidator {
    pub content_types: Option<HashSet<String>>,
    pub max_size: Option<usize>, // Bytes
}

impl Validator for FileValidator {
    type Error = FileError;
    fn validate_file(&self, file: &File) -> Result<(), FileError> {
        match self.max_size {
            Some(size) => {
                if file.bytes.len() > size {
                    return Err(FileError::TooBig(size));
                }
            },
            None => {},
        }

        match &self.content_types {
            Some(type_set) => {
                let as_lower = file.content_type.to_lowercase();
                if !type_set.contains(&as_lower) {
                    let list = join_iter(&mut type_set.iter(), ", ");
                    return Err(FileError::InvalidContentType(list));
                }
            },
            None => {},
        }

        Ok(())
    }
}

pub trait NumberType<T>: FromStr + fmt::Debug + fmt::Display + Ord + Copy {}
impl<T> NumberType<T> for T where T: FromStr + fmt::Debug + fmt::Display + Ord + Copy {}

#[derive(Debug)]
pub(crate) enum NumberError<T> where T: NumberType<T> {
    NotANumber(String),
    ParseFailure(String),
    TooSmall(T),
    TooLarge(T),
    NotInSet(String),
    Validation(ValidationError),
}

impl<T> From<ValidationError> for NumberError<T> where T: NumberType<T> {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
    }
}

impl<T> fmt::Display for NumberError<T> where T: NumberType<T> {
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

impl<T> Error for NumberError<T> where T: NumberType<T> {}

lazy_static! {
    static ref NUMBER_REGEX: Regex = Regex::new(r#"\d+"#).unwrap();
}

pub(crate) struct NumberValidator<T> where T: NumberType<T> {
    pub min: Option<T>,
    pub max: Option<T>,
    pub valid_list: Option<BTreeSet<T>>,
}

impl<T> Validator for NumberValidator<T> where T: NumberType<T>, <T as FromStr>::Err: fmt::Debug {
    type Error = NumberError<T>;
    fn validate_text(&self, text: &str) -> Result<(), Self::Error> {
        if !NUMBER_REGEX.is_match(text) {
            return Err(Self::Error::NotANumber(text.to_string()));
        }

        let num: T = text.parse().map_err(|err| Self::Error::ParseFailure(format!("{:?}", err)))?;

        match &self.valid_list {
            Some(list) => {
                if !list.contains(&num) {
                    return Err(Self::Error::NotInSet(join_iter(&mut list.iter(), ", ")));
                }
            },
            None => {
                if let Some(min) = &self.min {
                    if num < *min {
                        return Err(Self::Error::TooSmall(*min));
                    }
                }

                if let Some(max) = &self.max {
                    if num > *max {
                        return Err(Self::Error::TooLarge(*max));
                    }
                }
            }
        }
        
        Ok(())
    }
}

pub(crate) struct PhoneValidator {
    pub valid_area_codes: Option<Vec<String>>,
}

pub(crate) struct StringValidator {
    pub min_len: usize,
    pub max_len: usize,
    pub regex: Regex,
}

pub(crate) struct UrlValidator {
    pub valid_domains: Option<Vec<String>>,
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
