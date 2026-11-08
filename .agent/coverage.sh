#!/usr/bin/env bash
# カバレッジを計測する。テストコード(tests.rs と tests/)は集計から除く。
#
# 事前に必要なもの:
#   rustup component add llvm-tools-preview
#   cargo install cargo-llvm-cov --locked
#
# `cargo llvm-cov` を直接使わない理由:
#   E2E テストが MCP サーバの example バイナリを起動するが、`cargo llvm-cov` は
#   example を通常のバイナリとしてビルドしない。そのため、計測用の環境変数だけを
#   取り出し、専用のターゲットディレクトリでビルドとテストを実行している。
#
# 使い方: .agent/coverage.sh [cargo llvm-cov report に渡す引数]
#   例: .agent/coverage.sh --show-missing-lines
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
T=$PWD/target/llvm-cov-target
rm -rf "$T"

eval "$(cargo llvm-cov show-env --sh 2>/dev/null)"
export CARGO_TARGET_DIR=$T CARGO_LLVM_COV_TARGET_DIR=$T CARGO_LLVM_COV_BUILD_DIR=$T
export LLVM_PROFILE_FILE="$T/agentctl-%p-%8m.profraw"

cargo build --examples --quiet
cargo test --quiet

cargo llvm-cov report --ignore-filename-regex '(/tests\.rs$|/tests/)' "${@:---summary-only}"
