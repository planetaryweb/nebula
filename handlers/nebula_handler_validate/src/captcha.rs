//
// captcha.rs
// Copyright (C) 2020 shadow53 <shadow53@shadow53.com>
// Distributed under terms of the MIT license.
//

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
	}
}

enum CaptchaType {
    ReCaptcha,
    HCaptcha,
}

struct Captcha {
    pub typ: CaptchaType,
    pub api_secret: String,
    pub field_name: String,
}
