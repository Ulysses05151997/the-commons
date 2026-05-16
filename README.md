# The Commons Browser

A lightweight, privacy-focused web browser built in Rust with Tauri v2 and WebKitGTK. Part of The Compute Commons project -- a community-owned distributed computing platform for the Linux community.

The Commons is the community's gateway to the web: fast, minimal, and private by architecture.

## Features

- **Privacy by default** -- no browser history stored, ever. No visited URLs on disk. Nothing persists between sessions.
- **No persistent cookies** -- session cookies work normally while the browser is open. When the browser closes, all cookies are destroyed.
- **Smart URL bar** -- type `reddit` and it goes to reddit.com. Type `linux news` and it searches DuckDuckGo. Handles URLs with dots, single words, and search queries intelligently.
- **Tabs** -- lightweight tab support sharing a single WebKitGTK process. No 200MB-per-tab Chrome bloat.
- **Favorites** -- the only thing the browser writes to disk. Saved as simple JSON at `~/.config/the-commons/favorites.json`.
- **Home page** -- opens to computecommons.cloud with a DuckDuckGo fallback if unreachable.
- **Idle the Penguin** -- the Compute Commons mascot as the app icon.
- **9.6MB binary** -- the entire browser.

## Requirements

- Linux with GTK 3 and WebKitGTK 4.1+
- Rust toolchain (rustup.rs)
- System packages:

```bash
# Ubuntu/Debian
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev build-essential

# Fedora
sudo dnf install gtk3-devel webkit2gtk4.1-devel gcc
```

## Build

```bash
cd src-tauri
cargo build --release
```

The binary is at `target/release/the-commons`.

## Run

```bash
./src-tauri/target/release/the-commons
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+L | Focus URL bar |
| Ctrl+T | New tab |
| Ctrl+W | Close current tab |
| Ctrl+R | Refresh page |
| Enter (in URL bar) | Navigate or search |
| Escape | Stop loading |

## Smart URL Bar

- `google.com` -- navigates to https://google.com
- `reddit` -- navigates to https://www.reddit.com (DNS check)
- `linux news` -- searches DuckDuckGo
- `what is rust` -- searches DuckDuckGo
- `https://example.com` -- navigates directly

## Privacy Architecture

The Commons stores nothing. No history. No cookies. No cache. No tracking. The only file on disk is `~/.config/the-commons/favorites.json` -- and only if you explicitly bookmark a page.

On startup, the browser clears any data from the previous session. On shutdown, it clears again. WebKitGTK's data and cache directories for this app are wiped both ways.

## Desktop Entry

```bash
cat > ~/.local/share/applications/the-commons.desktop << 'EOF'
[Desktop Entry]
Name=The Commons
Comment=Privacy-focused browser for The Compute Commons
Exec=/path/to/src-tauri/target/release/the-commons
Icon=/path/to/src-tauri/icons/icon.png
Type=Application
Categories=Network;WebBrowser;
EOF
```

## Tested On

- Ubuntu 24.04
- MacBook Pro 17" (2011) -- Intel HD 3000, 16GB RAM

## License

MIT

## Author

Ulysses Isa -- [github.com/Ulysses05151997](https://github.com/Ulysses05151997)
