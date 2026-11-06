//! External control: OSC over UDP and a small HTTP API (for Bitfocus Companion, Stream Deck…).
//!
//! OSC  : /trapdoor/go, /trapdoor/take <n>, /trapdoor/standby <n>, /trapdoor/next, /trapdoor/prev,
//!        /trapdoor/pause, /trapdoor/resume, /trapdoor/stop, /trapdoor/panic, /trapdoor/goto <sec>,
//!        /trapdoor/loop, /trapdoor/black [0|1], /trapdoor/bank <F1..>
//!        Feedback (sent to the last sender, port+1 by default): /trapdoor/state/remain, /program, /standby
//! HTTP : POST /api/<same names>?arg=…  ·  GET /api/state

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_lot::RwLock;
use rosc::{OscMessage, OscPacket, OscType};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::controller::Command;

/// Map a verb + optional argument to a command. Shared by OSC and HTTP.
pub fn parse(verb: &str, arg: Option<&str>) -> Result<Command, String> {
    let need = |a: Option<&str>| a.map(|s| s.to_string()).ok_or_else(|| format!("{verb} needs an argument"));
    Ok(match verb.trim_matches('/') {
        "go" => Command::Go,
        "take" => Command::TakeNumber { number: need(arg)? },
        "standby" => Command::StandbyNumber { number: need(arg)? },
        "next" => Command::StandbyNext,
        "prev" => Command::StandbyPrev,
        "pause" => Command::Pause,
        "resume" | "play" => Command::Resume,
        "togglepause" => Command::TogglePause,
        "stop" => Command::Stop,
        "panic" => Command::Panic,
        "goto" => Command::GotoRemain { seconds: need(arg)?.parse().map_err(|_| "goto needs seconds")? },
        "loop" => Command::ToggleLoop,
        "black" => match arg {
            Some("0") | Some("false") | Some("off") => Command::Black { on: false },
            Some(_) => Command::Black { on: true },
            None => Command::ToggleBlack,
        },
        "bank" => Command::Bank { slot: need(arg)? },
        other => return Err(format!("unknown command: {other}")),
    })
}

fn osc_arg(args: &[OscType]) -> Option<String> {
    args.first().map(|a| match a {
        OscType::Int(i) => i.to_string(),
        OscType::Float(f) => f.to_string(),
        OscType::Double(f) => f.to_string(),
        OscType::String(s) => s.clone(),
        OscType::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
        other => format!("{other:?}"),
    })
}

pub fn parse_osc(msg: &OscMessage) -> Result<Command, String> {
    let verb = msg.addr.strip_prefix("/trapdoor/").ok_or("address must start with /trapdoor/")?;
    parse(verb, osc_arg(&msg.args).as_deref())
}

#[derive(Clone)]
pub struct RemoteCtx {
    pub tx: mpsc::UnboundedSender<Command>,
    /// Latest playout state as JSON (written by the app loop).
    pub state: Arc<RwLock<serde_json::Value>>,
}

#[derive(Deserialize)]
struct ArgQuery {
    arg: Option<String>,
}

