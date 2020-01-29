use lettre::{SendableEmail, Transport};
use lettre::sendmail::{error::SendmailResult, SendmailTransport};
use lettre::smtp::{error::SmtpResult, SmtpTransport};
use serde::Deserialize;

pub enum Sender {
    SMTP(SmtpTransport),
    Sendmail(SendmailTransport),
}

impl<'a> Transport<'a> for Sender {
    type Result = Result<(), String>;

    fn send(&mut self, email: SendableEmail) -> Self::Result {
        match self {
            Sender::SMTP(smtp) => {
                
            },
            Sender::Sendmail(send) => {

            }
        }

        Ok(())
    }
}
