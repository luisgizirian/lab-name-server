use anyhow::{Context, Result};
use clap::Parser;
use tokio::net::UdpSocket;
use tracing::{debug, error, info, warn};
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};

use lab_name_server::{Config, ZoneStore};

#[derive(Parser, Debug)]
#[command(name = "lab-name-server", version, about = "Simple authoritative+forwarding DNS server for labs")]
struct Args {
    /// Path to YAML config file
    #[arg(long, default_value = "config.yaml")]
    config: String,

    /// Override listen host (takes precedence over config)
    #[arg(long)]
    host: Option<String>,

    /// Override listen port (takes precedence over config)
    #[arg(long)]
    port: Option<u16>,

    /// Log level (ERROR, WARN, INFO, DEBUG, TRACE)
    #[arg(long, default_value = "INFO")]
    log: String,
}

use lab_name_server::forward_udp;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // init logging
    let filter = tracing_subscriber::EnvFilter::try_new(args.log.clone()).unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("INFO"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cfg_text = std::fs::read_to_string(&args.config).with_context(|| format!("reading config {}", args.config))?;
    let cfg: Config = serde_yaml::from_str(&cfg_text).context("parsing YAML config")?;

    let listen_host = args.host.or_else(|| cfg.listen.as_ref().map(|l| l.host.clone())).unwrap_or_else(|| "0.0.0.0".to_string());
    let listen_port = args.port.or_else(|| cfg.listen.as_ref().map(|l| l.port)).unwrap_or(5353);

    let upstreams = cfg.upstream.clone().unwrap_or_default();

    let store = ZoneStore::from_config(&cfg)?;

    let sock = std::sync::Arc::new(UdpSocket::bind(format!("{}:{}", listen_host, listen_port)).await?);
    info!("listening on {}:{}", listen_host, listen_port);

    let store = std::sync::Arc::new(store);
    let upstreams = std::sync::Arc::new(upstreams);

    loop {
        let mut buf = vec![0u8; 4096];
        let (len, peer) = match sock.recv_from(&mut buf).await { Ok(v) => v, Err(e) => { error!("recv_from error: {}", e); continue; } };
        let data = buf[..len].to_vec();
        let sock = sock.clone();
        let store = store.clone();
        let upstreams = upstreams.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_packet(sock, data, peer, store, upstreams).await { warn!("error handling packet from {}: {:#}", peer, e); }
        });
    }
}

async fn handle_packet(sock: std::sync::Arc<UdpSocket>, data: Vec<u8>, peer: std::net::SocketAddr, store: std::sync::Arc<ZoneStore>, upstreams: std::sync::Arc<Vec<String>>) -> Result<()> {
    // Try parse
    let req = match Message::from_vec(&data) { Ok(m) => m, Err(_) => { debug!("invalid DNS message from {}", peer); return Ok(()); } };

    if let Some(mut resp) = store.answer(&req) {
        resp.set_id(req.id());
        let out = resp.to_vec()?;
        sock.send_to(&out, &peer).await?;
        return Ok(());
    }

    // Forward to upstreams if configured
    if upstreams.is_empty() {
        // reply with SERVFAIL
        let mut r = Message::new();
        r.set_id(req.id());
        r.set_message_type(MessageType::Response);
        r.set_op_code(OpCode::Query);
        if let Some(q) = req.queries().first() { r.add_query(q.clone()); }
        r.set_response_code(ResponseCode::ServFail);
        let out = r.to_vec()?;
        sock.send_to(&out, &peer).await?;
        return Ok(());
    }

    // try each upstream
    for up in upstreams.iter() {
        let up_addr = format!("{}:53", up);
        match forward_udp(&up_addr, &data).await {
            Ok(resp) => {
                let _ = sock.send_to(&resp, &peer).await?;
                return Ok(());
            }
            Err(e) => {
                warn!("upstream {} failed: {}", up_addr, e);
                continue;
            }
        }
    }

    // all upstreams failed
    let mut r = Message::new();
    r.set_id(req.id());
    r.set_message_type(MessageType::Response);
    r.set_op_code(OpCode::Query);
    if let Some(q) = req.queries().first() { r.add_query(q.clone()); }
    r.set_response_code(ResponseCode::ServFail);
    let out = r.to_vec()?;
    sock.send_to(&out, &peer).await?;
    Ok(())
}

// tests were moved to the integration tests in `tests/`
