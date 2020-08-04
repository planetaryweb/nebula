#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub mod client;
pub mod config;
pub mod convert;
pub mod server;

pub mod rpc {
    tonic::include_proto!("nebula");
}

pub type Response = tonic::Response<rpc::Status>;
pub type Result = std::result::Result<Response, tonic::Status>;

pub use crate::client::{Client, ClientArgs};
pub use crate::config::{Config, Value as ConfigValue};
pub use crate::convert::{FromRPC, IntoRPC};
pub use crate::server::Handler;
