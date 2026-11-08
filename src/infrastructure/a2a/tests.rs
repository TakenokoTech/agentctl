use super::*;

#[test]
fn no_token_means_no_authorization_header() {
    assert!(auth_headers(None).unwrap().is_empty());
}

#[test]
fn token_becomes_a_bearer_authorization_header() {
    let headers = auth_headers(Some("s3cret".into())).unwrap();
    assert_eq!(headers[AUTHORIZATION], "Bearer s3cret");
}

#[test]
fn token_that_is_not_a_valid_header_value_is_an_error() {
    assert!(auth_headers(Some("line1\nline2".into())).is_err());
}

#[tokio::test]
async fn watch_reports_a_connect_timeout_with_the_flag_hint() {
    // 応答が返らない宛先(TEST-NET-1)への接続は、制限時間で打ち切られる。
    // 経路がなく即座に失敗する環境では、確かめられないので何もしない
    let agent = A2aAgent {
        url: "http://192.0.2.1:9".into(),
        description: None,
        token_env: None,
        allow_insecure: false,
    };
    let session = A2aRustConnector::new(Some(Duration::from_millis(300)))
        .connect(&agent)
        .unwrap();
    let started = std::time::Instant::now();
    let err = match session.watch("t").await {
        Ok(_) => panic!("接続できてしまった"),
        Err(e) => e,
    };
    if started.elapsed() < Duration::from_millis(250) {
        eprintln!("skipped: 接続が即座に失敗した: {err:#}");
        return;
    }
    let msg = format!("{err:#}");
    assert!(
        msg.contains("timed out after 300ms (change it with --timeout)"),
        "{msg}"
    );
}

#[test]
fn authorization_header_is_marked_sensitive() {
    let headers = auth_headers(Some("s3cret".into())).unwrap();
    assert!(headers[AUTHORIZATION].is_sensitive());
    assert!(!format!("{headers:?}").contains("s3cret"));
}

fn interface(url: &str, binding: &str) -> AgentInterface {
    AgentInterface {
        url: url.into(),
        protocol_binding: binding.into(),
        tenant: None,
        protocol_version: "1.0".into(),
    }
}

#[test]
fn endpoints_on_the_configured_origin_are_accepted() {
    let interfaces = [
        interface("/", "HTTP+JSON"),
        interface("rpc", "JSONRPC"),
        interface("https://agent.example.com:443/v1", "jsonrpc"),
    ];
    assert!(ensure_same_origin("https://agent.example.com/base", &interfaces).is_ok());
}

#[test]
fn an_endpoint_on_another_origin_is_rejected() {
    for url in [
        "https://evil.example.net/",
        "http://agent.example.com/",
        "https://agent.example.com:8443/",
    ] {
        let err = ensure_same_origin("https://agent.example.com", &[interface(url, "JSONRPC")])
            .unwrap_err()
            .to_string();
        assert!(err.contains("different origin"), "{url}: {err}");
    }
}

#[test]
fn bindings_the_client_does_not_use_are_not_checked() {
    let interfaces = [interface("https://other.example.net", "GRPC")];
    assert!(ensure_same_origin("https://agent.example.com", &interfaces).is_ok());
}

#[test]
fn redirects_stay_on_the_origin_of_the_configured_url() {
    let base = reqwest::Url::parse("https://agent.example.com:8443/base").unwrap();
    let ok = |u: &str| redirect_allowed(&reqwest::Url::parse(u).unwrap(), &base);
    assert!(ok("https://agent.example.com:8443/elsewhere"));
    assert!(!ok("http://agent.example.com:8443/"), "scheme change");
    assert!(!ok("https://agent.example.com:9443/"), "port change");
    assert!(!ok("https://evil.example.net:8443/"), "host change");
}
