mod config;
#[cfg(windows)]
mod desktop;

use config::{Config, Widget};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;

struct Host {
    label: String,
    monitor: String,
    primary: bool,
    #[cfg(windows)]
    hwnd: isize,
}

/// Tepsideki "Duraklat" işaretini dışarıdan (CLI) değişince eşitlemek için
struct PauseItem(tauri::menu::CheckMenuItem<tauri::Wry>);

struct AppState {
    cfg_path: PathBuf,
    config: Mutex<Config>,
    hosts: Mutex<Vec<Host>>,
    paused: AtomicBool,
    /// Yeniden kurulumda etiket çakışmasın diye her kuşak farklı etiket alır
    generation: AtomicU32,
    monitor_sig: Mutex<String>,
}

// ---------- komutlar ----------

#[tauri::command]
fn get_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<AppState>, config: Config) -> Result<(), String> {
    config::save(&state.cfg_path, &config)?;
    *state.config.lock().unwrap() = config;
    app.emit("config-changed", ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn config_path(state: State<AppState>) -> String {
    state.cfg_path.display().to_string()
}

#[tauri::command]
fn is_paused(state: State<AppState>) -> bool {
    state.paused.load(Ordering::Relaxed)
}

/// Çağıran host'un monitörüne düşen widget'lar. Monitörü bulunamayanlar ana monitöre gider.
#[tauri::command]
fn host_widgets(window: WebviewWindow, state: State<AppState>) -> Vec<Widget> {
    let hosts = state.hosts.lock().unwrap();
    let Some(me) = hosts.iter().find(|h| h.label == window.label()) else { return vec![] };
    let known = |m: &String| hosts.iter().any(|h| &h.monitor == m);
    state
        .config
        .lock()
        .unwrap()
        .widgets
        .iter()
        .filter(|w| w.enabled)
        .filter(|w| match &w.monitor {
            Some(m) if known(m) => *m == me.monitor,
            _ => me.primary,
        })
        .cloned()
        .collect()
}

// ---------- host pencereleri ----------

fn monitor_signature(app: &AppHandle) -> String {
    app.available_monitors()
        .map(|ms| {
            ms.iter()
                .map(|m| format!("{:?}{:?}{:?}{}", m.name(), m.position(), m.size(), m.scale_factor()))
                .collect::<Vec<_>>()
                .join("|")
        })
        .unwrap_or_default()
}

fn open_hosts(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let gen = state.generation.fetch_add(1, Ordering::Relaxed);
    let primary = app.primary_monitor()?.and_then(|m| m.name().cloned());
    let mut hosts = Vec::new();

    for (i, m) in app.available_monitors()?.iter().enumerate() {
        let name = m.name().cloned().unwrap_or_else(|| format!("monitor-{i}"));
        let (pos, size) = (m.position(), m.size());
        let label = format!("host-{gen}-{i}");
        let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("host.html".into()))
            .title("masaustu-widget host")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .build()?;
        // Gömmeden önce hedef monitöre taşı ki WebView doğru DPI ile başlasın
        win.set_position(*pos)?;

        #[cfg(windows)]
        let hwnd = {
            let hwnd = win.hwnd()?;
            match desktop::embed(hwnd, pos.x, pos.y, size.width as i32, size.height as i32) {
                Ok(layout) => println!("{label} ({name}) gömüldü: {layout:?}"),
                Err(e) => eprintln!("{label} ({name}) gömülemedi: {e}"),
            }
            if state.paused.load(Ordering::Relaxed) {
                desktop::set_visible(hwnd, false);
            }
            hwnd.0 as isize
        };

        hosts.push(Host {
            label,
            primary: primary.as_ref() == Some(&name),
            monitor: name,
            #[cfg(windows)]
            hwnd,
        });
    }
    // Hiçbiri ana monitör olarak işaretlenmediyse ilki üstlensin
    if !hosts.iter().any(|h| h.primary) {
        if let Some(h) = hosts.first_mut() {
            h.primary = true;
        }
    }
    *state.hosts.lock().unwrap() = hosts;
    *state.monitor_sig.lock().unwrap() = monitor_signature(app);
    Ok(())
}

fn rebuild_hosts(app: &AppHandle) {
    let old: Vec<String> = app.state::<AppState>().hosts.lock().unwrap().drain(..).map(|h| h.label).collect();
    for label in old {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.destroy();
        }
    }
    if let Err(e) = open_hosts(app) {
        eprintln!("host'lar kurulamadı: {e}");
    }
}

