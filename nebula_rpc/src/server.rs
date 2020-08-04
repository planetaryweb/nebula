use bytes::Bytes;
use crate::config::Config;
use crate::convert::{FromRPC, IntoRPC};
use crate::rpc;
use crate::{Response, Result as RPCResult};
use nebula_form::Form;
use nebula_status::Status;
use tonic::async_trait;
use tonic::transport::Server;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
	}
}

pub async fn start<T>(addr: std::net::SocketAddr) -> Result<(), tonic::transport::Error>
    where T: Handler + Default {
    let handler = T::default();
    Server::builder()
        .add_service(rpc::handler_server::HandlerServer::new(handler))
        .serve(addr)
        .await?;

    Ok(())
}

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn handle(&self, config: Config, form: Form) -> Status<Bytes>;
    async fn validate(&self, config: Config) -> Status<Bytes>;
}

#[async_trait]
impl<T> rpc::handler_server::Handler for T where T: Handler {
    async fn handle_rpc(&self, req: tonic::Request<rpc::HandleRequest>) -> RPCResult {
        let req = req.into_inner();
        let (config, form) = FromRPC::from_rpc(req)
            .map_err(|err| tonic::Status::new(tonic::Code::InvalidArgument, err))?;
        let status = self.handle(config, form).await.into_rpc()
            .map_err(|err| tonic::Status::new(tonic::Code::Internal, err))?;
        let response = Response::new(status);
        Ok(response)
    }

    async fn validate_rpc(&self, req: tonic::Request<rpc::Config>) -> RPCResult {
        let config = req.into_inner();
        let config = FromRPC::from_rpc(config)
            .map_err(|err| tonic::Status::new(tonic::Code::InvalidArgument, err))?;
        let status = self.validate(config).await.into_rpc()
            .map_err(|err| tonic::Status::new(tonic::Code::Internal, err))?;
        let response = Response::new(status);
        Ok(response)
    }
}
