# 0006. Bearer トークンは、https か loopback の http にだけ送る

- ステータス: Accepted
- 日付: 2026-11-08
- 関連: PR #3 の CodeRabbit の指摘(`src/infrastructure/a2a.rs`、`src/infrastructure/mcp.rs`)

## 状況

`token_env` を設定すると、値が `Authorization: Bearer` として全リクエストに付く。URL の検査が無く、`http://` の非 loopback な URL では、トークンが平文でネットワークに流れる(CWE-319)。設定ファイルの形(公開インターフェース)に関わる。

## 決定

点数化せず、ユーザーが次の選択肢から選んだ。

- トークンがあり、URL が `https://` でも loopback(`localhost`、`127.0.0.0/8`、`::1`)の `http://` でもないときは、通信の前にエラーで終了する(終了コード 1)。
- 設定の `allow_insecure = true`(`http` の MCP サーバーと A2A エージェントのエントリ)で、この検査を解除できる。既定は `false`。stdio の MCP サーバーには置けない。
- 検査は `infrastructure::config::ensure_secure_transport` に1か所で持ち、MCP と A2A の両方が使う。
- `Authorization` のヘッダ値に `set_sensitive(true)` を付け、`Debug` 出力にトークンが出ないようにする。

選ばれなかった案: 警告だけ出して送る(B。平文送信を止められない)。常に拒否してオプトインも無くす(閉じたネットワーク内の `http://` サービスが使えなくなる)。

## 結果

- 良い点: 既定で安全になる。意図した平文利用は、設定に明示した場合だけ可能。
- 代償: 既存の設定で、非 loopback の `http://` とトークンを組み合わせていたものは、`allow_insecure = true` を足すまで動かない。ホスト名が loopback を指すかは名前で判断し、DNS は引かない(`localhost` 以外の名前が 127.0.0.1 を指していても拒否する)。
- 見直す条件: URL ごとではなく環境全体で緩めたいという要望が出たとき。
