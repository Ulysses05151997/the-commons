# claude-native — Build Instructions

## The Approach

DO NOT inject mainView.js into the webview. It is an Electron preload script full of Node.js APIs that don't exist in a browser. Every attempt to fake those APIs creates an endless chain of compatibility errors.

Instead: read mainView.pretty.js (the de-minified preload), understand what globals it creates on `window`, and write a clean browser-native JavaScript file that creates those same globals directly. The globals route IPC through Tauri instead of Electron.

mainView.js is a machine that produces 14 window globals. We don't need the machine. We need the globals.

## What exists and works

### MCP host crate (done, tested, do not modify)
- `claude-mcp-host/` — 5 servers, 82 tools, `McpHost::from_default_sources().await`

### Tauri shell (done, compiles, do not rebuild)
- `src-tauri/` — real Tauri 2.x, builds clean, window loads claude.ai
- `src-tauri/src/lib.rs` — has `ipc_dispatch` command and MCP integration
- Keep the Rust backend as-is. Only the JavaScript init script changes.

### De-minified preload (reference only — do not inject)
- `electron-extracted/mainView.pretty.js` — 7,689 lines, READ this to understand what globals to create
- `docs/PRELOAD_ANALYSIS.md` — summary of what mainView.js exposes

## What you build

One file: `src/desktop-globals.js` — clean browser JavaScript, no Electron APIs, no fake require().

This file creates every window global that claude.ai checks for to enable desktop mode. Each global's methods call `window.__TAURI__.core.invoke('ipc_dispatch', ...)` for request/response and `window.__TAURI__.event.listen(...)` for events.
## The 14 globals to create

Read mainView.pretty.js and docs/PRELOAD_ANALYSIS.md to find the exact shape of each global. Here is what must exist on window before claude.ai's own JavaScript runs:

### Critical for desktop detection
1. `window.process` — `{ platform: 'linux', arch: 'x64', type: 'renderer', argv: [], versions: { electron: '33.0.0', chrome: '130.0.0.0', node: '20.18.0' }, version: '0.1.0', env: {} }`
2. `window.desktopBootFeatures` — object with feature flags. Read mainView.pretty.js around the exposeInMainWorld("desktopBootFeatures",...) call to find what flags it sets.
3. `window.claudeAppBindings` — the main IPC bridge. Must have `registerBinding(channel, callback)` and `unregisterBinding(channel)`. Read mainView.pretty.js to find all methods.

### Desktop UI features
4. `window.electronWindowControl` — `{ minimize(), maximize(), close(), fullscreen(), isFullscreen() }` — each calls Tauri window commands
5. `window.electronIntl` — `{ getInitialLocale() }` — MUST return the string "en-US", not null, not {}, not undefined. This is what causes the p.toLowerCase crash.
6. `window.claude.settings` — namespace with MCP, Extensions, AppPreferences, AppConfig, DesktopInfo, Startup, GlobalShortcut, FilePickers sub-objects
7. `window.claude.web` — namespace with LocalAgentModeSessions (MCP tool calls), AutoUpdater, other sub-objects

### Other globals
8-14. Read PRELOAD_ANALYSIS.md for the remaining globals. Each one follows the same pattern: an object with methods that call ipc_dispatch through Tauri.

## How each method works

Every method on every global follows one of three patterns:

### Pattern 1: Async invoke (most methods)
```javascript
async methodName(...args) {
    const channel = "$eipc_message$_a0ef28e9-b3bd-4b5e-a948-b651152457b3_$_namespace_$_Class_$_method";
    return await window.__TAURI__.core.invoke('ipc_dispatch', { channel, args: [...args] });
}
```

### Pattern 2: Event listener (on* methods, registerBinding)
```javascript
onSomeEvent(callback) {
    window.__TAURI__.event.listen('channel_name', (event) => callback(event.payload));
    return () => { /* unlisten */ };
}
```

### Pattern 3: Sync property (getters that return cached values)
```javascript
getInitialLocale() {
    return "en-US";  // Return the value directly, no IPC
}
```

## The EIPC channel format
All channels follow: `$eipc_message$_a0ef28e9-b3bd-4b5e-a948-b651152457b3_$_{namespace}_$_{class}_$_{method}`

The UUID is always: `a0ef28e9-b3bd-4b5e-a948-b651152457b3`
## Step by step

### Step 1: Read mainView.pretty.js
Find every `exposeInMainWorld(name, object)` call. For each one, document:
- The global name (first argument)
- Every method on the object (second argument)
- What IPC channel each method calls
- Whether it uses invoke (async), on (listener), or sendSync (sync return)
Write this to `docs/GLOBALS_MAP.md` before writing any code.

### Step 2: Write desktop-globals.js
Create `src/desktop-globals.js` that sets all 14 window globals.
- Plain browser JavaScript. No require(). No contextBridge. No Electron APIs.
- Every method that was async invoke → uses window.__TAURI__.core.invoke
- Every method that was sendSync → returns the value directly (hardcode locale strings, return [] for empty arrays, {} for empty objects)
- Every method that was ipcRenderer.on → uses window.__TAURI__.event.listen
- electronIntl.getInitialLocale MUST return "en-US" as a string. Not null. Not {}.
- Any method that returns data claude.ai iterates over MUST return [] not {}

### Step 3: Update lib.rs
Replace the init script. Instead of:
```rust
const ELECTRON_SHIM: &str = include_str!("../../src/electron-shim.js");
const MAIN_VIEW_JS: &str = include_str!("../../electron-extracted/.vite/build/mainView.js");
let init_script = format!("{}\n{}", ELECTRON_SHIM, MAIN_VIEW_JS);
```
Use:
```rust
const DESKTOP_GLOBALS: &str = include_str!("../../src/desktop-globals.js");
// No mainView.js — desktop-globals.js replaces it entirely
```
Keep everything else in lib.rs — ipc_dispatch, MCP host, event emission.
Update build.rs to track desktop-globals.js instead of the old files.

### Step 4: Build and test
```bash
cd src-tauri && cargo clean && cargo build --release
./target/release/claude-desktop
```
Check devtools console. If desktop-globals.js creates the right globals, claude.ai will show Chat/Cowork/Code tabs. If it shows web interface, a global is missing or wrong — check GLOBALS_MAP.md against what you created.

## Rules

1. DO NOT inject mainView.js. It does not run in a browser.
2. DO NOT fake require() or contextBridge. Write native browser JavaScript.
3. Read mainView.pretty.js BEFORE writing any code. Write GLOBALS_MAP.md first.
4. Every method that returns data must return the correct TYPE — strings are strings, arrays are arrays, never return {} where [] is expected.
5. electronIntl.getInitialLocale returns "en-US". This is the #1 cause of crashes.
6. cargo clean before every build to ensure fresh JS embedding.
7. Do not modify claude-mcp-host/. Do not modify the ipc_dispatch Rust code unless adding new channel handlers.
8. Test against real Claude Desktop for comparison — it's at /usr/bin/claude-desktop.

## What success looks like
Claude.ai loads with Chat, Cowork, Code tabs. Settings shows Desktop app section. MCP servers appear. Tool calls work.

## What failure looks like
"Claude will return soon" error page, or plain web interface without desktop tabs.