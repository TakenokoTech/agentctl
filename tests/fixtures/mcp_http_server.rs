//! E2E テスト用の MCP サーバ(Streamable HTTP)。
//! ツール定義は `mcp_handler.rs` と共有し、受け取った `Authorization` ヘッダを記録する。
#[path = "mcp_handler.rs"]
mod handler;

use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

pub struct HttpServer {
    /// MCP のエンドポイント(例: `http://127.0.0.1:12345/mcp`)
    pub url: String,
    /// 受け取った `Authorization` ヘッダの値(到着順)
    pub seen_auth: Arc<Mutex<Vec<String>>>,
}

async fn record_auth(
    State(seen): State<Arc<Mutex<Vec<String>>>>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(v) = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    {
        seen.lock().unwrap().push(v.to_owned());
    }
    next.run(req).await
}

/// ランダムポートでサーバを起動する。サーバはテストプロセスの終了まで動き続ける。
pub fn start() -> HttpServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let seen_auth = Arc::new(Mutex::new(Vec::new()));
    let seen = seen_auth.clone();
    std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let service = StreamableHttpService::new(
                    || Ok(handler::TestServer),
                    Arc::new(LocalSessionManager::default()),
                    StreamableHttpServerConfig::default(),
                );
                let app = Router::new()
                    .nest_service("/mcp", service)
                    .layer(middleware::from_fn_with_state(seen, record_auth));
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                axum::serve(listener, app).await.unwrap();
            })
    });
    HttpServer {
        url: format!("http://{addr}/mcp"),
        seen_auth,
    }
}
