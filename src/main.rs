use async_trait::async_trait;
use clap::Parser;
use pingora_core::services::background::background_service;
use serde::Deserialize;
use pingora_core::server::Server;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::{Result, Error, ErrorType};
use pingora_load_balancing::{health_check, selection::consistent::KetamaHashing, LoadBalancer};
use pingora_proxy::{ProxyHttp, Session};

use std::{
    fs::File,
    io::Read,
    sync::Arc,
    time::Duration
};

pub struct LB(Arc<LoadBalancer<KetamaHashing>>, Config);

#[derive(Parser)]
#[command(name = "steamp")]
#[command(version = "0.1.4")]
#[command(about = "反向代理工具", long_about = None)]
struct Cli {
    #[arg(short, long, required = true)]
    config_path: String
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    https: String,
    sni: String,
    cert: String,
    key: String,
    backends: Vec<String>,
}

#[async_trait]
impl ProxyHttp for LB {
    type CTX = ();
    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
        let upstream = match self
            .0
            .select(b"", 256) { 
                Some(upstream) => upstream,
                None => {
                    return Err(Error::new(ErrorType::new("没有健康的后端可以选择.")))
                }
            };
        println!("选择的后端: {:?}", upstream);

        let mut peer = Box::new(HttpPeer::new(upstream, true, self.1.sni.clone()));
        peer.options.verify_cert = false;
        peer.options.verify_hostname = false;
        Ok(peer)
    }

    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut pingora_http::RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        if let Some(host) = session.req_header().uri.host() {
            upstream_request.insert_header("Host", host.to_string()).unwrap();
        }
        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();

    let config = open_config(&cli.config_path).expect("无法加载配置文件");

    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();
    let backends = config.backends.clone();
    let mut upstreams = LoadBalancer::try_from_iter(backends).unwrap();
    let hc = health_check::TcpHealthCheck::new();
    upstreams.set_health_check(hc);
    upstreams.health_check_frequency = Some(Duration::from_secs(1));
    let background = background_service("health check", upstreams);
    let upstreams = background.task();

    let mut lb = pingora_proxy::http_proxy_service(&my_server.configuration, LB(upstreams, config.clone()));
    let mut tls_settings = pingora_core::listeners::TlsSettings::intermediate(&config.cert, &config.key).unwrap();
    tls_settings.enable_h2();
    lb.add_tls_with_settings(&config.https, None, tls_settings);

    println!("HTTPS监听地址: {}", config.https);
    println!("SNI: {}", config.sni);
    println!("cert: {}", config.cert);
    println!("key: {}", config.key);
    println!("backends: {:?}", config.backends);

    my_server.add_service(lb);
    my_server.add_service(background);
    my_server.run_forever();
}

fn open_config(config_path: &str) -> Result<Config> {
    let mut file = File::open(config_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let config: Config = toml::from_str(&contents).unwrap();
    Ok(config)
}