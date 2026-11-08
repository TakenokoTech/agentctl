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
