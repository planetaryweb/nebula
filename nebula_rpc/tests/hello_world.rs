use std::str::FromStr;
use bytes::Bytes;
use http::header::{self, HeaderValue};
use nebula_form::{Form, Field};
use nebula_rpc::client::Client;
use nebula_rpc::config::{ConfigExt, Config, Value as ConfigValue};
use nebula_rpc::server::Handler;
use nebula_status::{Status, StatusCode};
use tonic::async_trait;

mod utils {
    use super::*;
    use nebula_rpc::server;
    use std::net::{SocketAddr, IpAddr, Ipv4Addr};

    pub const NAME: &str = "TesterMan";
    pub const ADDR: &str = "http://127.0.0.1:1324";

    pub async fn delay() {
        tokio::time::delay_for(tokio::time::Duration::from_millis(100)).await;
    }

    pub fn get_addr() -> std::net::SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1324)
    }

    pub async fn get_timed_server() -> tokio::task::JoinHandle<Result<(), tonic::transport::Error>> {
        let addr = get_addr();
        tokio::spawn(async move {
            server::start::<HelloWorldServer>(addr).await
        })
    }

    pub async fn get_client() -> Client {
        Client::new(ADDR.to_string(), vec![]).await
            .expect("failed to create client")
    }

    pub fn get_valid_config() -> Config {
        let mut config = Config::new();
        config.insert(HelloWorldServer::CONFIG_FIELD_FAIL.to_string(), ConfigValue::LeafSingle("false".to_string()));
        config
    }

    pub fn get_invalid_config() -> Config {
        let mut config = Config::new();
        config.insert(HelloWorldServer::CONFIG_FIELD_FAIL.to_string(), ConfigValue::LeafSingle("true".to_string()));
        config
    }

    pub fn get_valid_form() -> Form {
        let mut form = Form::new();
        form.insert(HelloWorldServer::FORM_FIELD_NAME, Field::Text(NAME.to_string()));
        form
    }
}

#[derive(Default)]
struct HelloWorldServer {}

impl HelloWorldServer {
    const CONFIG_FIELD_FAIL: &'static str = "fail";
    const FORM_FIELD_NAME: &'static str = "name";
}

#[async_trait]
impl Handler for HelloWorldServer {
    async fn handle(&self, config: Config, form: Form) -> Status<Bytes> {
        let name = match form.get(Self::FORM_FIELD_NAME) {
            Some(name) => name.as_text(),
            None => return Status::with_data(
                StatusCode::BAD_REQUEST,
                format!("{{ \"field\": \"{}\" }}", Self::FORM_FIELD_NAME).into()
            ),
        };

        let name = match name {
            Some(name) => name,
            None => return Status::with_data(
                StatusCode::BAD_REQUEST,
                format!("{{ \"field\": \"{}\" }}", Self::FORM_FIELD_NAME).into()
            ),
        };

        let mut status = Status::with_data(StatusCode::OK, name.to_string().into());
        // Give text content-type so `Status::message()` works
        status.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        status
    }

    async fn validate(&self, config: Config) -> Status<Bytes> {
        let should_fail = match config.get_path(Self::CONFIG_FIELD_FAIL) {
            Ok(val) => match val {
                Some(val) => match val {
                    ConfigValue::LeafSingle(txt) => FromStr::from_str(&txt),
                    ConfigValue::LeafList(_) => return Status::with_data(StatusCode::BAD_REQUEST, Bytes::new()),
                    ConfigValue::Node(_) => return Status::with_data(StatusCode::BAD_REQUEST, Bytes::new()),
                },
                None => return Status::with_data(
                    StatusCode::BAD_REQUEST, "missing config field".to_string().into()
                ),
            },
            Err(err) => return Status::with_data(StatusCode::BAD_REQUEST, err.to_string().into()),
        }.expect("str to String should be Infallible");

        if should_fail {
            Status::with_data(StatusCode::BAD_REQUEST, "requested failure".to_string().into())
        } else {
            Status::with_data(StatusCode::OK, Bytes::new())
        }
    }
}

#[cfg(feature = "test-ports")]
#[tokio::test]
async fn test_connection_validate() {
    let server = utils::get_timed_server().await;
    utils::delay().await;
    
    let mut client = utils::get_client().await;
    let config = utils::get_valid_config();

    let status = client.validate(config).await
        .expect("validate operation should not error");

    utils::delay().await;

    if status.code().is_client_error() {
        panic!("unexpected client error: {:?}", status.message());
    } else if status.code().is_server_error() {
        panic!("unexpected server error: {:?}", status.message());
    }
}

#[cfg(feature = "test-ports")]
#[tokio::test]
async fn test_connection_validate_failure() {
    let server = utils::get_timed_server().await;
    utils::delay().await;
    
    let mut client = utils::get_client().await;
    let config = utils::get_invalid_config();

    let status = client.validate(config).await
        .expect("validate operation should not error");

    utils::delay().await;

    if status.code().is_server_error() {
        panic!("unexpected server error: {:?}", status.message());
    }

    assert!(status.code().is_client_error());
}

#[cfg(feature = "test-ports")]
#[tokio::test]
async fn test_connection_handle() {
    let server = utils::get_timed_server().await;
    utils::delay().await;

    let mut client = utils::get_client().await;
    let config = utils::get_valid_config();
    let form = utils::get_valid_form();

    let status = client.handle(config, form).await
        .expect("handle operation should not error");

    utils::delay().await;

    if status.code().is_client_error() {
        panic!("unexpected client error: {:?}", status.message());
    } else if status.code().is_server_error() {
        panic!("unexpected server error: {:?}", status.message());
    }

    assert_eq!(status.message(), Some(utils::NAME))
}
