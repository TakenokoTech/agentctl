# src/bin/agentctl-mcp/AGENTS.md

書き方と単体テストの置き方は、`src/AGENTS.md` に従う。

- このバイナリは、MCP のツール呼び出しを `agentctl` のサブプロセス実行に変換するだけにする。`agentctl` の内部コード(`domain` など)を取り込まない。
- ツールの引数は、呼び出し側(LLM など)が与える。名前や ID は `-` で始まるものを拒否し、自由なテキストは `--` の後ろに置いて、`agentctl` のオプションとして解釈されないようにする。
- `agentctl` の実装に依存しない結合テストは、偽の `agentctl` で `tests/mcp_wrapper_fake_cli.rs` に書く。本物の `agentctl` との結合は `tests/e2e/mcp_wrapper.rs` で確かめる。
