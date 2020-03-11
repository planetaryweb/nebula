use crate::sender::Sender;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::sync::Arc;
use lettre::smtp::{SmtpClient, SmtpTransport, authentication::Credentials};
use lettre::sendmail::SendmailTransport;
use serde::{Deserialize, de::{self, Deserializer, Visitor, MapAccess}};
use tera::{Context, Tera};
use tokio::sync::RwLock;

#[cfg(test)]
mod tests {
    use super::deserialize_tera;
    use super::{Handler, SenderConfig};
    use super::{FIELD_TO, FIELD_SUBJECT, FIELD_BODY, FIELD_REPLY_TO, FIELD_CC, FIELD_BCC};
    use serde::de::IntoDeserializer;
    use tera::{Context, Tera};
    use toml;

    const SMTP_CONFIG_BARE_TOML: &str = r#"
host = "smtp.gmail.com"
port = 587
user = "example@gmail.com"
pass = """My super "secure" GMail p@ssw0rd"""
"#;

    #[test]
    fn test_smtp_config_bare_toml() {
        let conf: SenderConfig = toml::from_str(SMTP_CONFIG_BARE_TOML).unwrap();
        match conf {
            SenderConfig::Sendmail(_) =>
                panic!("incorrectly parsed smtp config as sendmail config"),
            SenderConfig::SMTP(smtp) => {
                assert_eq!(smtp.host, "smtp.gmail.com");
                assert_eq!(smtp.port, 587u32);
                assert_eq!(smtp.user, "example@gmail.com");
                assert_eq!(smtp.pass, r#"My super "secure" GMail p@ssw0rd"#);
                assert_eq!(smtp.from, None);
            }
        }
    }
    
    const SMTP_CONFIG_FULL_TOML: &str = r#"
host = "smtp.gmail.com"
port = 587
user = "example@gmail.com"
pass = """My super "secure" GMail p@ssw0rd"""
from = "example+extratext@gmail.com"
"#;

    #[test]
    fn test_smtp_config_full_toml() {
        let conf: SenderConfig = toml::from_str(SMTP_CONFIG_FULL_TOML).unwrap();
        match conf {
            SenderConfig::Sendmail(_) =>
                panic!("incorrectly parsed smtp config as sendmail config"),
            SenderConfig::SMTP(smtp) => {
                assert_eq!(smtp.host, "smtp.gmail.com");
                assert_eq!(smtp.port, 587u32);
                assert_eq!(smtp.user, "example@gmail.com");
                assert_eq!(smtp.pass, r#"My super "secure" GMail p@ssw0rd"#);
                assert_eq!(smtp.from, Some(String::from("example+extratext@gmail.com")));
            }
        }
    }

    const SENDMAIL_BARE_TOML: &str = r#"
from = "admin@example.org"
"#;

    fn test_sendmail_bare_toml_to_config() {
        let conf: SenderConfig = toml::from_str(SMTP_CONFIG_BARE_TOML).unwrap();
        match conf {
            SenderConfig::SMTP(_) =>
                panic!("incorrectly parsed sendmail config as smtp config"),
            SenderConfig::Sendmail(sm) => {
                assert_eq!(sm.bin, None);
                assert_eq!(sm.from, "admin@example.org");
            }
        }
    }

    const SENDMAIL_FULL_TOML: &str = r#"
from = "admin@example.org"
bin = "/usr/local/bin/sendmail"
"#;
    
    fn test_sendmail_bare_full_to_config() {
        let conf: SenderConfig = toml::from_str(SMTP_CONFIG_BARE_TOML).unwrap();
        match conf {
            SenderConfig::SMTP(_) =>
                panic!("incorrectly parsed sendmail config as smtp config"),
            SenderConfig::Sendmail(sm) => {
                assert_eq!(sm.bin, Some(String::from("/usr/local/bin/sendmail")));
                assert_eq!(sm.from, "admin@example.org");
            }
        }
    }

    const CONFIG_BARE_SMTP: &str = r#"
name = "test-smtp"
[sender]
    host = "smtp.gmail.com"
    port = 587
    user = "example@gmail.com"
    pass = """My super "secure" GMail p@ssw0rd"""
[templates]
    to = "admin@example.org"
    subject = "Test Subject"
    body = """
The template parsing is mostly tested by other tests.
The SMTP tests will only validate that one template exists to ensure that
the templates were parsed at all.
"""
"#;

    #[test]
    fn test_config_bare_smtp() {
        let conf: Handler = toml::from_str(CONFIG_BARE_SMTP).unwrap();
        assert_eq!(conf.name, "test-smtp");
        assert_eq!(conf.depends.len(), 0);
        assert_eq!(conf.templates.render(FIELD_TO, &Context::new()).unwrap(), "admin@example.org");
    }

    const CONFIG_FULL_SMTP: &str = r#"
name = "test-smtp"
depends = ["testdep"]
[sender]
    host = "smtp.gmail.com"
    port = 587
    user = "example@gmail.com"
    pass = """My super "secure" GMail p@ssw0rd"""
    from = "example+extratext@gmail.com"
[templates]
    to = "admin@example.org"
    #from = "example+templates@gmail.com"
    subject = "Test Subject"
    body = """
The template parsing is mostly tested by other tests.
The SMTP tests will only validate that one template exists to ensure that
the templates were parsed at all.
"""
    reply_to = "user@domain.net"
    cc = "ccme@example.org"
    bcc = "bccme@example.com"
"#;

    #[test]
    fn test_config_full_smtp() {
        let conf: Handler = toml::from_str(CONFIG_FULL_SMTP).unwrap();
        assert_eq!(conf.name, "test-smtp");
        assert_eq!(conf.depends.len(), 1);
        assert_eq!(conf.depends[0], String::from("testdep"));
        assert_eq!(conf.templates.render(FIELD_REPLY_TO, &Context::new()).unwrap(), "user@domain.net");
    }

    const BARE_TMPL_CONFIG: &str = r#"
to = "admin@example.org"
subject = "Example subject"
body = """
This is the body of the message.

I am testing out multiline TOML strings.
"""
    "#;

    const FULL_TMPL_CONFIG: &str = r#"
to = "admin@example.org"
subject = "Example subject"
#from = "from-me@example.org"
reply_to = "user@domain.net"
cc = "ccme@example.org"
bcc = "bccme@example.org"
body = """
This is the body of the message.

I am testing out multiline TOML strings.
"""
    "#;

    #[test]
    fn test_bare_toml_to_config() {
        let tmpl: Tera = deserialize_tera(toml::de::Deserializer::new(BARE_TMPL_CONFIG).into_deserializer()).unwrap();

        let pairs = vec!(
            (FIELD_TO, "admin@example.org"),
            (FIELD_SUBJECT, "Example subject"),
            (FIELD_BODY, r#"This is the body of the message.

I am testing out multiline TOML strings.
"#)
        );

        for pair in pairs {
            match tmpl.render(pair.0, &Context::new()) {
                Ok(val) => assert_eq!(val, pair.1),
                Err(err) => panic!(err),
            }
        }
    }
    
    #[test]
    fn test_full_toml_to_config() {
        let tmpl: Tera = deserialize_tera(toml::de::Deserializer::new(FULL_TMPL_CONFIG).into_deserializer()).unwrap();

        let pairs = vec!(
            (FIELD_TO, "admin@example.org"),
            (FIELD_SUBJECT, "Example subject"),
            (FIELD_BODY, r#"This is the body of the message.

I am testing out multiline TOML strings.
"#),
            (FIELD_REPLY_TO, "user@domain.net"),
            //(FIELD_FROM, "from-me@example.org"),
            (FIELD_CC, "ccme@example.org"),
            (FIELD_BCC, "bccme@example.org"),
        );

        for pair in pairs {
            match tmpl.render(pair.0, &Context::new()) {
                Ok(val) => assert_eq!(val, pair.1),
                Err(err) => panic!(err),
            }
        }
    }
}

/// The main entry point for this crate. It will implement a `Handler` trait
/// from the `nebula_core` crate, once that gets fleshed out.
#[derive(Deserialize)]
pub struct Handler {
    // TODO: Move some items to a HandlerBase
    /// The name of the Handler.
    name: String,
    /// A `Sender` configured to send this email.
    #[serde(deserialize_with = "deserialize_sender")]
    sender: Sender,
    /// A (possibly empty) list of other `Handler`s this depends on.
    #[serde(default)]
    depends: Vec<String>,
    /// A `Tera` object loaded with all of the necessary templates for
    /// generating the email message.
    #[serde(deserialize_with = "deserialize_tera")]
    templates: Tera,
    /// An optional list of form field names containing files to attach to the
    /// email message.
    files: Option<Vec<String>>,
}

/// An intermediate struct for parsing configurations for sending emails
/// through an SMTP server.
#[derive(Deserialize)]
struct SmtpConfig {
    /// The hostname of the SMTP server, e.g. `"smtp.gmail.com"`
    host: String,
    /// The port used to access the SMTP server. Usually `25`, `465`, `587`,
    /// or `2525`.
    port: u32,
    /// The username for authenticating with the SMTP server.
    user: String,
    /// The password for authenticating with the SMTP server.
    pass: String,
    /// An optional email address to use in the `From` header. If not provided,
    /// defaults to the value of `user`.
    from: Option<String>,
}

/// An intermediate struct for parsing configurations for sending emails
/// using `sendmail`.
#[derive(Deserialize)]
struct SendmailConfig {
    /// The path to use to invoke `sendmail`. If not provided, follows the
    /// default from the `lettre` crate, currently `/usr/sbin/sendmail`.
    bin: Option<String>,
    /// The email address to use in the `From` header. Required for use with
    /// `sendmail`.
    from: String,
}

/// An enum to help get `serde` to parse one of either kind of `Sender`.
#[derive(Deserialize)]
#[serde(untagged)]
enum SenderConfig {
    /// An SMTP configuration
    SMTP(SmtpConfig),
    /// A Sendmail configuration
    Sendmail(SendmailConfig),
}

/// Helper type for parsing templates into a single `Tera` object.
struct TemplateVisitor;

/// An enum used by `serde` and the `deserialize_tera` function to parse
/// templates.
#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum TemplateField {
    To,
    //From,
    Subject,
    Body,
    #[serde(rename = "reply_to")]
    ReplyTo,
    CC,
    BCC,
}

/// The configuration field name for the BCC template
static FIELD_BCC: &str = "bcc";
/// The configuration field name for the email body template
static FIELD_BODY: &str = "body";
/// The configuration field name for the CC template
static FIELD_CC: &str = "cc";
/// The 
//static FIELD_FROM: &str = "from";
/// The configuration field name for the Reply-To template
static FIELD_REPLY_TO: &str = "reply_to";
/// The configuration field name for the Subject line
static FIELD_SUBJECT: &str = "subject";
/// The configuration field name for the To template
static FIELD_TO: &str = "to";

impl<'de> TemplateVisitor {
    /// Helper function for parsing a configuration option into an `Option`.
    /// Returns an `Error` if the `Option` already has a value set.
    fn helper_option<M,V>(map: &mut M, var: &mut Option<V>, name: &'static str) -> Result<(), M::Error> where M: MapAccess<'de>, V: Deserialize<'de> {
        if var.is_some() {
            return Err(de::Error::duplicate_field(name));
        }
        *var = Some(map.next_value::<V>()?);
        Ok(())
    }

    fn validate_exists<V,E>(var: Option<V>, name: &'static str) -> Result<V, E> where E: de::Error {
        var.ok_or_else(|| de::Error::missing_field(name))
    }
}

impl<'de> Visitor<'de> for TemplateVisitor {
    /// The type that this `Visitor` generates
    type Value = Tera;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Tera template as a string")
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error> where V: MapAccess<'de> {
        let mut tera = Tera::default();

        let mut to = None;
        //let mut from = None;
        let mut subject = None;
        let mut body = None;
        let mut reply_to = None;
        let mut cc = None;
        let mut bcc = None;

        while let Some(key) = map.next_key()? {
            match key {
                TemplateField::To => TemplateVisitor::helper_option(&mut map, &mut to, FIELD_TO)?,
                TemplateField::Subject => TemplateVisitor::helper_option(&mut map, &mut subject, FIELD_SUBJECT)?,
                TemplateField::Body => TemplateVisitor::helper_option(&mut map, &mut body, FIELD_BODY)?,
                TemplateField::ReplyTo => TemplateVisitor::helper_option(&mut map, &mut reply_to, FIELD_REPLY_TO)?,
                TemplateField::CC => TemplateVisitor::helper_option(&mut map, &mut cc, FIELD_CC)?,
                TemplateField::BCC => TemplateVisitor::helper_option(&mut map, &mut bcc, FIELD_BCC)?,
                //TemplateField::From => TemplateVisitor::helper_option(&mut map, &mut from, FIELD_FROM)?,
            }
        }

        let to = TemplateVisitor::validate_exists(to, FIELD_TO)?;
        let subject = TemplateVisitor::validate_exists(subject, FIELD_SUBJECT)?;
        let body = TemplateVisitor::validate_exists(body, FIELD_BODY)?;
        if let Err(err) = tera.add_raw_templates(vec![
            (FIELD_TO, to),
            (FIELD_SUBJECT, subject),
            (FIELD_BODY, body),
        ]) {
            return Err(de::Error::custom(err));
        }

        if let Some(val) = reply_to {
            if let Err(err) = tera.add_raw_template(FIELD_REPLY_TO, val) {
                return Err(de::Error::custom(err));
            }
        }

        //if let Some(val) = from {
        //    if let Err(err) = tera.add_raw_template(FIELD_FROM, val) {
        //        return Err(de::Error::custom(err));
        //    }
        //}

        if let Some(val) = cc {
            if let Err(err) = tera.add_raw_template(FIELD_CC, val) {
                return Err(de::Error::custom(err));
            }
        }

        if let Some(val) = bcc {
            if let Err(err) = tera.add_raw_template(FIELD_BCC, val) {
                return Err(de::Error::custom(err));
            }
        }

        Ok(tera)
    }
}

/// Parses a map into a `Tera` object.
fn deserialize_tera<'de, D> (deserializer: D) -> Result<Tera, D::Error> where D: Deserializer<'de> {
    deserializer.deserialize_map(TemplateVisitor)
}

/// Parses a map into a `Sender`.
fn deserialize_sender<'de, D> (deserializer: D) -> Result<Sender, D::Error> where D: Deserializer<'de> {
    match SenderConfig::deserialize(deserializer)? {
        SenderConfig::SMTP(smtp) => {
            match smtp.try_into() {
                Ok(t) => Ok(Sender::SMTP(t)),
                Err(err) => Err(de::Error::custom(err)),
            }
        },
        SenderConfig::Sendmail(sm) => {
            match sm.try_into() {
                Ok(t) => Ok(Sender::Sendmail(t)),
                Err(err) => Err(de::Error::custom(err)),
            }
        },
    }
}

impl Handler {
    /// Takes ownership of this `Handler` and stores it inside of a Tokio
    /// `RwLock` inside of an `Arc`.
    pub fn to_arc_rwlock(self) -> Arc<RwLock<Handler>> {
        Arc::new(RwLock::new(self))
    }

    //pub fn parse_template(&self, ctx: &Context) -> String {
    //    let mut tera = Tera::default();
    //}
}

impl TryInto<SmtpTransport> for SmtpConfig {
    type Error = lettre::smtp::error::Error;

    fn try_into(self) -> Result<SmtpTransport, Self::Error> {
        Ok(SmtpClient::new_simple(&self.host)?
            .smtp_utf8(true)
            .credentials(Credentials::new(self.user, self.pass))
            .transport())
    }
}

impl TryInto<SendmailTransport> for SendmailConfig {
    type Error = String;

    fn try_into(self) -> Result<SendmailTransport, Self::Error> {
        Ok(match self.bin {
            Some(path) => SendmailTransport::new_with_command(path),
            None => SendmailTransport::new(),
        })
    }
}
