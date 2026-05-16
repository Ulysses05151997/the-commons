use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::webview::{PageLoadPayload, WebviewBuilder};
use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, State, Webview, WebviewUrl, Wry};

const TOOLBAR_HEIGHT: f64 = 76.0;
const HOME_URL: &str = "https://computecommons.cloud";
const FALLBACK_HTML: &str = include_str!("../../src/fallback.html");

// --- Types ---

#[derive(Clone, Serialize, Deserialize)]
struct Favorite {
    title: String,
    url: String,
}

#[derive(Clone, Serialize)]
struct TabInfo {
    id: usize,
    title: String,
    url: String,
    active: bool,
}

struct Tab {
    id: usize,
    label: String,
    title: String,
    url: String,
}

struct BrowserState {
    tabs: Vec<Tab>,
    active_tab: usize,
    next_id: usize,
}

type StateHandle = Mutex<BrowserState>;

// --- Helpers ---

fn favorites_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = PathBuf::from(home).join(".config/the-commons");
    let _ = fs::create_dir_all(&dir);
    dir.join("favorites.json")
}

fn encode_query(q: &str) -> String {
    let mut out = String::with_capacity(q.len() * 3);
    for b in q.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

fn tab_infos(state: &BrowserState) -> Vec<TabInfo> {
    state
        .tabs
        .iter()
        .map(|t| TabInfo {
            id: t.id,
            title: t.title.clone(),
            url: t.url.clone(),
            active: t.id == state.active_tab,
        })
        .collect()
}

fn emit_tabs(app: &tauri::AppHandle, state: &BrowserState) {
    let _ = app.emit_to("toolbar", "tabs-updated", &tab_infos(state));
}

fn active_label(state: &BrowserState) -> Option<String> {
    state
        .tabs
        .iter()
        .find(|t| t.id == state.active_tab)
        .map(|t| t.label.clone())
}

fn cleanup_browser_data() {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };
    let base = PathBuf::from(&home);
    let _ = fs::remove_dir_all(base.join(".local/share/org.computecommons.browser"));
    let _ = fs::remove_dir_all(base.join(".cache/org.computecommons.browser"));
}

fn check_home_reachable() -> bool {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        use std::net::ToSocketAddrs;
        let ok = "computecommons.cloud:443"
            .to_socket_addrs()
            .map(|mut a| a.next().is_some())
            .unwrap_or(false);
        let _ = tx.send(ok);
    });
    rx.recv_timeout(std::time::Duration::from_millis(2000))
        .unwrap_or(false)
}

fn fallback_url() -> String {
    let path = std::env::temp_dir().join("commons-fallback.html");
    let _ = fs::write(&path, FALLBACK_HTML);
    format!("file://{}", path.display())
}

fn create_browser_webview(
    window: &tauri::Window,
    app_handle: &tauri::AppHandle,
    tab_id: usize,
    label: &str,
    url_str: &str,
) -> Result<Webview<Wry>, String> {
    let parsed_url = url_str
        .parse()
        .unwrap_or_else(|_| "about:blank".parse().unwrap());

    let ah1 = app_handle.clone();
    let ah2 = app_handle.clone();

    window
        .add_child(
            WebviewBuilder::new(label, WebviewUrl::External(parsed_url))
                .on_page_load(
                    move |_wv: Webview<Wry>, payload: PageLoadPayload<'_>| {
                        let url = payload.url().to_string();
                        if let Some(state) = ah1.try_state::<StateHandle>() {
                            let mut s = state.lock().unwrap();
                            if let Some(tab) = s.tabs.iter_mut().find(|t| t.id == tab_id) {
                                tab.url = url.clone();
                            }
                            if s.active_tab == tab_id {
                                let _ = ah1.emit_to("toolbar", "url-changed", &url);
                            }
                        }
                    },
                )
                .on_document_title_changed(move |wv: Webview<Wry>, title: String| {
                    if let Some(state) = ah2.try_state::<StateHandle>() {
                        let mut s = state.lock().unwrap();
                        if let Some(tab) = s.tabs.iter_mut().find(|t| t.id == tab_id) {
                            tab.title = title.clone();
                        }
                        if s.active_tab == tab_id {
                            let win = wv.window();
                            let display = if title.is_empty() {
                                "The Commons".to_string()
                            } else {
                                format!("{} \u{2014} The Commons", title)
                            };
                            let _ = win.set_title(&display);
                        }
                        emit_tabs(&ah2, &s);
                    }
                }),
            LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
            LogicalSize::new(1200.0, 800.0 - TOOLBAR_HEIGHT),
        )
        .map_err(|e| e.to_string())
}

// --- Commands: Favorites ---

