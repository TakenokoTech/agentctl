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
