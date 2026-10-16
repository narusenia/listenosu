# AGENTS.md

osu!lazer のライブラリにある楽曲を流し聴きするための、非公式の音楽プレーヤー（Rust + egui + kira）。

- 要件は [docs/requirements.md](docs/requirements.md)、実装計画と進捗は [docs/implementations.md](docs/implementations.md) にある。作業の前に読む
- 作業を終えたら、implementations.md のチェックボックスと進捗サマリを更新する。決定事項や仕様変更は変更履歴に日付付きで追記する

## 構成

- `crates/ui`：テーマ定義・UI 部品・画面（gallery と本体で共有）
- `crates/gallery`：モックデータで全画面と部品を表示する UI ギャラリー
- `crates/listenosu`：本体バイナリ
- `crates/library`：曲モデル・検索・並べ替え・ヘルパーとの通信・キャッシュ
- `crates/player`：kira による再生・キュー・クロスフェード・スペクトラム解析
- `crates/platform`：OS のメディア連携・トレイ・ホットキー・Discord
- `helper/`：client.realm を読む C# ヘルパー（Realm .NET）

## ビルドとツール

- ツールのバージョンは `mise.toml` で管理する。`mise install` で入れる
- ビルド：`cargo build --workspace`
- 検査：`cargo fmt --all --check` と `cargo clippy --workspace --all-targets -- -D warnings`
- Windows では MSVC Build Tools が必要。Git Bash から `cargo` を実行すると、coreutils の `link` が MSVC の `link.exe` より先に見つかってビルドに失敗する。PowerShell から実行する

## 守ること

- lazer の DB とデータフォルダには、**書き込まない**。読み取り専用で扱う
- `realm_db_reader` は使わない。lazer の client.realm（フォーマット 24）を読めないため、C# ヘルパーで読む
- UI のコードに色・寸法・フォントサイズの値を直接書かない。値はすべて `Theme` から参照する
- UI の文言は翻訳ファイルで管理し、コードに直接書かない
- osu! のロゴと Torus フォントは同梱しない。README と About に「非公式」を明記する
- 利用者が頼まない限り、新しいソフトウェアやパッケージを入れず、システム設定や環境変数を変更しない

## Git

- 利用者が頼まない限り commit / push / PR はしない
- commit は論理単位 1 つ、英語 1 行の Conventional Commit（`feat:` `fix:` `refactor:` `docs:` `test:` `chore:` `perf:` `ci:`）。prefix の後は小文字
- ブランチ名は同じ prefix + 具体的な kebab-case（例：`feat/varint-codec`）
- commit メッセージにタスク ID・issue 番号・エージェント名を入れない
- PR は merge commit でマージする
