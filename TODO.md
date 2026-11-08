# TODO

優先度の高い順。

1. [x] 未コミットの変更をコミットする(`0ccd06c`、`3d3cea0`)
2. [ ] MCP の Streamable HTTP トランスポートの E2E を足す。`infrastructure/mcp.rs` の `McpTransport::Http`(接続と `token_env` の Bearer 送信)が唯一、本番コードで未テスト。`tests/fixtures/` に HTTP の MCP テストサーバを置く(`rmcp` の dev-dependency に HTTP サーバ用の feature が必要)
3. [ ] カバレッジの計測スクリプトをリポジトリに入れる。`cargo llvm-cov show-env` のフローでは example がビルドされないため、計測専用のターゲットディレクトリを手動で指定する必要がある
4. [ ] `CLAUDE.md` を作り、`@AGENTS.md` で参照する。Claude Code がルートと `src/`、`tests/` の `AGENTS.md` を読むかは未確認
5. [ ] clippy の `mod_module_files` を有効にするか決める。`mod.rs` を禁止し、`<名前>.rs` 形式に強制する lint
6. [ ] `infrastructure/a2a.rs`(本番 203 行)を、接続部と変換部に分けるか決める
7. [ ] README の `git clone <this repository>` を、実際のリポジトリ URL に置き換える
8. [ ] ライセンスを決め、`LICENSE` ファイルと README の記載を足す(現在は `LICENSE` が無い)
9. [x] `README.md` と `README.ja.md` を同期させる運用を決める(`docs/AGENTS.md` に規約として記載)
10. [ ] README の「Rust 1.85 以降」を、実際に 1.85 でビルドして確認する
11. [ ] インストールを楽にする。リポジトリの公開先と URL が決まったら、まず `cargo install --git <URL>` の1行に差し替える(7番に依存)。配布先を広げるなら `cargo-dist` でビルド済みバイナリとインストーラを配る。crates.io への公開は、ライセンス(8番)と名前を決めてから最後に行う。判断材料は、利用者に Rust 環境を求めてよいか