/// Explorer yeniden başlarsa ya da monitör düzeni değişirse host'ları yeniden kurar.
fn spawn_watcher(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(3));
        let state = app.state::<AppState>();
        let sig_changed = *state.monitor_sig.lock().unwrap() != monitor_signature(&app);
        #[cfg(windows)]
        let detached = state.hosts.lock().unwrap().iter().any(|h| {
            !desktop::still_embedded(windows::Win32::Foundation::HWND(h.hwnd as *mut _))
        });
        #[cfg(not(windows))]
        let detached = false;
        #[cfg(windows)]
        if !detached {
            for h in state.hosts.lock().unwrap().iter() {
                desktop::keep_below_icons(windows::Win32::Foundation::HWND(h.hwnd as *mut _));
            }
        }
        if sig_changed || detached {
            println!("yeniden kurulum (monitör değişti: {sig_changed}, gömme koptu: {detached})");
            rebuild_hosts(&app);
        }
    });
}

fn set_paused(app: &AppHandle, paused: bool) {
    let state = app.state::<AppState>();
    state.paused.store(paused, Ordering::Relaxed);
    #[cfg(windows)]
    for h in state.hosts.lock().unwrap().iter() {
        desktop::set_visible(windows::Win32::Foundation::HWND(h.hwnd as *mut _), !paused);
    }
    if let Some(item) = app.try_state::<PauseItem>() {
        let _ = item.0.set_checked(paused);
    }
    let _ = app.emit("paused", paused);
}

// ---------- yönetim penceresi ve tepsi ----------

fn open_manager(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Masaüstü Widget")
        .inner_size(1000.0, 700.0)
        .center()
        .build();
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let open = MenuItemBuilder::with_id("open", "Aç").build(app)?;
    let edit = MenuItemBuilder::with_id("edit", "Düzenleme modu (yakında)").enabled(false).build(app)?;
    let pause = CheckMenuItemBuilder::with_id("pause", "Duraklat").build(app)?;
    let autostart = CheckMenuItemBuilder::with_id("autostart", "Başlangıçta çalıştır")
        .checked(autostart_on)
        .build(app)?;
    app.manage(PauseItem(pause.clone()));
    let quit = MenuItemBuilder::with_id("quit", "Çıkış").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &edit])
        .separator()
        .items(&[&pause, &autostart])
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Masaüstü Widget")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, e| match e.id().as_ref() {
            "open" => open_manager(app),
            "pause" => set_paused(app, pause.is_checked().unwrap_or(false)),
            "autostart" => {
                let al = app.autolaunch();
                let r = if autostart.is_checked().unwrap_or(false) { al.enable() } else { al.disable() };
                if let Err(e) = r {
                    eprintln!("autostart ayarlanamadı: {e}");
                }
                let _ = autostart.set_checked(al.is_enabled().unwrap_or(false));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                open_manager(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // İkinci kez başlatılırsa yeni kopya açma: --pause/--resume ilet, yoksa yönetim penceresini göster
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            match argv.get(1).map(String::as_str) {
                Some("--pause") => set_paused(app, true),
                Some("--resume") => set_paused(app, false),
                _ => open_manager(app),
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_config, save_config, config_path, is_paused, host_widgets])
        .setup(|app| {
            let cfg_path = app.path().app_config_dir()?.join("widgets.json");
            let first_run = !cfg_path.exists();
            app.manage(AppState {
                config: Mutex::new(config::load(&cfg_path)),
                cfg_path,
                hosts: Mutex::new(Vec::new()),
                paused: AtomicBool::new(false),
                generation: AtomicU32::new(0),
                monitor_sig: Mutex::new(String::new()),
            });
            // Geliştirme sürümünde debug exe'yi başlangıca kaydetmemek için yalnızca release'de
            if first_run && !cfg!(debug_assertions) {
                let _ = app.autolaunch().enable();
            }
            build_tray(app.handle())?;
            open_hosts(app.handle())?;
            spawn_watcher(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, e| {
            // Pencereler kapansa da (Explorer çökmesi, yönetim penceresi) tepside yaşamaya devam et;
            // yalnızca app.exit() (kod Some) çıkar
            if let RunEvent::ExitRequested { code: None, api, .. } = e {
                api.prevent_exit();
            }
        });
}
