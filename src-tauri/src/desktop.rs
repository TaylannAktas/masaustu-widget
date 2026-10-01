//! Pencereyi masaüstü ikonlarının arkasına (duvar kağıdı katmanına) gömer.
//!
//! İki yapı var:
//! - Klasik (24H2 öncesi): Progman'a 0x052C gönderilince ikonları tutan pencerenin
//!   arkasında ayrı bir üst düzey WorkerW oluşur, pencere onun içine konur.
//! - 24H2 ve sonrası: SHELLDLL_DefView (ikonlar) ve WorkerW (duvar kağıdı) Progman'ın
//!   çocuklarıdır. Pencere Progman'ın çocuğu yapılıp DefView'in hemen altına sıralanır.

use windows::core::{w, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Debug)]
pub enum Layout {
    Classic { workerw: HWND },
    Modern { progman: HWND, defview: HWND },
}

fn find_layout() -> Option<Layout> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), None).ok()?;
        // WorkerW'nin oluşmasını tetikle
        let mut res = 0usize;
        let _ = SendMessageTimeoutW(progman, 0x052C, WPARAM(0xD), LPARAM(0x1), SMTO_NORMAL, 1000, Some(&mut res));
        let _ = SendMessageTimeoutW(progman, 0x052C, WPARAM(0), LPARAM(0), SMTO_NORMAL, 1000, Some(&mut res));

        // 24H2+: DefView doğrudan Progman'ın çocuğu
        if let Ok(defview) = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None) {
            if FindWindowExW(Some(progman), None, w!("WorkerW"), None).is_ok() {
                return Some(Layout::Modern { progman, defview });
            }
        }

        // Klasik: DefView'i barındıran üst düzey pencereden sonraki WorkerW
        let mut workerw = HWND::default();
        unsafe extern "system" fn cb(top: HWND, out: LPARAM) -> BOOL {
            if FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), None).is_ok() {
                if let Ok(ww) = FindWindowExW(None, Some(top), w!("WorkerW"), None) {
                    *(out.0 as *mut HWND) = ww;
                }
            }
            BOOL(1)
        }
        let _ = EnumWindows(Some(cb), LPARAM(&mut workerw as *mut HWND as isize));
        if !workerw.is_invalid() {
            return Some(Layout::Classic { workerw });
        }
        None
    }
}

/// `x, y, w, h` sanal ekran koordinatlarında fiziksel piksel.
pub fn embed(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> Result<Layout, String> {
    let layout = find_layout().ok_or("masaüstü katmanı bulunamadı")?;
    unsafe {
        // Çocuk koordinatları ebeveynin sol üstüne göre; ebeveyn sanal ekranın tamamını kaplar
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
        match layout {
            Layout::Classic { workerw } => {
                SetParent(hwnd, Some(workerw)).map_err(|e| e.to_string())?;
                SetWindowPos(hwnd, None, x - vx, y - vy, w, h, flags | SWP_NOZORDER).map_err(|e| e.to_string())?;
            }
            Layout::Modern { progman, defview } => {
                SetParent(hwnd, Some(progman)).map_err(|e| e.to_string())?;
                // DefView'in hemen altına: ikonlar üstte, duvar kağıdı (WorkerW) altta
                SetWindowPos(hwnd, Some(defview), x - vx, y - vy, w, h, flags).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(layout)
}

/// Explorer yeniden başladıysa ebeveyn yok olur ya da Progman değişir.
/// Mesaj göndermez, gözcü döngüsünde sık çağrılabilir.
pub fn still_embedded(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return false;
        }
        // GetParent WS_CHILD olmayan pencerede sahibi döndürür, gerçek ebeveyn için GetAncestor
        let parent = GetAncestor(hwnd, GA_PARENT);
        if !IsWindow(Some(parent)).as_bool() {
            return false;
        }
        match FindWindowW(w!("Progman"), None) {
            // Modern yapıda ebeveyn Progman'ın kendisi, klasikte ayrı bir WorkerW
            Ok(progman) => parent == progman || GetAncestor(parent, GA_PARENT) == GetDesktopWindow(),
            Err(_) => false,
        }
    }
}

/// Tauri'nin show()'u pencereyi etkinleştirebilir; gömülü pencerede odak çalmamak için doğrudan Win32.
pub fn set_visible(hwnd: HWND, visible: bool) {
    unsafe {
        let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
    }
    // Gösterme pencereyi kardeşlerin en üstüne, yani ikonların önüne taşıyabiliyor
    if visible {
        keep_below_icons(hwnd);
    }
}

/// Modern yapıda pencere DefView'in (ikonlar) altında kalmalı; üste çıktıysa geri indirir.
/// Klasik yapıda ikonlar ayrı üst düzey pencerede olduğu için bir şey yapmaz.
pub fn keep_below_icons(hwnd: HWND) {
    unsafe {
        let parent = GetAncestor(hwnd, GA_PARENT);
        let Ok(defview) = FindWindowExW(Some(parent), None, w!("SHELLDLL_DefView"), None) else { return };
        // Önceki kardeşlerde DefView varsa sıra doğru
        let mut prev = GetWindow(hwnd, GW_HWNDPREV);
        while let Ok(p) = prev {
            if p == defview {
                return;
            }
            prev = GetWindow(p, GW_HWNDPREV);
        }
        let _ = SetWindowPos(hwnd, Some(defview), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

// ---------- otomatik duraklatma algılayıcıları ----------

/// Öndeki pencere bir monitörü tamamen kaplıyorsa (büyütülmüş ya da tam ekran) o monitörün adı.
/// Masaüstünün kendisi, görev çubuğu ve bu uygulamanın pencereleri sayılmaz.
pub fn covered_monitor() -> Option<String> {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::System::Threading::GetCurrentProcessId;
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || !IsWindowVisible(fg).as_bool() || IsIconic(fg).as_bool() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return None;
        }
        let mut class = [0u16; 64];
        let n = GetClassNameW(fg, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return None;
        }

        let mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(mon, &mut info as *mut _ as *mut MONITORINFO).as_bool() {
            return None;
        }
        let mut r = Default::default();
        GetWindowRect(fg, &mut r).ok()?;
        let m = info.monitorInfo.rcMonitor;
        let fullscreen = r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom;
        if !(fullscreen || IsZoomed(fg).as_bool()) {
            return None;
        }
        let len = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
        Some(String::from_utf16_lossy(&info.szDevice[..len]))
    }
}

/// Kilit ekranında girdi masaüstü Winlogon'a geçer; normal süreç onu açamaz.
pub fn session_locked() -> bool {
    use windows::Win32::System::StationsAndDesktops::*;
    unsafe {
        match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_ACCESS_FLAGS(0x0001)) {
            Ok(d) => {
                let _ = CloseDesktop(d);
                false
            }
            Err(_) => true,
        }
    }
}

pub fn on_battery() -> bool {
    use windows::Win32::System::Power::*;
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).is_ok() && s.ACLineStatus == 0 }
}
