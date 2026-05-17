# The Commons Browser — v1.0 Roadmap & Handover

## Current State: v0.1.2 (installed)
- **Source:** `/home/isaulysses/projects/the-commons/`
- **GitHub:** `github.com/Ulysses05151997/the-commons`
- **Binary launch path:** `/home/isaulysses/projects/claude-desktop/src-tauri/target/release/the-commons`
  - NOTE: The .desktop launcher points HERE, not to `/usr/bin/the-commons`. Both must be updated on install.
- **System install:** `/usr/bin/the-commons` (via dpkg)
- **Config dir:** `~/.config/the-commons/` (favorites.json lives here)
- **Data dir:** `~/.local/share/org.computecommons.browser/` (WebKitGTK data — currently wiped on every launch)
- **Stack:** Tauri 2 + WebKitGTK (webkit2gtk 2.0) + GTK 0.18, Rust

## What v0.1.2 Has
- Tabbed browsing (new tab, close tab, switch tab)
- Toolbar with URL bar, back/forward/refresh/stop
- Smart navigation (bare words → DuckDuckGo search, domains auto-prefixed with https://)
- Favorites backend (save/remove/load commands exist)
- Offline fallback page
- No history, no persistent cookies, no tracking (wipes `~/.local/share/` and `~/.cache/` on launch AND exit)
- Download handler: saves to ~/Downloads, avoids overwrites, desktop notifications via notify-send
- New-window policy: target="_blank" and target="_top" links navigate in current tab
- Homepage: computecommons.cloud

## What's Broken in v0.1.2
- **Favorites UI doesn't work** — backend commands exist but toolbar JS may not be wired up correctly. Need to read `src/index.html` (the toolbar) and trace the issue.
- **No printing** — Ctrl+P does nothing
- **No password saving** — no credential storage at all
- **No desktop shortcut creation** — can't pin sites as apps
- **No cookie persistence option** — everything is wiped every launch, no choice

## v1.0 Feature List

### 1. Printing (Quick)
- Bind Ctrl+P to `window.print()` on the active webview
- WebKitGTK handles the print dialog natively
- Estimated: 10 minutes

### 2. Cookie/Password Persistence Toggle (Medium)
- Add a `persistent_mode` boolean to `~/.config/the-commons/config.json`
- When `persistent_mode: false` (default): current behavior — wipe everything on launch/exit
- When `persistent_mode: true`: skip `cleanup_browser_data()`, preserve WebKitGTK data dir
- Toggle accessible from toolbar (gear icon or menu)
- When switching FROM true TO false: immediately wipe all saved data
- Estimated: 30-45 minutes

### 3. Password Saving (Heavy)
- WebKitGTK can integrate with GNOME Keyring / libsecret for credential storage
- Only active when `persistent_mode: true`
- Need to configure WebKitGTK's WebContext to use the system credential store
- Alternatively: custom credential storage in `~/.config/the-commons/credentials.enc` (encrypted with a master password)
- When switching to non-persistent mode: credentials are deleted
- Estimated: 1-2 hours (depends on WebKitGTK's credential API)

### 4. Fix Favorites (Medium)
- Debug `src/index.html` toolbar JavaScript
- The Rust backend commands work: `save_favorite`, `remove_favorite`, `load_favorites`
- Issue is likely in the toolbar's JS — either the invoke calls aren't firing or the UI isn't rendering the results
- Estimated: 30 minutes (diagnosis) + fix time

### 5. Desktop Shortcuts (Medium)
- Right-click context menu option: "Create Desktop Shortcut"
- Or a toolbar button/menu option: "Add to Desktop"
- Writes a `.desktop` file to `~/.local/share/applications/` with:
  - Name = page title
  - Exec = `the-commons --url=<URL>` (need to add CLI arg support)
  - Icon = favicon (download and save to `~/.local/share/icons/`)
  - Categories = Network;WebBrowser;
- The shortcut opens The Commons directly to that URL
- Estimated: 1 hour

### 6. Print Support Details
- Add keyboard shortcut handler in the Tauri setup
- Intercept Ctrl+P, inject `window.print()` into active webview
- WebKitGTK's print dialog supports PDF export, printer selection, page range
- No custom UI needed — the system dialog handles everything

## Implementation Order
1. Printing (trivial, instant win)
2. Persistence toggle (unlocks items 3 and 4)
3. Fix favorites (quick debug)
4. Password saving (depends on persistence toggle)
5. Desktop shortcuts (standalone feature)

## Version Strategy
- Current: v0.1.2
- This roadmap: v1.0.0
- Full version bump — this is the "real browser" release
- Update Cargo.toml version, rebuild .deb, create GitHub release, update computecommons.cloud download link

## Site Changes Made Today
- computecommons.cloud/declaration.html — PDF rendered via pdf.js, no download needed
- .deb download button copies `wget + dpkg -i` command to clipboard
- declaration link opens declaration.html (same domain, same tab)
- All changes pushed to github.com/Ulysses05151997/computecommons-site

## Session Notes
- The .desktop launcher points to `/home/isaulysses/projects/claude-desktop/src-tauri/target/release/the-commons`, NOT `/usr/bin/the-commons`. Fix the launcher to point to the system install path.
- WebKitGTK has NO built-in PDF viewer. PDFs trigger downloads, not inline rendering. Any PDF viewing must go through pdf.js or similar.
- `webkit2gtk::NavigationPolicyDecisionExt::request()` is deprecated since webkit2gtk 2.6 — produces a compiler warning. Should migrate to the replacement API in v1.0.
- RAM note: 4x Hynix 2GB DDR3-1066 (PC3-8500S) sticks salvaged from 2009 iMac — inventory for Real Tech Bros, wrong speed for current machines.
