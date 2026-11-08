# agentctl ドキュメント

[← README](../../README.ja.md) | [English](../index.md) | 日本語

[MCP](https://modelcontextprotocol.io) サーバーと [A2A](https://a2a-protocol.org) エージェントを、1つのコマンドラインツールから呼び出します。

`agentctl` は、シェル経由で作業する AI エージェント向けに作られています。すべてのツール定義を AI エージェントのコンテキストに読み込む代わりに、`agentctl list` から始めて、必要な深さまでだけ掘り下げます。

```console
$ agentctl list
mcp	demo	Demo MCP server
a2a	helper	Demo A2A agent

$ agentctl mcp demo tools
echo	Echo the message
structured	Return structured content
fail	Always return an error result

$ agentctl mcp demo call echo --args '{"message":"hi"}'
echo: hi
```

上の出力は、このリポジトリのテストスイートが使うテスト用サーバーのものです。MCP の部分は [クイックスタート](../../README.ja.md#クイックスタート) で再現できます。

> **用語。** *AI エージェント* は `agentctl` を実行するプログラムです。*A2A エージェント* は `agentctl` が通信する相手のリモートエージェントです。

## 特徴

- MCP サーバー(stdio と Streamable HTTP)と A2A エージェントを、**1つの入口**から呼べる
- **段階的な開示**: `list` → `tools` → `schema` → `call` と進むので、必要な定義だけが AI エージェントに届く
- **既定は簡潔な出力**: 一覧は TSV、それ以外は1行の JSON(`mcp call` は text をそのまま出力)。`--raw` で `mcp call` と A2A コマンドのプロトコルの応答をそのまま出力
- 「呼び方が悪い」と「ツールが失敗した」を区別できる**終了コード**
- **長時間の A2A タスク**: 待たずに送信し、あとから `get`、`watch`(NDJSON で流す)、`cancel`

## 実行例

### A2A の例

次のように A2A エージェントを設定すると、`send` が返信を返します。

```toml
[a2a.helper]
description = "Demo A2A agent"
url = "http://127.0.0.1:3000"
```

```console
$ agentctl a2a helper send hello
{"contextId":"152860f9-0f62-4e2d-aaae-b54324b88064","text":"echo: hello","type":"message"}
```

時間のかかる依頼は task を返します。待たずに送信し、あとから監視します(1行に1つの JSON イベント)。

```console
$ agentctl a2a helper send "slow report" --no-wait
{"artifacts":[],"contextId":"d1bf1619-2ba3-436d-aed8-6f0b2083e33a","id":"8253770c-a237-43c7-8742-e1b1ac096ff6","message":null,"state":"working","type":"task"}
$ agentctl a2a helper task watch 8253770c-a237-43c7-8742-e1b1ac096ff6
{"artifacts":[],"contextId":"d1bf1619-2ba3-436d-aed8-6f0b2083e33a","id":"8253770c-a237-43c7-8742-e1b1ac096ff6","message":null,"state":"working","type":"task"}
{"message":null,"state":"completed","taskId":"8253770c-a237-43c7-8742-e1b1ac096ff6","type":"status"}
```

### 終了コードの実例

ツールがエラーを返すと終了コードは `2` になり、エラーの本文も出力されます。

```console
$ agentctl mcp demo call fail
boom
$ echo $?
2
```

task が `failed` で終わった場合も `2` で終了します。

```console
$ agentctl a2a helper send "fail it"
{"artifacts":[],"contextId":"03528ca0-a394-4643-9f1b-737c612b91b2","id":"4dd01a05-b4d8-4b13-b978-7199799df979","message":null,"state":"failed","type":"task"}
$ echo $?
2
```

上の A2A の出力は、このリポジトリのテストスイートが使うテスト用エージェントのものです。

## AI エージェントから使う

AI エージェントには、`agentctl list` から始めて、必要なときだけ一段ずつ深く進むよう伝えます。

```text
Use `agentctl` to reach external tools and agents.
1. `agentctl list` shows what is available.
2. `agentctl mcp <server> tools`, then `agentctl mcp <server> schema <tool>`, then `agentctl mcp <server> call <tool> --args '<json>'`.
3. `agentctl a2a <agent> card`, then `agentctl a2a <agent> send "<text>"`.
Exit code 2 means the tool or task itself failed; 1 means the call could not be made; 64 means the command line was invalid.
```

## マニュアル(英語)

マニュアルは英語のみです。

- [Usage](../manual/usage.md): コマンド、オプション、ツールの引数の渡し方
- [Configuration](../manual/configuration.md): 設定ファイルと認証
- [Reference](../manual/reference.md): 終了コード、トラブルシューティング、既知の制限
- [Development](../manual/development.md): ビルド、テスト、コード構成
