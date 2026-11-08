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
