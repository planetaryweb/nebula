use chrono::{DateTime, FixedOffset};
use serde::{Serialize, Deserialize};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::offset::TimeZone;

    const RESPONSE_ERROR_JSON: &str = r#"
        {
            "success": false,
            "challenge_ts": "2020-12-31T21:59:59.324310806-05:00",
            "hostname": "not-provided",
            "error-codes": [
                "missing-input-secret",
                "invalid-input-secret",
                "missing-input-response",
                "invalid-input-response",
                "bad-request",
                "invalid-or-already-seen-response",
                "sitekey-secret-mismatch"
            ]
        }
    "#;

    const RESPONSE_SUCCESS_JSON: &str = r#"
        {
            "success": true,
            "challenge_ts": "2020-12-31T21:59:59.324310806-05:00",
            "hostname": "example.org",
            "credit": true
        }
    "#;

    #[test]
    fn deserialize_fail_response() {
        let response: Response = serde_json::from_str(RESPONSE_ERROR_JSON)
            .expect("parsing should not fail");
        let expected = Response {
            success: false,
            challenge_ts: FixedOffset::west(5 * 3600).ymd(2020, 12, 31).and_hms_nano(21, 59, 59, 324310806),
            hostname: "not-provided".to_string(),
            credit: None,
            error_codes: vec![
                CaptchaError::MissingSecret,
                CaptchaError::InvalidSecret,
                CaptchaError::MissingResponse,
                CaptchaError::InvalidResponse,
                CaptchaError::BadRequest,
                CaptchaError::DuplicateResponseOrOther,
                CaptchaError::SitekeySecretMismatch,
            ],
        };

        assert_eq!(response, expected);
    }

    #[test]
    fn deserialize_success_response() {
        let response: Response = serde_json::from_str(RESPONSE_SUCCESS_JSON)
            .expect("parsing should not fail");
        let expected = Response {
            success: true,
            challenge_ts: FixedOffset::west(5 * 3600).ymd(2020, 12, 31).and_hms_nano(21, 59, 59, 324310806),
            hostname: "example.org".to_string(),
            credit: Some(true),
            error_codes: vec![],
        };

        assert_eq!(response, expected);
    }
}

pub const FIELD_RESPONSE: &str = "response";
pub const FIELD_SECRET: &str = "secret";
pub const TEST_SITE_KEY: &str = "10000000-ffff-ffff-ffff-000000000001";
pub const TEST_SECRET_KEY: &str = "0x0000000000000000000000000000000000000000";
pub const VERIFY_URL: &str = "https://hcaptcha.com/siteverify";

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum CaptchaError {
    #[serde(rename = "missing-input-secret")]
    MissingSecret,
    #[serde(rename = "invalid-input-secret")]
    InvalidSecret,
    #[serde(rename = "missing-input-response")]
    MissingResponse,
    #[serde(rename = "invalid-input-response")]
    InvalidResponse,
    #[serde(rename = "bad-request")]
    BadRequest,
    #[serde(rename = "invalid-or-already-seen-response")]
    DuplicateResponseOrOther,
    #[serde(rename = "sitekey-secret-mismatch")]
    SitekeySecretMismatch,
}

#[derive(Debug)]
pub enum Error {
    Captcha(CaptchaError),
    Reqwest(reqwest::Error),
    Unknown,
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    success: bool,
    challenge_ts: DateTime<FixedOffset>,
    hostname: String,
    credit: Option<bool>,
    #[serde(default, rename = "error-codes")]
    error_codes: Vec<CaptchaError>,
}

pub struct HCaptcha {
    client: reqwest::Client,
    secret: String,
}

impl HCaptcha {
    pub fn new(secret: String) -> HCaptcha {
        Self {
            client: reqwest::Client::new(),
            secret,
        }
    }

    pub async fn verify(&self, token: &str) -> Result<Response, Error> {
        let params = [(FIELD_SECRET, self.secret.as_str()), (FIELD_RESPONSE, token)];
        let response = self.client.post(VERIFY_URL)
            .form(&params)
            .send()
            .await?
            .json::<Response>()
            .await?;

        if !response.success {
            if response.error_codes.is_empty() {
                Err(Error::Captcha(*response.error_codes.get(0).unwrap()))
            } else {
                Err(Error::Unknown)
            }
        } else {
            Ok(response)
        }
    }
}

pub async fn verify(secret: &str, token: &str) -> Result<Response, Error> {
    let captcha = HCaptcha::new(secret.to_string());
    captcha.verify(token).await
}
