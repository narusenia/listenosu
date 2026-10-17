# listenosu!

English | [日本語](README_ja.md)

A music player for listening to the songs in your osu!lazer library as background music.

> **This is an unofficial app.** It is not affiliated with or endorsed by osu! or ppy Pty Ltd.

## Download

Get the latest zip for Windows 10 / 11 (64-bit) from [Releases](https://github.com/narusenia/listenosu/releases/latest), extract it, and run `listenosu.exe`. Keep `listenosu-helper.exe` and the other files next to it: the helper is what reads your lazer library. osu!lazer must be installed (it is found automatically, or you can pick its data folder in the settings).

The app is not code-signed, so Windows SmartScreen may warn you the first time. macOS and Linux builds are not available yet.

## Features

- Works while osu!lazer is running (lazer's data is opened read-only and never modified)
- A song list with duplicate audio merged into one entry, full-text search, and sorting
- lazer collections, plus your own playlists
- Shuffle, repeat, a play queue, and crossfade
- Audio spectrum
- A UI styled after lazer's song select screen, and a mini player mode
- Media keys, OS media controls, and a system tray icon
- Discord Rich Presence and global hotkeys. Discord needs an Application ID of your own from the Discord Developer Portal; the beatmap cover and page button are optional and off by default
- Color themes (a few are bundled; put your own `*.toml` in `%APPDATA%\listenosu\themes`)
- Space plays and pauses while the window is focused; launching the app a second time brings the running window forward
- English and Japanese

Windows is the primary platform, followed by macOS. Linux is best-effort. Currently only Windows is released.

## Documentation

- [Requirements](docs/requirements.md) (Japanese)
- [Implementation plan and progress](docs/implementations.md) (Japanese)

## Development

Tool versions are managed with [mise](https://mise.jdx.dev/).

```sh
mise install
cargo build --workspace
cargo run -p gallery   # UI gallery
```

On Windows, MSVC Build Tools ("Desktop development with C++") are required. Run the commands from PowerShell.

## License

listenosu! is under the [MIT License](LICENSE). It bundles these fonts and icons:

- Nunito and Noto Sans JP: SIL Open Font License 1.1
- Lucide icons: ISC License
- Color themes (palettes of Catppuccin, Nord, Dracula, Tokyo Night, Gruvbox, Solarized and Rosé Pine): MIT License

The osu! logo and the Torus font are not included. "osu!" is a name of ppy Pty Ltd.