async fn http_cmd(State(ctx): State<RemoteCtx>, Path(verb): Path<String>, Query(q): Query<ArgQuery>) -> Json<serde_json::Value> {
    match parse(&verb, q.arg.as_deref()) {
        Ok(c) => {
            let _ = ctx.tx.send(c);
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

async fn http_state(State(ctx): State<RemoteCtx>) -> Json<serde_json::Value> {
    Json(ctx.state.read().clone())
}

pub fn router(ctx: RemoteCtx) -> Router {
    Router::new().route("/api/state", get(http_state)).route("/api/{verb}", post(http_cmd).get(http_cmd)).with_state(ctx)
}

/// Run OSC + HTTP servers until the runtime stops. Errors (port in use) are reported, not fatal.
pub async fn serve(ctx: RemoteCtx, osc_port: u16, http_port: u16, on_error: impl Fn(String) + Send + Sync + 'static) {
    let on_error = Arc::new(on_error);
    let osc_ctx = ctx.clone();
    let e1 = on_error.clone();
    let osc = tokio::spawn(async move {
        let sock = match tokio::net::UdpSocket::bind(("0.0.0.0", osc_port)).await {
            Ok(s) => s,
            Err(e) => return e1(format!("OSC port {osc_port}: {e}")),
        };
        let mut buf = vec![0u8; 65536];
        let mut last_peer: Option<SocketAddr> = None;
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            tokio::select! {
                r = sock.recv_from(&mut buf) => {
                    let Ok((n, peer)) = r else { continue };
                    last_peer = Some(peer);
                    if let Ok((_, pkt)) = rosc::decoder::decode_udp(&buf[..n]) {
                        for m in flatten(pkt) {
                            if let Ok(c) = parse_osc(&m) { let _ = osc_ctx.tx.send(c); }
                        }
                    }
                }
                _ = tick.tick() => {
                    if let Some(peer) = last_peer {
                        let st = osc_ctx.state.read().clone();
                        let fb = SocketAddr::new(peer.ip(), osc_port.wrapping_add(1));
                        for (addr, v) in [
                            ("/trapdoor/state/remain", st["program"]["remainTc"].as_str().unwrap_or("").to_string()),
                            ("/trapdoor/state/program", st["program"]["name"].as_str().unwrap_or("").to_string()),
                            ("/trapdoor/state/standby", st["standbyName"].as_str().unwrap_or("").to_string()),
                        ] {
                            if let Ok(bytes) = rosc::encoder::encode(&OscPacket::Message(OscMessage { addr: addr.into(), args: vec![OscType::String(v)] })) {
                                let _ = sock.send_to(&bytes, fb).await;
                            }
                        }
                    }
                }
            }
        }
    });
    let e2 = on_error.clone();
    let http = tokio::spawn(async move {
        match tokio::net::TcpListener::bind(("0.0.0.0", http_port)).await {
            Ok(l) => {
                if let Err(e) = axum::serve(l, router(ctx)).await {
                    e2(format!("HTTP: {e}"));
                }
            }
            Err(e) => e2(format!("HTTP port {http_port}: {e}")),
        }
    });
    let _ = tokio::join!(osc, http);
}

fn flatten(p: OscPacket) -> Vec<OscMessage> {
    match p {
        OscPacket::Message(m) => vec![m],
        OscPacket::Bundle(b) => b.content.into_iter().flat_map(flatten).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbs() {
        assert_eq!(parse("go", None).unwrap(), Command::Go);
        assert_eq!(parse("take", Some("3")).unwrap(), Command::TakeNumber { number: "3".into() });
        assert_eq!(parse("goto", Some("10")).unwrap(), Command::GotoRemain { seconds: 10.0 });
        assert_eq!(parse("black", Some("0")).unwrap(), Command::Black { on: false });
        assert!(parse("take", None).is_err());
        assert!(parse("explode", None).is_err());
        let m = OscMessage { addr: "/trapdoor/bank".into(), args: vec![OscType::String("F3".into())] };
        assert_eq!(parse_osc(&m).unwrap(), Command::Bank { slot: "F3".into() });
        let m = OscMessage { addr: "/trapdoor/take".into(), args: vec![OscType::Int(7)] };
        assert_eq!(parse_osc(&m).unwrap(), Command::TakeNumber { number: "7".into() });
    }

    #[tokio::test]
    async fn http_roundtrip() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let ctx = RemoteCtx { tx, state: Arc::new(RwLock::new(serde_json::json!({"black": false}))) };
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(l, router(ctx)).await.unwrap() });
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        s.write_all(b"POST /api/take?arg=4 HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        assert!(out.contains("\"ok\":true"), "{out}");
        assert_eq!(rx.recv().await.unwrap(), Command::TakeNumber { number: "4".into() });
    }
}
