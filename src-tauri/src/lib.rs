mod bridge;
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
    /// Otomatik duraklatmayla WebView dondurulmuş mu
    asleep: bool,
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
    /// Düzenleme modunda her monitörün üstündeki katman pencereleri (hwnd kullanılmaz)
    editors: Mutex<Vec<Host>>,
    /// Düzenleme sürerken host'lar bu taslağı çizer; kaydedilince yalnızca konum/boyut config'e aktarılır
    edit_draft: Mutex<Option<Config>>,
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

/// Çağıran host'un (ya da düzenleme katmanının) monitörüne düşen widget'lar.
/// Monitörü bulunamayanlar ana monitöre gider. Düzenleme sürerken taslak kullanılır.
#[tauri::command]
fn host_widgets(window: WebviewWindow, state: State<AppState>) -> Vec<Widget> {
    let hosts = state.hosts.lock().unwrap();
    let editors = state.editors.lock().unwrap();
    let Some(me) = hosts.iter().chain(editors.iter()).find(|h| h.label == window.label()) else { return vec![] };
    let known = |m: &String| hosts.iter().any(|h| &h.monitor == m);
    let draft = state.edit_draft.lock().unwrap();
    let config = state.config.lock().unwrap();
    draft
        .as_ref()
        .unwrap_or(&config)
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

#[derive(serde::Serialize)]
struct MonitorInfo {
    name: String,
    primary: bool,
    /// CSS pikseli (widget koordinatlarıyla aynı birim)
    w: f64,
    h: f64,
    scale: f64,
}

#[tauri::command]
fn list_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let primary = app.primary_monitor().map_err(|e| e.to_string())?.and_then(|m| m.name().cloned());
    Ok(app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let name = m.name().cloned().unwrap_or_else(|| format!("monitor-{i}"));
            let scale = m.scale_factor();
            MonitorInfo {
                primary: primary.as_ref() == Some(&name),
                name,
                w: m.size().width as f64 / scale,
                h: m.size().height as f64 / scale,
                scale,
            }
        })
        .collect())
}

#[tauri::command]
/// Arayüzdeki taslak (kaydedilmemiş değişiklikler dahil) dışa aktarılır
fn export_config(path: String, config: Config) -> Result<(), String> {
    config::save(&PathBuf::from(path), &config)
}

/// Dışa aktarılmış dosyayı okur: tam config, widget listesi ya da tek widget kabul edilir.
#[tauri::command]
fn read_widgets_file(path: String) -> Result<Vec<Widget>, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if let Ok(c) = serde_json::from_str::<Config>(&text) {
        if !c.widgets.is_empty() {
            return Ok(c.widgets);
        }
    }
    if let Ok(list) = serde_json::from_str::<Vec<Widget>>(&text) {
        return Ok(list);
    }
    serde_json::from_str::<Widget>(&text).map(|w| vec![w]).map_err(|_| "dosyada widget bulunamadı".into())
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
            asleep: false,
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

/// Saniyede bir otomatik duraklatmayı, 3 saniyede bir gömmenin ve monitör düzeninin sağlamlığını denetler.
fn spawn_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        for tick in 0u64.. {
            std::thread::sleep(Duration::from_secs(1));
            #[cfg(windows)]
            auto_pause(&app);
            if tick % 3 == 2 {
                check_embedding(&app);
            }
        }
    });
}

/// Görünmeyen monitörün WebView'ı dondurulur: çizim durur, JS askıya alınır ama durum korunur.
/// (Pencereyi gizlemek WebView2'yi yavaşlatmıyor; ölçümle doğrulandı.)
#[cfg(windows)]
fn auto_pause(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.config.lock().unwrap().settings.clone();
    let editing = state.edit_draft.lock().unwrap().is_some();
    let locked = desktop::session_locked();
    let battery = settings.pause_on_battery && desktop::on_battery();
    let covered = if settings.pause_when_covered { desktop::covered_monitor() } else { None };

    let mut changes = Vec::new();
    for h in state.hosts.lock().unwrap().iter_mut() {
        let want = !editing && (locked || battery || covered.as_deref() == Some(h.monitor.as_str()));
        if want != h.asleep {
            h.asleep = want;
            changes.push((h.label.clone(), want));
        }
    }
    // Kilidi bırakıp uygula: with_webview ana iş parçacığına gider, orada host_widgets aynı kilidi ister
    for (label, asleep) in changes {
        let why = if locked { "kilit" } else if battery { "pil" } else if asleep { "üstü kapalı" } else { "görünür" };
        println!("{label}: {} ({why})", if asleep { "donduruldu" } else { "uyandırıldı" });
        set_webview_active(app, &label, !asleep);
    }
}

#[cfg(windows)]
fn set_webview_active(app: &AppHandle, label: &str, active: bool) {
    use webview2_com::{Microsoft::Web::WebView2::Win32::ICoreWebView2_3, TrySuspendCompletedHandler};
    use windows::core::Interface;
    let Some(win) = app.get_webview_window(label) else { return };
    let _ = win.with_webview(move |wv| unsafe {
        let c = wv.controller();
        // Görünür yapmak askıyı da otomatik kaldırır
        let _ = c.SetIsVisible(active);
        if !active {
            if let Ok(core3) = c.CoreWebView2().and_then(|core| core.cast::<ICoreWebView2_3>()) {
                let _ = core3.TrySuspend(&TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(()))));
            }
        }
    });
}

