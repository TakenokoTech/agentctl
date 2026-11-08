#!/usr/bin/env bash
# 層間の依存ルール違反を検出する。ルールは SKILL.md を参照。
# 実体は tests/architecture.rs。`syn` で use と式の中のパスを解析する。
set -euo pipefail

cd "$(dirname "$0")/../.."

if output=$(cargo test --test architecture --quiet 2>&1); then
  echo "OK: 依存ルール違反なし"
else
  echo "$output"
  echo "違反: 層の依存ルールを守っていない(上の出力を参照)" >&2
  exit 1
fi
