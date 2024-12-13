
use async_trait::async_trait;
use clap::Parser;
use serde::Deserialize;
use pingora_core::services::background::background_service;
use pingora_core::server::configuration::Opt;
use pingora_core::server::Server;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_core::{Result, Error, ErrorType};
use pingora_load_balancing::{health_check, selection::consistent::KetamaHashing, LoadBalancer};
use pingora_proxy::{ProxyHttp, Session};

use std::{
    env,
    process,
    fs::File,
    io::Read,
    sync::Arc,
    time::Duration
};


pub struct LB(Arc<LoadBalancer<KetamaHashing>>, Config);

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

    env_logger::init();

    let opt = Opt::parse();

    let exe_path = env::current_exe().expect("无法获取可执行文件路径");
    let current_dir = exe_path.parent().expect("无法获取可执行文件所在目录").to_path_buf();

    let config_path = current_dir.join("config.toml");
    let mut file = File::open(&config_path).unwrap_or_else(|err| {
        eprintln!("无法打开“config.toml”文件: {err}");
        process::exit(1);
    });

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap_or_else(|err| {
        eprintln!("无法读取文件: {err}");
        process::exit(1);
    });

    let config: Config = toml::from_str(&contents).unwrap_or_else(|err| {
        eprintln!("无法解析配置文件: {err}");
        process::exit(1);
    });

    let mut my_server = Server::new(Some(opt)).unwrap();
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
