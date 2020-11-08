use super::{join_iter, Validator, ValidationError};
use lazy_static::lazy_static;
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};
use std::collections::BTreeSet;
use std::convert::From;
use std::error::Error;
use std::fmt;
use url::{Url, ParseError};

#[cfg(test)]
mod tests {
    use super::*;

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
        let toml = r#"
            host-whitelist = [ "whitelisted.com" ]
            host-blacklist = [ "blacklisted.com" ]
            schemes = [ "https" ]
        "#;

        serde_
    }

    #[test]
    fn blacklisted_domains_are_blacklisted() {
        let validator = get_validator();
        validator.host_whitelist = None;

        for url in BLACKLISTED_URLS.iter() {
            let err = validator.validate_text(url)
                .expect_err("url should not validate");

            match err {
                UrlError::HostBlacklisted(_) => {},
                err => panic!("expected UrlError::HostBlacklisted, got {:?}", err),
            }
        }
    }

    #[test]
    fn blacklisted_wildcard_subdomains_are_blacklisted() {
        todo!();
    }

    #[test]
    fn blacklisted_domain_does_not_block_subdomain() {
        let validator = get_validator();
        validator.host_whitelist = None;

        for url in BLACKLISTED_SUBDOMAIN_URLS.iter() {
            validator.validate_text(url)
                .expect("url should validate because it doesn't match the blacklisted domain");
        }
    }

    #[test]
    fn whitelisted_domains_are_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;

        for url in WHITELISTED_URLS.iter() {
            validator.validate_text(url)
                .expect("whitelisted url should validate");
        }
    }

    #[test]
    fn non_whitelisted_domains_are_not_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;

        for url in BLACKLISTED_URLS.iter() {
            let err = validator.validate_text(url)
                .expect_err("url should not validate");

            match err {
                UrlError::HostNotWhitelisted(_) => {},
                err => panic!("expected UrlError::HostBlacklisted, got {:?}", err),
            }
        }
    }

    #[test]
    fn non_whitelisted_subdomains_are_not_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;

        for url in WHITELISTED_SUBDOMAIN_URLS.iter() {
            let err = validator.validate_text(url)
                .expect_err("url should not validate");

            match err {
                UrlError::HostNotWhitelisted(_) => {},
                err => panic!("expected UrlError::HostBlacklisted, got {:?}", err),
            }
        }
    }

    #[test]
    fn whitelisted_wildcard_domains_are_allowed() {
        todo!();
    }

    #[test]
    fn allowed_schemes_are_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;
        validator.host_whitelist = None;

        assert!(validator.schemes.expect("schemes must exist for this test").contains("https"));
        // Default validator allows https and all of the following URLs should use https
        for list in vec![ WHITELISTED_URLS, WHITELISTED_SUBDOMAIN_URLS, BLACKLISTED_URLS, BLACKLISTED_SUBDOMAIN_URLS ].iter() {
            for url in list.iter() {
                validator.validate_text(url).expect("HTTPS URLs should validate");
            }
        }
    }

    #[test]
    fn not_explicitly_allowed_schemes_are_not_allowed() {
        let mut validator = get_validator();
        validator.host_blacklist = None;
        validator.host_whitelist = None;

        assert!(!validator.schemes.expect("schemes must exist for this test").contains("http"));

        // Default validator allows https and all of the following URLs should use https
        for list in vec![ WHITELISTED_URLS, WHITELISTED_SUBDOMAIN_URLS, BLACKLISTED_URLS, BLACKLISTED_SUBDOMAIN_URLS ].iter() {
            for url in list.iter() {
                let mut newurl = "http".to_string();
                newurl.push_str(url.strip_prefix("https"));
                let err = validator.validate_text(url).expect_err("HTTP URLs should not validate");

                match err {
                    UrlError::SchemeNotWhitelisted(_) => {},
                    err => panic!("expected UrlError::SchemeNotWhitelisted, got {:?}", err),
                }
            }
        }
    }

    #[test]
    fn allowed_schemes_without_required_hosts_are_invalid() {

    }

    #[test]
    fn common_schemes_requiring_hosts_automatically_require_them() {
        for scheme in SCHEMES_REQ_HOSTS_DEFAULT.iter() {
            
        }
    }
}

