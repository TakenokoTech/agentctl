# TODO

優先度の高い順。

1. [x] 未コミットの変更をコミットする(`0ccd06c`、`3d3cea0`)
2. [x] MCP の Streamable HTTP トランスポートの E2E を足す(`tests/e2e/mcp_http.rs`、`tests/fixtures/mcp_http_server.rs`)
3. [x] カバレッジの計測スクリプトをリポジトリに入れる(`.agent/coverage.sh`)
4. [x] `CLAUDE.md` は作らない。`CLAUDE.md` が無ければ、Claude Code(v2.1.277 以降。Amazon Bedrock のセッションやテレメトリを無効にしたセッションでは v2.1.281 以降)はルートの `AGENTS.md` を起動時に、サブディレクトリの `AGENTS.md` をそのディレクトリのファイルを Read したときに自動で読む。`CLAUDE.md` を1つでも作ると `AGENTS.md` は読まれなくなる。読み込みの確認は `/memory`
5. [ ] clippy の `mod_module_files` を有効にするか決める。`mod.rs` を禁止し、`<名前>.rs` 形式に強制する lint
6. [ ] `infrastructure/a2a.rs`(本番 203 行)を、接続部と変換部に分けるか決める
7. [x] README の `git clone <this repository>` を、実際のリポジトリ URL に置き換える
8. [ ] ライセンスを決め、`LICENSE` ファイルと README の記載を足す(現在は `LICENSE` が無い)
9. [x] `README.md` と `README.ja.md` を同期させる運用を決める(`docs/AGENTS.md` に規約として記載)
10. [x] README の Rust の最小バージョンを確認する(依存クレートの要求は最大で uuid の 1.89。1.89.0 でビルドできる)
11. [ ] インストールを楽にする。リポジトリの公開先と URL が決まったら、まず `cargo install --git <URL>` の1行に差し替える(7番に依存)。`npx` のようにインストール不要で使いたい場合は、`cargo-dist`(現 `dist`)の npm インストーラで、ビルド済みバイナリをダウンロードする npm パッケージを公開する(`npx -y -p <パッケージ> agentctl-mcp` の形になる見込みで、複数バイナリでの書式は未検証)。そのほかの案は Docker イメージ、PyPI(`uvx`)。いずれも GitHub のリポジトリ、リリースの CI、ライセンス(8番)が前提。crates.io への公開は、ライセンスと名前を決めてから最後に行う。判断材料は、利用者に Rust 環境を求めてよいか。決めるときは `.agent/decision/SKILL.md` の手順で ADR に残す

## `/code-review` の指摘

サブエージェントによる指摘。「再現済み」は実際に動かして確認したもの、「未検証」はコードの読みからの指摘。

12. [ ] (再現済み)MCP の設定で未知のキーが黙って無視される。`McpServer` は `flatten` を使うため `deny_unknown_fields` が効かず、`token_en` のようなタイポが無視されて 401 になる
13. [ ] (再現済み)`a2a send "-1 …"` のように `-` で始まる text が、clap にフラグと解釈される。`--` を付ければ回避できる。`allow_hyphen_values` か、標準入力から読む手段を検討する
14. [ ] (再現済み)`agentctl … | head` のようにパイプが切れると、`println!` が panic して exit 101 になる。標準出力への書き込みエラーを処理し、静かに終了する
15. [ ] (未検証)環境変数が空文字のとき、空の Bearer トークンとして送ってしまう。「未設定」と同じエラーにする
16. [ ] (未検証)タイムアウトが無い。応答しない A2A / MCP サーバーで CLI が止まり続ける。`task watch` は全体のタイムアウトから除外する
17. [ ] (未検証)`list` / `tools` の TSV で、説明に含まれるタブや改行をエスケープしていない
18. [ ] (未検証)終端状態に達する前にストリームが閉じても、`task watch` は exit 0 になる。`canceled` も成功扱い
19. [ ] (未検証)`state_info` は、未知の状態名だと空文字になり、失敗の判定を取りこぼす
20. [ ] (設計判断)stdio サーバーの標準エラーを捨てているため、起動に失敗した原因が見えない。オプトインで通す手段を検討する
21. [ ] (未検証)サーバーが終端状態のあともストリームを閉じないと、`task watch` が戻らない。終端状態で止めることを検討する

## 機能追加

22. [x] MCP クライアントから使えるようにする。同じパッケージの2つ目のバイナリ `agentctl-mcp`(`src/bin/agentctl-mcp/`)が、ツール呼び出しごとに `agentctl` をサブプロセスで実行する。`cargo install --path .` で `agentctl` と一緒に入る。公開するツールは `list` / `mcp_tools` / `mcp_schema` / `mcp_call` / `a2a_card` / `a2a_send` / `a2a_task_get` / `a2a_task_cancel`。`a2a task watch` は公開しない。stdio のみで、Streamable HTTP は未対応
23. [ ] `agentctl-mcp` の内部構成を決める。現在は平らな4モジュール(`command` / `runner` / `server` / `tools`)で、層構成の対象外としている。候補と点数は ADR 0003(`docs/adr/0003-mcp-wrapper-internal-structure.md`、Proposed。推奨は「C から入って D を目指す」)。決めたら、ADR を `Accepted` にし、`AGENTS.md` と `.agent/architecture/SKILL.md` の「対象外」の記述を直し、必要なら `check.sh` を `src/bin/agentctl-mcp/` に広げる
24. [ ] `agentctl-mcp` を同じパッケージに置いたことで、`agentctl` 本体のコンパイル時間がどれだけ増えるか(`rmcp` のサーバー機能を一緒にビルドする)を計測する。独立して公開する必要が出たら、`crates/agentctl-mcp` への切り出しを検討する(ADR 0002 の「見直す条件」)
25. [ ] 意思決定の手順(`.agent/decision/SKILL.md`)を実際の設計判断で使い、手順の足りない点を直す。最初の対象は ADR 0003
26. [ ] `agentctl-mcp` が、`PATH` や `AGENTCTL_BIN` より先に、自分と同じ場所にある `agentctl` を探すようにする。配布物(npm パッケージなど)に両方のバイナリが同梱されても、設定なしで動く。実装前に、探す順序(同じ場所 → `AGENTCTL_BIN` → `PATH` のどれを優先するか)を決める
