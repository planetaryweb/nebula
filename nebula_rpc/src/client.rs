use crate::config::Config;
use crate::convert::{self, FromRPC, IntoRPC};
use crate::rpc::handler_client::HandlerClient;
use bytes::Bytes;
use http::uri::InvalidUri;
use nebula_form::Form;
use nebula_status::Status;
use nix::sys::signal::{kill as send_signal, Signal};
use nix::unistd::Pid;
use std::io::Error as IOError;
use std::process::{Child, Command};
use tonic::transport::{channel::Channel, Error as TransportError, Uri};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}

#[derive(Debug)]
pub enum Error {
    ClientExists(String),
    ClientNoExists(String),
    Command(IOError),
    Connect(TransportError),
    Convert(convert::Error),
    HandlerExists(String),
    HandlerNoExists(String),
    InvalidUri(InvalidUri),
    RPC(tonic::Status),
    Signal(nix::Error),
}

pub struct ClientArgs {
    pub name: String,
    pub args: Vec<String>,
    pub addr: String,
}

pub struct Client {
    program: Option<Child>,
    client: HandlerClient<Channel>,
    args: Vec<String>,
}

impl Client {
    pub async fn new(addr: String, args: Vec<String>) -> Result<Self, Error> {
        let program = {
            args.get(0)
                .map(|cmd| {
                    let mut cmd = Command::new(cmd);
                    if args.len() > 1 {
                        cmd.args(args.iter().skip(1));
                    }
                    cmd.spawn().map_err(Error::Command)
                })
                .transpose()
        }?;

        let uri = addr.parse::<Uri>().map_err(Error::InvalidUri)?;

        let client = HandlerClient::connect(uri).await.map_err(Error::Connect)?;

        let new = Self {
            args,
            program,
            client,
        };

        Ok(new)
    }

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
    pub fn reload(&self) -> Result<(), Error> {
        let pid = match &self.program {
            None => return Ok(()),
            // TODO: Casting can and probably will cause issues here
            // Not sure how to mitigate that
            Some(prog) => Pid::from_raw(prog.id() as i32),
        };

        send_signal(pid, Signal::SIGHUP).map_err(Error::Signal)
    }

    pub async fn handle(&mut self, config: Config, form: Form) -> Result<Status<Bytes>, Error> {
        let req = (config, form).into_rpc().map_err(Error::Convert)?;

        self.client
            .handle_rpc(req)
            .await
            .map(|res| Status::<Bytes>::from_rpc(res.into_inner()).map_err(Error::Convert))
            .map_err(Error::RPC)?
    }

    pub async fn validate(&mut self, config: Config) -> Result<Status<Bytes>, Error> {
        let req = config.into_rpc().map_err(Error::Convert)?;

        self.client
            .validate_rpc(req)
            .await
            .map(|res| Status::<Bytes>::from_rpc(res.into_inner()).map_err(Error::Convert))
            .map_err(Error::RPC)?
    }
}