#[derive(Debug)]
pub(crate) enum UrlError {
    HostBlacklisted(String),
    HostMissing(String),
    HostNotWhitelisted(String),
    Parse(ParseError),
    SchemeNotWhitelisted(String),
    Validation(ValidationError),
}

impl From<ValidationError> for UrlError {
    fn from(err: ValidationError) -> Self {
        Self::Validation(err)
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
            Self::Parse(err) => write!(f, "Invalid URL: {}", err),
            Self::SchemeNotWhitelisted(list) => write!(f, "URL scheme must be one of the following: {}", list),
            Self::Validation(err) => write!(f, "{}", err),
        }
    }
}

impl Error for UrlError {}

lazy_static! {
    static ref SCHEMES_REQ_HOSTS_DEFAULT: BTreeSet<String> = {
        let mut schemes_requiring_hosts = BTreeSet::new();
        schemes_requiring_hosts.insert("feed".to_string());
        schemes_requiring_hosts.insert("file".to_string());
        schemes_requiring_hosts.insert("ftp".to_string());
        schemes_requiring_hosts.insert("git".to_string());
        schemes_requiring_hosts.insert("gopher".to_string());
        schemes_requiring_hosts.insert("http".to_string());
        schemes_requiring_hosts.insert("https".to_string());
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
        schemes_requiring_hosts
    };
}

pub(crate) struct UrlValidator {
    pub host_blacklist: Option<BTreeSet<String>>,
    pub host_whitelist: Option<BTreeSet<String>>,
    pub schemes_requiring_hosts: BTreeSet<String>,
    pub schemes: Option<BTreeSet<String>>,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename = "lowercase")]
enum Field { HostBlacklist, HostWhitelist, SchemesRequiringHosts, Schemes }

struct UrlVisitor;

impl<'de> Visitor<'de> for UrlVisitor {
    type Value = UrlValidator;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("URL validator")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error> where V: de::MapAccess<'de> {
        let mut host_blacklist = None;
        let mut host_whitelist = None;
        let mut schemes = None;
        let mut schemes_requiring_hosts = None;

        while let Some(key) = map.next_key()? {
            match key {
                Field::HostBlacklist => {
                    super::set_option_field("host_blacklist", &mut host_blacklist, &mut map)?;
                },
                Field::HostWhitelist => {
                    super::set_option_field("host_whitelist", &mut host_whitelist, &mut map)?;
                },
                Field::Schemes => {
                    super::set_option_field("schemes", &mut schemes, &mut map)?;
                },
                Field::SchemesRequiringHosts => {
                    super::set_option_field("schemes_required_hosts", &mut schemes_requiring_hosts, &mut map)?;
                }
            }
        }

        let mut schemes_requiring_hosts = schemes_requiring_hosts
            .map(|tree| SCHEMES_REQ_HOSTS_DEFAULT.union(&tree).cloned().collect())
            .unwrap_or_else(|| SCHEMES_REQ_HOSTS_DEFAULT.clone());

        Ok(Self::Value { host_blacklist, host_whitelist, schemes, schemes_requiring_hosts })
    }
}

impl<'de> Deserialize<'de> for UrlValidator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_map(UrlVisitor)
    }
}

impl Validator for UrlValidator {
    type Error = UrlError;
    fn validate_text(&self, text: &str) -> Result<(), Self::Error> {
        let url = Url::parse(text)?;

        if let Some(schemes) = &self.schemes {
            if !schemes.contains(url.scheme()) {
                return Err(Self::Error::SchemeNotWhitelisted(join_iter(&mut schemes.iter(), ", ")));
            }
        }

        // Only test hosts if URL has a host. Some do not.
        if let Some(host) = url.host_str() {
            if let Some(hosts) = &self.host_whitelist {
                if !hosts.contains(host) {
                    return Err(Self::Error::HostNotWhitelisted(join_iter(&mut hosts.iter(), ", ")));
                }
            } else if let Some(hosts) = &self.host_blacklist {
                if hosts.contains(host) {
                    return Err(Self::Error::HostBlacklisted(join_iter(&mut hosts.iter(), ", ")));
                }
            }
        } else {
            if self.schemes_requiring_hosts.contains(url.scheme()) {
                return Err(Self::Error::HostMissing(url.scheme().to_string()));
            }
        }

        Ok(())
    }
}