# tests/AGENTS.md

書き方は、ルートの `AGENTS.md` の注意書きに従う。

- テスト専用のサーバやヘルパは `tests/fixtures/` に置く。`examples/` は利用者向けの使用例用で、テスト用途には使わない。
- 別バイナリとして起動する必要があるときは、`Cargo.toml` の `[[example]]` で `path` を `tests/fixtures/` に向ける。
