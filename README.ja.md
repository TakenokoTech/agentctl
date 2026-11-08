# agentctl

[English](README.md) | 日本語

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

上の出力は、このリポジトリのテストスイートが使うテスト用サーバーのものです。MCP の部分は [クイックスタート](#クイックスタート) で再現できます。

## インストール

`agentctl` は crates.io には公開していません。Rust 1.85 以降でソースからビルドします。

```sh
git clone <this repository>
cd agentctl
cargo install --path .
```

これで2つのプログラムが入ります。`agentctl` と、MCP クライアントから `agentctl` を使うための MCP サーバー `agentctl-mcp` です([MCP クライアントから使う](#mcp-クライアントから使う) を参照)。

## クイックスタート

リポジトリに含まれるテスト用 MCP サーバーをビルドし、`agentctl.toml` に記述します(コマンドはリポジトリのルートで実行します)。

```sh
cargo build --example test_mcp_server
```

```toml
[mcp.demo]
description = "Demo MCP server"
transport = "stdio"
command = ["target/debug/examples/test_mcp_server"]
```

そのうえで、確認して呼び出します。

```sh
agentctl list
agentctl mcp demo tools
agentctl mcp demo schema echo
agentctl mcp demo call echo --args '{"message":"hi"}'
```

自分のサーバーやエージェントを使うには、[設定](docs/manual/configuration.md)(英語) に従って設定ファイルに追加します。

## MCP クライアントから使う

`agentctl-mcp` は、各コマンドを stdio の MCP ツールとして公開します。MCP クライアントは、シェルを使わずにこれらを呼べます。クライアントの MCP 設定(`mcp.json` や `.mcp.json` など)に追加します。

```json
{
  "mcpServers": {
    "agentctl": {
      "command": "agentctl-mcp",
      "env": { "AGENTCTL_CONFIG": "/path/to/agentctl.toml" }
    }
  }
}
```

`agentctl-mcp` は、ツールを呼ぶたびに `agentctl` を実行します。そのため `agentctl` が `PATH` 上にあるか、`AGENTCTL_BIN` にそのパスを設定してください。使えるツールは [Usage](docs/manual/usage.md#running-as-an-mcp-server)(英語)にあります。

## ドキュメント

- [概要](docs/ja/index.md): 特徴、実行例、AI エージェントからの使い方
- マニュアル(英語): [Usage](docs/manual/usage.md) · [Configuration](docs/manual/configuration.md) · [Reference](docs/manual/reference.md) · [Development](docs/manual/development.md)
