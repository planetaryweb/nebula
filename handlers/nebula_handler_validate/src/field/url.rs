use crate::join_iter;

use super::{Validator, ValidationError};
use nebula_rpc::config::{Config, ConfigError, ConfigExt};
use lazy_static::lazy_static;
use std::collections::BTreeSet;
use std::convert::{From, TryFrom};
use std::error::Error;
use std::fmt;
use url::{Url, ParseError, SyntaxViolation};

#[cfg(test)]
mod tests {
    use super::*;
    use nebula_rpc::config::Value;

    lazy_static! {
        static ref BLACKLISTED_URLS: Vec<&'static str> = vec! [
            "https://blacklisted.com/",
            "https://blacklisted.com/foobar",
            "https://blacklisted.com/file.txt",
            "https://blacklisted.com/subdir?foo=blah&bar=baz%20quux",
        ];

        static ref BLACKLISTED_SUBDOMAIN_URLS: Vec<&'static str> = vec! [
            "https://subdomain.blacklisted.com/",
            "https://subdomain.blacklisted.com/foobar",
            "https://nested.subdomain.blacklisted.com/file.txt",
            "https://nested.subdomain.blacklisted.com/subdir?foo=blah&bar=baz%20quux",
        ];

        static ref WHITELISTED_URLS: Vec<&'static str> = vec! [
            "https://whitelisted.com/",
            "https://whitelisted.com/foobar",
            "https://whitelisted.com/file.txt",
            "https://whitelisted.com/subdir?foo=blah&bar=baz%20quux",
        ];

        static ref WHITELISTED_SUBDOMAIN_URLS: Vec<&'static str> = vec! [
            "https://subdomain.whitelisted.com/",
            "https://subdomain.whitelisted.com/foobar",
            "https://nested.subdomain.whitelisted.com/file.txt",
            "https://nested.subdomain.whitelisted.com/subdir?foo=blah&bar=baz%20quux",
        ];
    }

    fn get_validator() -> UrlValidator {
        let config = {
            let mut config = Config::new();
            config.insert(UrlValidator::FIELD_HOST_WHITELIST.to_owned(), Value::LeafList(vec!["whitelisted.com".to_owned()]));
            config.insert(UrlValidator::FIELD_HOST_BLACKLIST.to_owned(), Value::LeafList(vec!["blacklisted.com".to_owned()]));
            config.insert(UrlValidator::FIELD_SCHEMES.to_owned(), Value::LeafList(vec!["https".to_owned()]));
            config
        };

        UrlValidator::try_from(config).expect("test validator should successfully be created")
    }

    #[test]
    fn blacklisted_domains_are_blacklisted() {
        let mut validator = get_validator();
        validator.host_whitelist = None;

        for url in BLACKLISTED_URLS.iter() {
            let err = validator.do_validate(url)
                .expect_err("url should not validate");

            match err {
                UrlError::HostBlacklisted(_) => {},
                err => panic!("expected UrlError::HostBlacklisted, got {:?}", err),
            }
        }
    }

    #[test]
    fn blacklisted_domain_does_not_block_subdomain() {
        let mut validator = get_validator();
        validator.host_whitelist = None;

        for url in BLACKLISTED_SUBDOMAIN_URLS.iter() {
            validator.do_validate(url)
                .expect("url should validate because it doesn't match the blacklisted domain");
        }
    }

    #[test]
    fn whitelisted_domains_are_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;

        for url in WHITELISTED_URLS.iter() {
            validator.do_validate(url)
                .expect("whitelisted url should validate");
        }
    }

    #[test]
    fn non_whitelisted_domains_are_not_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;

        for url in BLACKLISTED_URLS.iter() {
            let err = validator.do_validate(url)
                .expect_err("url should not validate");

            match err {
                UrlError::HostNotWhitelisted(_) => {},
                err => panic!("expected UrlError::HostNotWhitelisted, got {:?}", err),
            }
        }
    }

    #[test]
    fn non_whitelisted_subdomains_are_not_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;

        for url in WHITELISTED_SUBDOMAIN_URLS.iter() {
            let err = validator.do_validate(url)
                .expect_err("url should not validate");

            match err {
                UrlError::HostNotWhitelisted(_) => {},
                err => panic!("expected UrlError::HostNotWhitelisted, got {:?}", err),
            }
        }
    }

    #[test]
    fn allowed_schemes_are_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;
        validator.host_whitelist = None;

        assert!(validator.schemes.as_ref().expect("schemes must exist for this test").contains("https"));
        // Default validator allows https and all of the following URLs should use https
        for list in vec![ WHITELISTED_URLS.iter(), WHITELISTED_SUBDOMAIN_URLS.iter(), BLACKLISTED_URLS.iter(), BLACKLISTED_SUBDOMAIN_URLS.iter() ].into_iter() {
            for url in list {
                validator.do_validate(url).expect("HTTPS URLs should validate");
            }
        }
    }

    #[test]
    fn not_explicitly_allowed_schemes_are_not_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;
        validator.host_whitelist = None;

        assert!(!validator.schemes.as_ref().expect("schemes must exist for this test").contains("http"),
            "schemes must not contain 'http' for this test");

        // Default validator allows https and all of the following URLs should use https
        for list in vec![ WHITELISTED_URLS.iter(), WHITELISTED_SUBDOMAIN_URLS.iter(), BLACKLISTED_URLS.iter(), BLACKLISTED_SUBDOMAIN_URLS.iter() ].into_iter() {
            for url in list {
                let mut newurl = "http".to_string();
                newurl.push_str(url.strip_prefix("https").unwrap());
                let err = validator.do_validate(&newurl).expect_err(format!("HTTP URLs ({}) should not validate", newurl).as_str());

                match err {
                    UrlError::SchemeNotWhitelisted(_) => {},
                    err => panic!("expected UrlError::SchemeNotWhitelisted, got {:?}", err),
                }
            }
        }
    }

    #[test]
    fn allowed_schemes_without_required_hosts_are_invalid() {
        // This validator should only require https, no specific hostname matching
        let validator = UrlValidator {
            host_blacklist: None,
            host_whitelist: None,
            schemes_requiring_hosts: SCHEMES_REQ_HOSTS_DEFAULT.clone(),
            schemes: Some(vec!["https"].into_iter().map(String::from).collect()),
        };

        let invalid_uris = vec!["https:///path/to/file", "https://?key1=val1&key2=val2"];

        for uri in invalid_uris {
            let err = validator.do_validate(uri).expect_err(&format!("https uris without hosts should not validate: {}", uri));

            match err {
                UrlError::HostMissing(_) => {},
                // Some missing hosts produce a syntax violation instead
                UrlError::SyntaxViolation(SyntaxViolation::ExpectedDoubleSlash) => {},
                UrlError::Parse(err) => panic!("uri ({}) should not fail parsing: {}", uri, err),
                err => panic!("expected UrlError::HostMissing, got {:?}", err),
            }
        }
    }

    #[test]
    fn common_schemes_requiring_hosts_automatically_require_them() {
        let config = Config::new();

        let validator = UrlValidator::try_from(config)
            .expect("validator should generate from empty config");

        for scheme in SCHEMES_REQ_HOSTS_DEFAULT.iter() {
            for suffix in vec![":///path/to/resource", "://?key1=val1"] {
                let mut uri = scheme.to_string();
                uri.push_str(suffix);

                {
                    if let Ok(url) = Url::parse(&uri) {
                        if let Some(host) = url.host_str() {
                            println!("scheme: '{}'\thost: '{}'\turi: '{}'", url.scheme(), host, uri);
                        }
                    }
                }

                let err = validator.do_validate(&uri)
                    .expect_err(&format!("URI scheme {} should require host and not validate: {}", scheme, uri));

                match err {
                    UrlError::HostMissing(_) => {},
                    // Some missing hosts produce a syntax violation instead
                    UrlError::SyntaxViolation(SyntaxViolation::ExpectedDoubleSlash) => {},
                    UrlError::Parse(err) => panic!("unexpected parser error for uri ({}): {}", uri, err),
                    err => panic!("expected UrlError::HostMissing, got {:?}", err),
                }
            }
        }
    }
}

