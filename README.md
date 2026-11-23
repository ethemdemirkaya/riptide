# Riptide

A fast, minimal BitTorrent client written in Rust.

Riptide aims to be the torrent client you don't have to think about:
paste a magnet link, pick a folder, done. It stays out of your way, remembers
everything between restarts, and never asks you to configure ports or trackers
unless you want to.

> Work in progress. See the open pull requests for what is being built.

## Building

```sh
cargo run --release
```

On Linux you need the GTK3 and xkbcommon development headers
(`libgtk-3-dev libxkbcommon-dev` on Debian/Ubuntu).

## License

MIT
