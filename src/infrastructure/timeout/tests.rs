use std::{future::pending, time::Duration};

use anyhow::anyhow;

use super::*;

#[test]
fn short_limits_are_shown_in_milliseconds_and_long_ones_in_seconds() {
    assert_eq!(describe(Duration::from_millis(50)), "50ms");
    assert_eq!(describe(Duration::from_millis(999)), "999ms");
    assert_eq!(describe(Duration::from_secs(1)), "1s");
    assert_eq!(describe(Duration::from_secs(60)), "60s");
}

#[tokio::test]
async fn a_future_that_finishes_in_time_returns_its_value() {
    let out = within(Some(Duration::from_secs(5)), "doing it", async { Ok(7) }).await;
    assert_eq!(out.unwrap(), 7);
}

#[tokio::test]
async fn a_future_that_fails_in_time_returns_its_own_error() {
    let out: Result<()> = within(Some(Duration::from_secs(5)), "doing it", async {
        Err(anyhow!("nope"))
    })
    .await;
    assert_eq!(out.unwrap_err().to_string(), "nope");
}

#[tokio::test]
async fn a_future_that_does_not_finish_is_reported_with_what_and_how_to_change_the_limit() {
    let out: Result<()> = within(
        Some(Duration::from_millis(30)),
        "connecting to the MCP server",
        pending(),
    )
    .await;
    assert_eq!(
        out.unwrap_err().to_string(),
        "timed out after 30ms connecting to the MCP server (change it with --timeout)"
    );
}

#[tokio::test]
async fn no_limit_waits_for_the_future() {
    let out = within(None, "doing it", async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        Ok("done")
    })
    .await;
    assert_eq!(out.unwrap(), "done");
}
