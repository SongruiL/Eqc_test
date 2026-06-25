//! 一次性连通性 spike（前端 LLM Agent arc）：确认这台无管理员的中国网络机器上，
//! Rust 的 ureq 能否真的连到 api.anthropic.com（TLS + 代理）。默认 `#[ignore]`，手动跑：
//!   cargo test --features cli --test llm_spike -- --ignored --nocapture
//! 无需真 key：dummy key 返回 HTTP 401 = TLS/网络通；传输错误 = 连不上。
//! 设了 ANTHROPIC_API_KEY 则用真 key（200 = 完全打通）。
#![cfg(feature = "cli")]

fn try_post(agent: &ureq::Agent, label: &str) {
    let key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "dummy-no-key".to_string());
    let body = r#"{"model":"claude-opus-4-8","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#;
    let r = agent
        .post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", &key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .timeout(std::time::Duration::from_secs(25))
        .send_string(body);
    match r {
        Ok(resp) => println!("[{label}] OK status={} (网络+TLS 通)", resp.status()),
        Err(ureq::Error::Status(code, _)) => {
            println!("[{label}] HTTP {code} (可达；TLS+网络通——401=key 无效但连得上)")
        }
        Err(ureq::Error::Transport(t)) => println!("[{label}] 传输失败（连不上/TLS 失败）: {t}"),
    }
}

#[test]
#[ignore]
fn spike_reachability() {
    // (a) 直连（ureq 默认不读 env 代理，会真·直连）
    try_post(&ureq::AgentBuilder::new().build(), "direct");

    // (b) 走本地代理 127.0.0.1:10808（PowerShell 实测可达的那条路）
    match ureq::Proxy::new("http://127.0.0.1:10808") {
        Ok(p) => try_post(&ureq::AgentBuilder::new().proxy(p).build(), "proxy-10808"),
        Err(e) => println!("[proxy-10808] 代理构造失败: {e}"),
    }
}
