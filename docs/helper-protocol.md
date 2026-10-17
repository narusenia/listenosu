# ヘルパーとの通信プロトコル

アプリ本体（Rust）と C# ヘルパーは、子プロセスの標準入出力で JSON Lines（1 行 1 メッセージ、UTF-8、改行は `\n`）をやり取りする。標準エラー出力はログ用で、プロトコルには含めない。

ヘルパーは lazer の `client.realm` を **読み取り専用・動的**に開く。書き込みは一切しない（検証結果は implementations.md の 2026-10-17 の項を参照）。

## アプリからヘルパーへ

| `cmd` | 項目 | 意味 |
| --- | --- | --- |
| `open` | `data_dir` | lazer のデータフォルダを指定して読み込みを始める。`version` と `snapshot` を返し、以後は更新を監視する。もう一度送ると切り替える |
| `refresh` | なし | 今すぐ読み直す。結果は `changed` で返る |
| `quit` | なし | 終了する（標準入力が閉じたときも終了する） |

## ヘルパーからアプリへ

| `type` | 項目 | 意味 |
| --- | --- | --- |
| `version` | `helper`, `realm`, `lazer`（不明なら null） | `open` の直後に 1 回。エラー時の案内に使う |
| `snapshot` | `songs`, `collections` | `open` への返答。全件を 1 回で送る |
| `changed` | `songs`, `collections` | 更新を検知して読み直した結果。差分ではなく全件の再送 |
| `error` | `code`, `message` | 読めなかった。`code` は下表 |

### error の code

| `code` | 意味 |
| --- | --- |
| `no_database` | `data_dir` に `client.realm` が無い |
| `open_failed` | 開けなかった。lazer の更新でフォーマットやスキーマが変わった可能性が高い（要件 §7） |
| `read_failed` | 開けたが、必要な項目を読めなかった |
| `unstable_read` | 3 回読み直しても、読んでいる間に lazer が書き込み続けていた |
| `bad_command` | 解釈できないコマンド |

エラーの後も、ヘルパーは動き続ける。以後の更新や `refresh` で読めれば `changed` を送る。

### song

音声ファイルのハッシュ単位で重複をまとめた 1 曲。Hidden の譜面と DeletePending の譜面セットは含めない。代表のメタデータは、その音声を使う譜面のうち先頭のもの。

| 項目 | 型 | 内容 |
| --- | --- | --- |
| `audio_hash` | string | 音声ファイルの SHA-256。実体は `<data_dir>/files/<h[0]>/<h[0..2]>/<hash>` |
| `background_hash` | string / null | 背景画像のハッシュ（同じ配置） |
| `online_id` | number / null | 代表の譜面セットの osu! 上の ID（譜面ページと画像の URL に使う）。osu! から取得していないセットや、読めなかったときは null |
| `title`, `title_unicode` | string | 曲名（ローマ字、原語）。空のときは空文字 |
| `artist`, `artist_unicode` | string | アーティスト名（ローマ字、原語） |
| `source`, `tags`, `author` | string | ソース、タグ、譜面作者 |
| `bpm` | number | 代表の譜面の BPM |
| `length_ms` | number | 代表の譜面の長さ（ミリ秒）。0 のことがある（手元では 1 曲）ので、0 のときは再生時に取得した長さを使う |
| `date_added_ms` | number | 譜面セットの追加日時（Unix エポックからのミリ秒） |
| `last_played_ms` | number / null | この音声を使う譜面のうち最も新しい最終プレイ日時（Unix エポックからのミリ秒）。無ければ null |
| `beatmap_md5s` | string[] | この音声を使う譜面すべての MD5（コレクションとの突き合わせ用） |

### collection

| 項目 | 型 | 内容 |
| --- | --- | --- |
| `name` | string | コレクション名 |
| `beatmap_md5s` | string[] | 含まれる譜面の MD5 |

## 更新の検知

`client.realm` の先頭 24 バイトを約 1 秒ごとに読み、変わったら約 300 ms 待って開き直し、全件を読んで閉じる。読む前後で先頭 24 バイトが変わっていたら読み直す（3 回まで）。更新日時とファイル監視は、mmap のため当てにならない。
