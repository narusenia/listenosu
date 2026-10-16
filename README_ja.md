# listenosu

[English](README.md) | 日本語

osu!lazer のライブラリにある楽曲を、作業用 BGM として流し聴きするための音楽プレーヤーです。

> **これは非公式のアプリです。** osu! および ppy Pty Ltd とは関係がなく、承認も受けていません。

> **開発中です。** まだ使えるものはありません。

## 特徴（予定）

- lazer を起動したままでも使える（lazer のデータは読み取り専用で扱い、書き込みません）
- 同じ音声は 1 曲にまとめた一覧、全文検索、並べ替え
- lazer のコレクションの表示と、独自のプレイリスト
- シャッフル、リピート、再生キュー、クロスフェード
- オーディオスペクトラム
- lazer の選曲画面風の UI、ミニプレイヤー
- メディアキー、OS のメディア表示、システムトレイ
- Discord Rich Presence、グローバルホットキー（どちらも既定は OFF）
- 英語と日本語

対応 OS は Windows を最優先とし、次に macOS、Linux は努力目標です。

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

[MIT](LICENSE)
