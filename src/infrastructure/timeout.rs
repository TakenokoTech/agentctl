//! 接続とリクエストの制限時間。
use std::{future::Future, time::Duration};

use anyhow::Result;

/// 制限時間を、`500ms` や `5s` のように短く表す。
pub fn describe(limit: Duration) -> String {
    if limit.as_millis() < 1000 {
        format!("{}ms", limit.as_millis())
    } else {
        format!("{}s", limit.as_secs())
    }
}

/// `limit` を超えたら、`what` を示すエラーにする。`None` なら制限しない。
pub async fn within<T>(
    limit: Option<Duration>,
    what: &str,
    future: impl Future<Output = Result<T>>,
) -> Result<T> {
    let Some(limit) = limit else {
        return future.await;
    };
    match tokio::time::timeout(limit, future).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "timed out after {} {what} (change it with --timeout)",
            describe(limit)
        ),
    }
}

#[cfg(test)]
mod tests;