fn parse_syntax_violations_are_errors(uri: &str) -> Result<Url, UrlError> {
    use std::cell::RefCell;
    let violation = RefCell::new(None);
    let url = Url::options()
        .syntax_violation_callback(Some(&|v| { violation.replace(Some(v)); }))
        .parse(uri)
        .map_err(|err| match err {
            ParseError::EmptyHost => UrlError::HostMissing(uri.to_string()),
            err => UrlError::Parse(err),
        })?;       

    match violation.into_inner() {
        Some(v) => Err(UrlError::SyntaxViolation(v)),
        None => Ok(url),
    }
}

#[derive(Debug)]
pub enum UrlError {
    HostBlacklisted(String),
    HostMissing(String),
    HostNotWhitelisted(String),
    SchemeNotWhitelisted(String),
    Parse(ParseError),
    SyntaxViolation(SyntaxViolation),
}

impl From<UrlError> for ValidationError {
    fn from(err: UrlError) -> Self {
        Self::InvalidInput(err.to_string())
    }
}

impl From<ParseError> for UrlError {
    fn from(err: ParseError) -> Self {
        Self::Parse(err)
    }
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostBlacklisted(list) => write!(f, "URLs not allowed from the following: {}", list),
            Self::HostMissing(scheme) => write!(f, "The {} scheme requires a host/domain", scheme),
            Self::HostNotWhitelisted(list) => write!(f, "URLs must be from one of the following: {}", list),
            Self::Parse(err) => write!(f, "Failed to parse URL: {}", err),
            Self::SchemeNotWhitelisted(list) => write!(f, "URL scheme must be one of the following: {}", list),
            Self::SyntaxViolation(v) => write!(f, "URL syntax is invalid: {}", v),
        }
    }
}

impl Error for UrlError {}

