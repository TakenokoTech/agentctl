---
name: architecture
description: agentctl の DDD レイヤー構成と依存ルール。src 配下のモジュール追加・移動・依存の変更をするとき、および構成を検証したいときに使う。
---

# アーキテクチャ

単一クレート内のモジュールで層を分ける。

| 層 | 役割 | 依存してよい先 |
|---|---|---|
| `domain` | エンティティ、値オブジェクト、リポジトリなどの `trait` | なし |
| `application` | ユースケース | `domain` |
| `infrastructure` | `domain` の `trait` の実装(外部 I/O) | `application`, `domain` |
| `presentation` | CLI の引数処理と出力 | `application`, `domain` |

- `infrastructure` と `presentation` は互いに依存しない。
- `main.rs` は各層の組み立てのみ。ロジックは置かない。

## 設計方針

- 依存の逆転は `trait` で行う。
- 値オブジェクトは newtype で表し、不変条件はコンストラクタで検証する。
- 層や抽象化を必要以上に増やさない。

## 検証

変更後に実行する。違反があれば該当箇所を表示して失敗する。

```sh
.agent/architecture/check.sh
```

実体は `tests/architecture.rs`(`cargo test --test architecture` でも実行できる)。`src/**/*.rs` を `syn` で解析し、複数行の `use`、入れ子のグループ、`crate::` / `self::` / `super::` の相対パスに加えて、`use` を使わずに書いたパスも検出する。`src/bin/` は対象外。
