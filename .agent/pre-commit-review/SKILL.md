---
name: pre-commit-review
description: コミット前に差分を見直し、定めた観点で Approve か Reject のどちらかを判定する。コミットする前にユーザーが呼ぶ。
disable-model-invocation: true
---

# コミット前レビュー

コミット対象の差分を、下記の観点だけで見直し、**Approve** か **Reject** のどちらか一方を判定する。条件付きの承認や保留は出さない。

## 手順

1. `git status --short` で変更ファイルを把握する。
2. `git diff HEAD` で追跡中ファイルの差分を読む。未追跡ファイルは中身を直接読む。
3. 下記の観点を順に確認する。
4. 1つでも違反があれば Reject。すべて満たせば Approve。

## 観点

1. **機械検査**: 次がすべて成功する。
   - `.agent/architecture/check.sh`
   - `cargo fmt --check`
   - `cargo clippy -- -D warnings`
   - `cargo test`
2. **アーキテクチャ**: `.agent/architecture/SKILL.md` の層構成と設計方針に反していない。`check.sh` で検出できない違反(ロジックを `main.rs` に置く、不要な抽象化など)も見る。
3. **AGENTS.md との整合**: `AGENTS.md` の記述が実態と合っている。`AGENTS.md` 自身の書き方の注意書きにも従っている。
4. **混入物**: 個人設定、認証情報、ローカル専用の MCP 設定(`.mcp.json` への個人用サーバー追記など)、デバッグ用の残骸が含まれていない。
5. **変更の一貫性**: 1回のコミットとして目的が1つにまとまっている。無関係な変更が混ざっていない。

## 出力形式

先頭行に判定だけを書く。

```
Approve
```

```
Reject
```

Reject のときだけ、その下に違反を箇条書きにする。各項目は「観点の番号、ファイルと行、何が問題か」を書く。Approve のときは理由や補足を書かない。

## 禁止

- Approve / Reject 以外の判定を出さない。
- 判定のあとにコミットやファイル修正を自動で行わない。
- Reject のあとに修正して自動で再レビューしない。再実行はユーザーが呼んだときだけ行う(無限ループ防止)。