lazy_static! {
    static ref SCHEMES_REQ_HOSTS_DEFAULT: BTreeSet<String> = {
        let mut schemes_requiring_hosts = BTreeSet::new();
        schemes_requiring_hosts.insert("feed".to_string());
        schemes_requiring_hosts.insert("git".to_string());
        schemes_requiring_hosts.insert("imap".to_string());
        schemes_requiring_hosts.insert("irc".to_string());
        schemes_requiring_hosts.insert("irc6".to_string());
        schemes_requiring_hosts.insert("ircs".to_string());
        schemes_requiring_hosts.insert("rsync".to_string());
        schemes_requiring_hosts.insert("rtmp".to_string());
        schemes_requiring_hosts.insert("rtmfp".to_string());
        schemes_requiring_hosts.insert("sftp".to_string());
        schemes_requiring_hosts.insert("sip".to_string());
        schemes_requiring_hosts.insert("snmp".to_string());
        schemes_requiring_hosts.insert("ssh".to_string());
        schemes_requiring_hosts.insert("stun".to_string());
        schemes_requiring_hosts.insert("stuns".to_string());
        schemes_requiring_hosts.insert("telnet".to_string());
        schemes_requiring_hosts.insert("tftp".to_string());
        schemes_requiring_hosts.insert("turn".to_string());
        schemes_requiring_hosts.insert("turns".to_string());
        schemes_requiring_hosts.insert("xmpp".to_string());

        
        /*
         * The following commented-out schemes are designated "special" by the whatwg standard and
         * appear to be host-validated by `Url::Parse`.
         *
         * (file:// allows omitting `localhost`)
         * schemes_requiring_hosts.insert("file".to_string());
         */
         schemes_requiring_hosts.insert("ftp".to_string());
         schemes_requiring_hosts.insert("http".to_string());
         schemes_requiring_hosts.insert("https".to_string());
         schemes_requiring_hosts.insert("ws".to_string());
         schemes_requiring_hosts.insert("wss".to_string());
         /*
         * The following appear to also be host-validated by `Url::Parse`.
         */ 
         schemes_requiring_hosts.insert("gopher".to_string());
         /**/
        schemes_requiring_hosts
    };
}

pub struct UrlValidator {
    pub host_blacklist: Option<BTreeSet<String>>,
    pub host_whitelist: Option<BTreeSet<String>>,
    pub schemes_requiring_hosts: BTreeSet<String>,
    pub schemes: Option<BTreeSet<String>>,
}

impl UrlValidator {
    const FIELD_HOST_BLACKLIST: &'static str = "host-blacklist";
    const FIELD_HOST_WHITELIST: &'static str = "host-whitelist";
    const FIELD_SCHEMES_REQUIRING_HOSTS: &'static str = "schemes-requiring-hosts";
    const FIELD_SCHEMES: &'static str = "schemes";

    fn do_validate(&self, text: &str) -> Result<(), UrlError> {
let url = parse_syntax_violations_are_errors(text)?;


        if let Some(schemes) = &self.schemes {
            if !schemes.contains(url.scheme()) {
                return Err(UrlError::SchemeNotWhitelisted(join_iter(&mut schemes.iter(), ", ")));
            }
        }

        // Only test hosts if URL has a host. Some do not.
        if let Some(host) = url.host_str() {
            if let Some(hosts) = &self.host_whitelist {
                if !hosts.contains(host) {
                    return Err(UrlError::HostNotWhitelisted(join_iter(&mut hosts.iter(), ", ")));
                }
            } else if let Some(hosts) = &self.host_blacklist {
                if hosts.contains(host) {
                    return Err(UrlError::HostBlacklisted(join_iter(&mut hosts.iter(), ", ")));
                }
            }
        } else {
            if self.schemes_requiring_hosts.contains(url.scheme()) {
                return Err(UrlError::HostMissing(url.scheme().to_string()));
            }
        }

        Ok(())
    }
}

impl TryFrom<Config> for UrlValidator {
    type Error = ConfigError;
    fn try_from(config: Config) -> Result<Self, ConfigError> {
        let host_blacklist = config.get_path_list(Self::FIELD_HOST_BLACKLIST)?;
        let host_whitelist = config.get_path_list(Self::FIELD_HOST_WHITELIST)?;
        let schemes = config.get_path_list(Self::FIELD_SCHEMES)?;
        let schemes_requiring_hosts = config.get_path_list(Self::FIELD_SCHEMES_REQUIRING_HOSTS)?
            .unwrap_or_else(|| {
                match &schemes {
                    None => SCHEMES_REQ_HOSTS_DEFAULT.clone(),
                    Some(set) => SCHEMES_REQ_HOSTS_DEFAULT.intersection(set).cloned().collect(),
                }
            });
        Ok(Self { host_blacklist, host_whitelist, schemes, schemes_requiring_hosts })
    }
}

impl Validator for UrlValidator {
    fn validate_text(&self, text: &str) -> crate::Result {
        self.do_validate(text).map_err(Into::into)
    }

    fn try_from_config(config: Config) -> Result<Self, ConfigError> where Self: Sized {
        Self::try_from(config)
    }
}
