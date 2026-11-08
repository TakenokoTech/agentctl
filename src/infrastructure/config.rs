use std::path::Path;

use anyhow::{Context, Result};

use crate::domain::config::Config;

pub fn load(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read config {}", path.display()))?;
    parse(&text).with_context(|| format!("invalid config {}", path.display()))
}

fn parse(text: &str) -> Result<Config> {
    Ok(toml::from_str(text)?)
}

/// 環境変数名から認証トークンを読む。名前が未指定なら `None`。
pub fn read_token(env: &Option<String>) -> Result<Option<String>> {
    read_token_with(env, |k| std::env::var(k).ok())
}

/// トークンを送るときは、https か loopback の http に限る。`allow_insecure` で解除できる。
pub fn ensure_secure_transport(url: &str, has_token: bool, allow_insecure: bool) -> Result<()> {
    if !has_token || allow_insecure {
        return Ok(());
    }
    let parsed = reqwest::Url::parse(url).with_context(|| format!("invalid url {url}"))?;
    let host = parsed.host_str().unwrap_or("");
    let loopback = host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if parsed.scheme() == "https" || (parsed.scheme() == "http" && loopback) {
        return Ok(());
    }
    anyhow::bail!(
        "refusing to send the token over an insecure connection to {url}; \
         use https, or set `allow_insecure = true` for this entry"
    )
}

fn read_token_with(
    env: &Option<String>,
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<Option<String>> {
    env.as_deref()
        .map(|k| lookup(k).with_context(|| format!("env var {k} is not set")))
        .transpose()
}

#[cfg(test)]
mod tests;
