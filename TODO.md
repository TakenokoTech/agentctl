# TODO

優先度の高い順。

1. [x] 未コミットの変更をコミットする(`0ccd06c`、`3d3cea0`)
2. [x] MCP の Streamable HTTP トランスポートの E2E を足す(`tests/e2e/mcp_http.rs`、`tests/fixtures/mcp_http_server.rs`)
3. [x] カバレッジの計測スクリプトをリポジトリに入れる(`.agent/coverage.sh`)
4. [x] `CLAUDE.md` は作らない。`CLAUDE.md` が無ければ、Claude Code(v2.1.277 以降)はルートの `AGENTS.md` を起動時に、サブディレクトリの `AGENTS.md` をそのディレクトリのファイルを Read したときに自動で読む。`CLAUDE.md` を1つでも作ると `AGENTS.md` は読まれなくなる。読み込みの確認は `/memory`
5. [ ] clippy の `mod_module_files` を有効にするか決める。`mod.rs` を禁止し、`<名前>.rs` 形式に強制する lint
6. [x] `infrastructure/a2a.rs` を、接続部と変換部(`a2a/convert.rs`)に分けた。あわせて、`infrastructure/mcp.rs` から標準エラーの末尾を保持する部品(`mcp/stderr_tail.rs`)も出した(`/pre-commit-review` で、本番コード 200 行前後を超えた分の見直しを指摘されたため)
7. [ ] README の `git clone <this repository>` を、実際のリポジトリ URL に置き換える
8. [ ] ライセンスを決め、`LICENSE` ファイルと README の記載を足す(現在は `LICENSE` が無い)
9. [x] `README.md` と `README.ja.md` を同期させる運用を決める(`docs/AGENTS.md` に規約として記載)
10. [ ] README の「Rust 1.85 以降」を、実際に 1.85 でビルドして確認する
11. [ ] インストールを楽にする。リポジトリの公開先と URL が決まったら、まず `cargo install --git <URL>` の1行に差し替える(7番に依存)。`npx` のようにインストール不要で使いたい場合は、`cargo-dist`(現 `dist`)の npm インストーラで、ビルド済みバイナリをダウンロードする npm パッケージを公開する(`npx -y -p <パッケージ> agentctl-mcp` の形になる見込みで、複数バイナリでの書式は未検証)。そのほかの案は Docker イメージ、PyPI(`uvx`)。いずれも GitHub のリポジトリ、リリースの CI、ライセンス(8番)が前提。crates.io への公開は、ライセンスと名前を決めてから最後に行う。判断材料は、利用者に Rust 環境を求めてよいか。決めるときは `.agent/decision/SKILL.md` の手順で ADR に残す

## `/code-review` の指摘

サブエージェントによる指摘。「再現済み」は実際に動かして確認したもの、「未検証」はコードの読みからの指摘。

12. [x] (再現済み)MCP の設定で未知のキーが黙って無視される。`McpServer` は `flatten` を使うため `deny_unknown_fields` が効かず、`token_en` のようなタイポが無視されて 401 になる。`McpServer` を、`transport` ごとに使えるキーを検証して読み込むようにした。未知のキーと、`transport` に合わないキーはエラー
13. [x] (再現済み)`a2a send "-1 …"` のように `-` で始まる text が、clap にフラグと解釈される。`--` を付ければ回避できる。`allow_hyphen_values` か、標準入力から読む手段を検討する。`text` に `allow_hyphen_values` を付けた。`--` を付けなくても受け付ける
14. [x] (再現済み)`agentctl … | head` のようにパイプが切れると、`println!` が panic して exit 101 になる。標準出力への書き込みエラーを処理し、静かに終了する。標準出力への書き込みを `presentation::output` に集め、パイプの切断は終了コード 0 で静かに終える
15. [x] (未検証)環境変数が空文字のとき、空の Bearer トークンとして送ってしまう。「未設定」と同じエラーにする。空の環境変数は `env var NAME is empty` のエラーにした
16. [x] (未検証)タイムアウトが無い。応答しない A2A / MCP サーバーで CLI が止まり続ける。`task watch` は全体のタイムアウトから除外する。`--timeout`(既定 60 秒、`0` で無制限、`AGENTCTL_TIMEOUT`)を追加した。`task watch` は全体の制限から除く。ADR 0004 に決定を記録
17. [x] (未検証)`list` / `tools` の TSV で、説明に含まれるタブや改行をエスケープしていない。`list` と `tools` の列から、タブと改行を空白にまとめて除く
18. [x] (未検証)終端状態に達する前にストリームが閉じても、`task watch` は exit 0 になる。`canceled` も成功扱い。終端状態に達する前にストリームが閉じたら、終了コード 1 にした(`canceled` は成功のまま)。ADR 0004 に決定を記録
19. [x] (未検証)`state_info` は、未知の状態名だと空文字になり、失敗の判定を取りこぼす。`a2a-rust` の状態は閉じた列挙で、未知の状態は読み込み時にエラー(終了コード 1)になるため、空文字にはならない。全状態の名前と失敗判定を固定する回帰テストを足し、変換できない場合は `unknown` にした
20. [x] (設計判断)stdio サーバーの標準エラーを捨てているため、起動に失敗した原因が見えない。オプトインで通す手段を検討する。接続に失敗したときだけ、サーバーの標準エラーの末尾をエラーに含めるようにした。ADR 0004 に決定を記録
21. [x] (未検証)サーバーが終端状態のあともストリームを閉じないと、`task watch` が戻らない。終端状態で止めることを検討する。終端状態を受けたら、ストリームが閉じるのを待たずに終わるようにした。ADR 0004 に決定を記録

## 機能追加

22. [x] MCP クライアントから使えるようにする。同じパッケージの2つ目のバイナリ `agentctl-mcp`(`src/bin/agentctl-mcp/`)が、ツール呼び出しごとに `agentctl` をサブプロセスで実行する。`cargo install --path .` で `agentctl` と一緒に入る。公開するツールは `list` / `mcp_tools` / `mcp_schema` / `mcp_call` / `a2a_card` / `a2a_send` / `a2a_task_get` / `a2a_task_cancel`。`a2a task watch` は公開しない。stdio のみで、Streamable HTTP は未対応
23. [ ] `agentctl-mcp` の内部構成を決める。現在は平らな4モジュール(`command` / `runner` / `server` / `tools`)で、層構成の対象外としている。候補と点数は ADR 0003(`docs/adr/0003-mcp-wrapper-internal-structure.md`、Proposed。推奨は「C から入って D を目指す」)。決めたら、ADR を `Accepted` にし、`AGENTS.md` と `.agent/architecture/SKILL.md` の「対象外」の記述を直し、必要なら `check.sh` を `src/bin/agentctl-mcp/` に広げる
24. [ ] `agentctl-mcp` を同じパッケージに置いたことで、`agentctl` 本体のコンパイル時間がどれだけ増えるか(`rmcp` のサーバー機能を一緒にビルドする)を計測する。独立して公開する必要が出たら、`crates/agentctl-mcp` への切り出しを検討する(ADR 0002 の「見直す条件」)
25. [ ] 意思決定の手順(`.agent/decision/SKILL.md`)を実際の設計判断で使い、手順の足りない点を直す。最初の対象は ADR 0003
26. [ ] `agentctl-mcp` が、`PATH` や `AGENTCTL_BIN` より先に、自分と同じ場所にある `agentctl` を探すようにする。配布物(npm パッケージなど)に両方のバイナリが同梱されても、設定なしで動く。実装前に、探す順序(同じ場所 → `AGENTCTL_BIN` → `PATH` のどれを優先するか)を決める
