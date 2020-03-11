use crate::handlers::Handler;
use std::net::IpAddr;
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    logger: LoggerConfig,
    server: ServerConfig,
    handlers: Vec<Handler>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Logger {
    File{ file: String },
    Stdout,
    Syslog,
}

#[derive(Deserialize)]
struct Server {
    port: u32,
    ip_address: IpAddr,
}
