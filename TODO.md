# TODO

優先度の高い順。

1. [ ] 未コミットの変更をコミットする。分けるなら「CLI 実装」「テスト」「`AGENTS.md` 類(`src/`、`tests/` のものを含む)」の3つ
2. [ ] MCP の Streamable HTTP トランスポートの E2E を足す。`infrastructure/mcp.rs` の `McpTransport::Http`(接続と `token_env` の Bearer 送信)が唯一、本番コードで未テスト。`tests/fixtures/` に HTTP の MCP テストサーバを置く(`rmcp` の dev-dependency に HTTP サーバ用の feature が必要)
3. [ ] カバレッジの計測スクリプトをリポジトリに入れる。`cargo llvm-cov show-env` のフローでは example がビルドされないため、計測専用のターゲットディレクトリを手動で指定する必要がある
4. [ ] `CLAUDE.md` を作り、`@AGENTS.md` で参照する。Claude Code がルートと `src/`、`tests/` の `AGENTS.md` を読むかは未確認
5. [ ] clippy の `mod_module_files` を有効にするか決める。`mod.rs` を禁止し、`<名前>.rs` 形式に強制する lint
6. [ ] `infrastructure/a2a.rs`(本番 203 行)を、接続部と変換部に分けるか決める
