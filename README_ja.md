# listenosu!

[English](README.md) | 日本語

osu!lazer のライブラリにある楽曲を、作業用 BGM として流し聴きするための音楽プレーヤーです。

> **これは非公式のアプリです。** osu! および ppy Pty Ltd とは関係がなく、承認も受けていません。

## ダウンロード

[Releases](https://github.com/narusenia/listenosu/releases/latest) から Windows 10 / 11（64 ビット）用の最新の zip を取得し、展開して `listenosu.exe` を実行します。`listenosu-helper.exe` などのファイルは、`listenosu.exe` の隣に置いたままにしてください。ヘルパーが lazer のライブラリを読みます。osu!lazer が必要です（自動で見つけます。設定でデータフォルダを選ぶこともできます）。

コード署名をしていないため、初回は Windows の SmartScreen が警告を出すことがあります。macOS と Linux 向けはまだ公開していません。

## 特徴

- lazer を起動したままでも使える（lazer のデータは読み取り専用で扱い、書き込みません）
- 同じ音声は 1 曲にまとめた一覧、全文検索、並べ替え
- lazer のコレクションの表示と、独自のプレイリスト
- シャッフル、リピート、再生キュー、クロスフェード
- オーディオスペクトラム
- lazer の選曲画面風の UI、ミニプレイヤー
- メディアキー、OS のメディア表示、システムトレイ
- Discord Rich Presence、グローバルホットキー。Discord は、Discord Developer Portal で自分で作った Application ID が必要。譜面のカバー画像とページのボタンは任意で、既定は OFF
- カラーテーマ（いくつか同梱。自作の `*.toml` は `%APPDATA%\listenosu\themes` に置く）
- ウィンドウにフォーカスがあるとき Space で再生 / 一時停止。2 つ目を起動すると、動いているウィンドウを前面に出す
- 英語と日本語

対応 OS は Windows を最優先とし、次に macOS、Linux は努力目標です。現在公開しているのは Windows のみです。

## ドキュメント

- [要件定義](docs/requirements.md)（日本語）
- [実装計画と進捗](docs/implementations.md)（日本語）

## 開発

ツールのバージョンは [mise](https://mise.jdx.dev/) で管理します。

```sh
mise install
cargo build --workspace
cargo run -p gallery   # UI ギャラリー
```

Windows では MSVC Build Tools（C++ によるデスクトップ開発）が必要です。PowerShell から実行してください。

## ライセンス

listenosu! は [MIT License](LICENSE) です。次のフォントとアイコンを同梱しています。

- Nunito と Noto Sans JP：SIL Open Font License 1.1
- Lucide アイコン：ISC License
- カラーテーマ（Catppuccin、Nord、Dracula、Tokyo Night、Gruvbox、Solarized、Rosé Pine の配色）：MIT License

osu! のロゴと Torus フォントは含みません。「osu!」は ppy Pty Ltd の名称です。
