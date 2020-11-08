use super::{join_iter, ValidationError, Validator};
use nebula_form::FormFile as File;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    
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