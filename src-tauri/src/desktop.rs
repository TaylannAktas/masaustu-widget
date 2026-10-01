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
