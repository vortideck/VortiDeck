use anyhow::Result;
use clap::Parser;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::net::{TcpListener, UdpSocket};
use trust_dns_server::ServerFuture;
use trust_dns_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use trust_dns_server::proto::rr::{Record, RecordType, rdata::A};
use trust_dns_server::rr::Name;
use trust_dns_server::proto::op::ResponseCode;

/// Timeout for TCP connections.
const TCP_TIMEOUT: Duration = Duration::from_secs(10);

/// Create the DNS server bound to the provided IP and port.
pub async fn create_websocket_dns(ip: String, port: u16) -> Result<()> {
    tracing_subscriber::fmt::init();

    let udp_address = format!("{}:{}", ip, port);
    let tcp_address = format!("{}:{}", ip, port);

    let handler = Handler {};

    let mut server = ServerFuture::new(handler);

    let udp_socket = UdpSocket::bind(udp_address).await?;
    server.register_socket(udp_socket);

    let tcp_listener = TcpListener::bind(tcp_address).await?;
    server.register_listener(tcp_listener, TCP_TIMEOUT);

    server.block_until_done().await?;

    Ok(())
}

#[derive(Parser, Clone, Debug)]
pub struct Options {
    #[clap(long, short, default_value = "0.0.0.0", env = "DNSFUN_IP")]
    pub ip: String,
    #[clap(long, short, default_value = "1053", env = "DNSFUN_PORT")]
    pub port: u16,
    #[clap(long, short, default_value = "vortideck.local", env = "DNSFUN_DOMAIN")]
    pub domain: String,
    #[clap(long, default_value = "192.128.178.124", env = "DNSFUN_TARGET_IP")]
    pub target_ip: String,
}

#[derive(Clone, Debug)]
pub struct Handler {}

impl Handler {
    pub fn from_options(_options: &Options) -> Self {
        Handler {}
    }
}

#[async_trait::async_trait]
impl RequestHandler for Handler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response: R,
    ) -> ResponseInfo {
        let domain = "vortideck.local";
        let ip = Ipv4Addr::new(192, 128, 178, 124);

        for query in request.queries() {
            let query_name = query.name().to_string();
            if query_name == domain {
                let mut record = Record::with(Name::from_ascii(domain).unwrap(), RecordType::A, 3600);
                let rdata = A::new(ip);
                record.set_rdata(Some(rdata.into()));
                response.send_response(&[record], vec![], vec![], ResponseCode::NoError).await;
                break;
            }
        }

        response.send_response(&[], vec![], vec![], ResponseCode::NoError).await;

        ResponseInfo::default()
    }
}