fn check_embedding(app: &AppHandle) {
    {
        let app = app.clone();
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
    }
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

// ---------- düzenleme modu ----------

/// Her monitörün üstüne, ikonların ve pencerelerin önünde etkileşimli bir katman açar.
/// async olmalı: Windows'ta senkron komut içinde pencere oluşturmak ana iş parçacığını kilitler.
#[tauri::command]
async fn start_edit(app: AppHandle) -> Result<(), String> {
    open_editors(&app)
}

fn open_editors(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut draft = state.edit_draft.lock().unwrap();
        if draft.is_some() {
            return Ok(());
        }
        *draft = Some(state.config.lock().unwrap().clone());
    }
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let targets: Vec<(String, bool)> = state.hosts.lock().unwrap().iter().map(|h| (h.monitor.clone(), h.primary)).collect();
    let mut editors = Vec::new();
    for (i, (name, primary)) in targets.into_iter().enumerate() {
        let Some(m) = monitors.iter().find(|m| m.name() == Some(&name)).or(monitors.get(i)) else { continue };
        let label = format!("edit-{i}");
        let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("edit.html".into()))
            .title("Masaüstü Widget — düzenleme")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .resizable(false)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?;
        let _ = win.set_position(*m.position());
        let _ = win.set_size(*m.size());
        let _ = win.show();
        let _ = win.set_focus();
        editors.push(Host {
            label,
            monitor: name,
            primary,
            #[cfg(windows)]
            hwnd: 0,
            asleep: false,
        });
    }
    *state.editors.lock().unwrap() = editors;
    let _ = app.emit("edit-mode", true);
    Ok(())
}

/// Sürükleme/boyutlandırma sırasında taslağı günceller; host'lar canlı olarak yeni konuma geçer.
#[tauri::command]
fn edit_move(app: AppHandle, id: String, x: f64, y: f64, w: f64, h: f64) {
    let state = app.state::<AppState>();
    if let Some(d) = state.edit_draft.lock().unwrap().as_mut() {
        if let Some(wd) = d.widgets.iter_mut().find(|wd| wd.id == id) {
            (wd.x, wd.y, wd.w, wd.h) = (x, y, w, h);
        }
    }
    let _ = app.emit("config-changed", ());
}

/// Kaydedilirse yalnızca konum/boyut güncel config'e aktarılır; düzenleme sırasında
/// yönetim penceresinden yapılmış başka değişiklikler ezilmez.
#[tauri::command]
async fn end_edit(app: AppHandle, save: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let Some(draft) = state.edit_draft.lock().unwrap().take() else { return Ok(()) };
    let mut result = Ok(());
    if save {
        let mut config = state.config.lock().unwrap();
        for w in config.widgets.iter_mut() {
            if let Some(d) = draft.widgets.iter().find(|d| d.id == w.id) {
                (w.x, w.y, w.w, w.h) = (d.x, d.y, d.w, d.h);
            }
        }
        result = config::save(&state.cfg_path, &config);
    }
    for e in state.editors.lock().unwrap().drain(..) {
        if let Some(w) = app.get_webview_window(&e.label) {
            let _ = w.destroy();
        }
    }
    let _ = app.emit("config-changed", ());
    let _ = app.emit("edit-mode", false);
    result
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
    let edit = MenuItemBuilder::with_id("edit", "Düzenleme modu").build(app)?;
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
            "edit" => {
                if let Err(e) = open_editors(app) {
                    eprintln!("düzenleme modu açılamadı: {e}");
                }
            }
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            config_path,
            is_paused,
            host_widgets,
            list_monitors,
            export_config,
            read_widgets_file,
            start_edit,
            edit_move,
            end_edit,
            bridge::widget_fetch,
            bridge::system_info
        ])
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
                editors: Mutex::new(Vec::new()),
                edit_draft: Mutex::new(None),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("masaustu-widget-test-{}-{name}", std::process::id()))
    }

    fn widget(id: &str) -> Widget {
        Widget { id: id.into(), name: format!("W {id}"), html: "<b>ş</b>".into(), ..Default::default() }
    }

    #[test]
    fn disa_aktarilan_dosya_geri_okunur() {
        let p = tmp("full.json");
        let c = Config { widgets: vec![widget("a"), widget("b")], ..Default::default() };
        export_config(p.display().to_string(), c.clone()).unwrap();
        let back = read_widgets_file(p.display().to_string()).unwrap();
        assert_eq!(back, c.widgets);
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn liste_ve_tek_widget_bicimleri_kabul_edilir() {
        let p = tmp("list.json");
        std::fs::write(&p, serde_json::to_string(&vec![widget("x"), widget("y")]).unwrap()).unwrap();
        assert_eq!(read_widgets_file(p.display().to_string()).unwrap().len(), 2);
        std::fs::write(&p, serde_json::to_string(&widget("z")).unwrap()).unwrap();
        assert_eq!(read_widgets_file(p.display().to_string()).unwrap()[0].id, "z");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn eksik_alanlar_varsayilanla_dolar() {
        let p = tmp("partial.json");
        std::fs::write(&p, r#"{"widgets":[{"id":"k","html":"<i>hi</i>"}]}"#).unwrap();
        let w = &read_widgets_file(p.display().to_string()).unwrap()[0];
        assert_eq!((w.w, w.h, w.opacity, w.enabled), (320.0, 180.0, 1.0, true));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn bozuk_dosya_hata_verir() {
        let p = tmp("bad.json");
        std::fs::write(&p, "bu json değil").unwrap();
        assert!(read_widgets_file(p.display().to_string()).is_err());
        let _ = std::fs::remove_file(p);
    }
}