#[tauri::command]
fn save_favorite(title: String, url: String) -> Result<(), String> {
    let path = favorites_path();
    let mut favs: Vec<Favorite> = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if !favs.iter().any(|f| f.url == url) {
        favs.push(Favorite { title, url });
        fs::write(
            path,
            serde_json::to_string_pretty(&favs).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn remove_favorite(url: String) -> Result<(), String> {
    let path = favorites_path();
    let mut favs: Vec<Favorite> = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    favs.retain(|f| f.url != url);
    fs::write(
        path,
        serde_json::to_string_pretty(&favs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn load_favorites() -> Vec<Favorite> {
    let path = favorites_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// --- Commands: Smart Navigation ---

#[tauri::command]
async fn smart_navigate(input: String) -> String {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return HOME_URL.to_string();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed;
    }
    if trimmed.contains('.') && !trimmed.contains(' ') {
        return format!("https://{}", trimmed);
    }
    if !trimmed.contains(' ') && !trimmed.contains('?') {
        let word = trimmed.to_lowercase();
        use std::net::ToSocketAddrs;
        if format!("www.{}.com:443", word).to_socket_addrs().is_ok() {
            return format!("https://www.{}.com", word);
        }
        return format!("https://lite.duckduckgo.com/lite/?q={}", encode_query(&trimmed));
    }
    format!("https://lite.duckduckgo.com/lite/?q={}", encode_query(&trimmed))
}

// --- Commands: Navigation (operates on active tab) ---

#[tauri::command]
fn navigate(app: tauri::AppHandle, state: State<'_, StateHandle>, url: String) {
    let label = active_label(&state.lock().unwrap());
    if let Some(label) = label {
        if let Some(wv) = app.get_webview(&label) {
            let parsed = url.parse().unwrap_or_else(|_| "about:blank".parse().unwrap());
            let _ = wv.navigate(parsed);
        }
    }
}

#[tauri::command]
fn go_back(app: tauri::AppHandle, state: State<'_, StateHandle>) {
    let label = active_label(&state.lock().unwrap());
    if let Some(label) = label {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.eval("window.history.back()");
        }
    }
}

#[tauri::command]
fn go_forward(app: tauri::AppHandle, state: State<'_, StateHandle>) {
    let label = active_label(&state.lock().unwrap());
    if let Some(label) = label {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.eval("window.history.forward()");
        }
    }
}

#[tauri::command]
fn refresh(app: tauri::AppHandle, state: State<'_, StateHandle>) {
    let label = active_label(&state.lock().unwrap());
    if let Some(label) = label {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.eval("window.location.reload()");
        }
    }
}

#[tauri::command]
fn stop_loading(app: tauri::AppHandle, state: State<'_, StateHandle>) {
    let label = active_label(&state.lock().unwrap());
    if let Some(label) = label {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.eval("window.stop()");
        }
    }
}

// --- Commands: Tabs ---

#[tauri::command]
fn get_tabs(state: State<'_, StateHandle>) -> Vec<TabInfo> {
    tab_infos(&state.lock().unwrap())
}

#[tauri::command]
fn new_tab(
    app: tauri::AppHandle,
    state: State<'_, StateHandle>,
    url: Option<String>,
) -> Result<usize, String> {
    let tab_url = url.unwrap_or_else(|| HOME_URL.to_string());
    let (id, label, old_label) = {
        let mut s = state.lock().unwrap();
        let id = s.next_id;
        s.next_id += 1;
        let label = format!("browser_{}", id);
        let old = active_label(&s);
        s.tabs.push(Tab {
            id,
            label: label.clone(),
            title: "New Tab".to_string(),
            url: tab_url.clone(),
        });
        s.active_tab = id;
        (id, label, old)
    };

    if let Some(old) = old_label {
        if let Some(wv) = app.get_webview(&old) {
            let _ = wv.with_webview(|platform| {
                use gtk::prelude::*;
                platform.inner().hide();
            });
        }
    }

    let window = app.get_window("main").ok_or("Window not found")?;
    let browser = create_browser_webview(&window, &app, id, &label, &tab_url)?;

    let _ = browser.with_webview(|platform| {
        use gtk::prelude::*;
        let wv = platform.inner();
        if let Some(parent) = wv.parent() {
            if let Some(vbox) = parent.downcast_ref::<gtk::Box>() {
                vbox.set_child_packing(&wv, true, true, 0, gtk::PackType::Start);
            }
        }
    });

    {
        let s = state.lock().unwrap();
        emit_tabs(&app, &s);
        let _ = app.emit_to("toolbar", "url-changed", &tab_url);
    }

    Ok(id)
}

#[tauri::command]
fn close_tab(
    app: tauri::AppHandle,
    state: State<'_, StateHandle>,
    tab_id: usize,
) -> Result<(), String> {
    let (label_to_close, switch_to) = {
        let mut s = state.lock().unwrap();
        let pos = s
            .tabs
            .iter()
            .position(|t| t.id == tab_id)
            .ok_or("Tab not found")?;

        let label = s.tabs[pos].label.clone();
        s.tabs.remove(pos);

        if s.tabs.is_empty() {
            if let Some(window) = app.get_window("main") {
                let _ = window.close();
            }
            return Ok(());
        }

        let switch = if s.active_tab == tab_id {
            let new_pos = if pos < s.tabs.len() { pos } else { s.tabs.len() - 1 };
            let id = s.tabs[new_pos].id;
            let lbl = s.tabs[new_pos].label.clone();
            let u = s.tabs[new_pos].url.clone();
            let t = s.tabs[new_pos].title.clone();
            s.active_tab = id;
            Some((lbl, u, t))
        } else {
            None
        };

        (label, switch)
    };

    if let Some(wv) = app.get_webview(&label_to_close) {
        let _ = wv.close();
    }

    if let Some((new_label, new_url, new_title)) = switch_to {
        if let Some(wv) = app.get_webview(&new_label) {
            let _ = wv.with_webview(|platform| {
                use gtk::prelude::*;
                platform.inner().show();
            });
        }
        let _ = app.emit_to("toolbar", "url-changed", &new_url);
        if let Some(window) = app.get_window("main") {
            let display = if new_title.is_empty() {
                "The Commons".to_string()
            } else {
                format!("{} \u{2014} The Commons", new_title)
            };
            let _ = window.set_title(&display);
        }
    }

    {
        let s = state.lock().unwrap();
        emit_tabs(&app, &s);
    }

    Ok(())
}

#[tauri::command]
fn switch_tab(
    app: tauri::AppHandle,
    state: State<'_, StateHandle>,
    tab_id: usize,
) -> Result<(), String> {
    let (old_label, new_label, new_url, new_title) = {
        let mut s = state.lock().unwrap();
        if s.active_tab == tab_id {
            return Ok(());
        }

        let old = active_label(&s);
        let new = s
            .tabs
            .iter()
            .find(|t| t.id == tab_id)
            .ok_or("Tab not found")?;

        let new_label = new.label.clone();
        let new_url = new.url.clone();
        let new_title = new.title.clone();
        s.active_tab = tab_id;

        (old, new_label, new_url, new_title)
    };

    if let Some(old) = old_label {
        if let Some(wv) = app.get_webview(&old) {
            let _ = wv.with_webview(|platform| {
                use gtk::prelude::*;
                platform.inner().hide();
            });
        }
    }

    if let Some(wv) = app.get_webview(&new_label) {
        let _ = wv.with_webview(|platform| {
            use gtk::prelude::*;
            platform.inner().show();
        });
    }

    let _ = app.emit_to("toolbar", "url-changed", &new_url);

    if let Some(window) = app.get_window("main") {
        let display = if new_title.is_empty() {
            "The Commons".to_string()
        } else {
            format!("{} \u{2014} The Commons", new_title)
        };
        let _ = window.set_title(&display);
    }

    {
        let s = state.lock().unwrap();
        emit_tabs(&app, &s);
    }

    Ok(())
}

// --- Entry Point ---

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(BrowserState {
            tabs: Vec::new(),
            active_tab: 0,
            next_id: 0,
        }))
        .invoke_handler(tauri::generate_handler![
            navigate,
            go_back,
            go_forward,
            refresh,
            stop_loading,
            smart_navigate,
            save_favorite,
            remove_favorite,
            load_favorites,
            new_tab,
            close_tab,
            switch_tab,
            get_tabs,
        ])
        .setup(|app| {
            cleanup_browser_data();

            let initial_url = if check_home_reachable() {
                HOME_URL.to_string()
            } else {
                fallback_url()
            };

            let window = tauri::window::WindowBuilder::new(app, "main")
                .title("The Commons")
                .inner_size(1200.0, 800.0)
                .build()?;

            let toolbar = window.add_child(
                WebviewBuilder::new("toolbar", WebviewUrl::App("index.html".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1200.0, TOOLBAR_HEIGHT),
            )?;

            toolbar.with_webview(|platform| {
                use gtk::prelude::*;
                let wv = platform.inner();
                wv.set_size_request(-1, TOOLBAR_HEIGHT as i32);
                if let Some(parent) = wv.parent() {
                    if let Some(vbox) = parent.downcast_ref::<gtk::Box>() {
                        vbox.set_child_packing(&wv, false, false, 0, gtk::PackType::Start);
                    }
                }
            })?;

            let app_handle = app.handle().clone();
            let browser =
                create_browser_webview(&window, &app_handle, 0, "browser_0", &initial_url)
                    .map_err(|e| Box::<dyn std::error::Error>::from(e))?;

            let _ = browser.with_webview(|platform| {
                use gtk::prelude::*;
                let wv = platform.inner();
                if let Some(parent) = wv.parent() {
                    if let Some(vbox) = parent.downcast_ref::<gtk::Box>() {
                        vbox.set_child_packing(&wv, true, true, 0, gtk::PackType::Start);
                    }
                }
            });

            if let Some(state) = app.try_state::<StateHandle>() {
                let mut s = state.lock().unwrap();
                s.tabs.push(Tab {
                    id: 0,
                    label: "browser_0".to_string(),
                    title: "Home".to_string(),
                    url: initial_url,
                });
                s.active_tab = 0;
                s.next_id = 1;
            }

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                cleanup_browser_data();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running application");
}
