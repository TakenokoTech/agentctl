# AGENTS.md

> **このファイルの書き方**
>
> - コードや設定から確認できる事実だけを書く。依頼されていない運用ルールや方針を推測で足さない。
> - 時間が経っても変わらない(タイムレスな)記述だけを書く。「現在は〜」「〜段階」「まだ〜がない」など古くなる記述は書かない。
> - コードや設定ファイルを読めばわかることは書かない。
> - 内容を変えたら、このファイルの記述と実態が合っているかも確認する。

## プロジェクト概要

`agentctl` は Rust 製の CLI プロジェクトです。

## 構成

DDD に沿った層構成。ルールは `.agent/architecture/SKILL.md` に従う。`src` の構成や依存を変えたら `.agent/architecture/check.sh` で検証する。

## 開発コマンド

```sh
cargo build            # ビルド
cargo run              # 実行
cargo test             # テスト
cargo fmt              # フォーマット
cargo clippy           # lint
```

## 注意事項

- `.mcp.json` はチーム共有用。個人用の MCP サーバーは `claude mcp add --scope local` で登録し、`.mcp.json` には書かない。
