//! Widget köprüsü: yalıtılmış iframe'lerin yapamadığı işler (CORS'suz HTTP, sistem bilgisi).
//! Widget → postMessage → host sayfası → bu komutlar.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex, time::Duration};

#[derive(Deserialize)]
pub struct FetchRequest {
    url: String,
    #[serde(default = "get")]
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<String>,
}

fn get() -> String {
    "GET".into()
}

#[derive(Serialize)]
pub struct FetchResponse {
    status: u16,
    headers: HashMap<String, String>,
    text: String,
}

/// 5 MB'tan büyük yanıtlar kesilir; widget'lar için fazlası gerekmez
const MAX_BODY: u64 = 5 * 1024 * 1024;

fn agent() -> &'static ureq::Agent {
    use std::sync::OnceLock;
    use ureq::tls::{TlsConfig, TlsProvider};
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            // 4xx/5xx da widget'a yanıt olarak dönsün, hata değil
            .http_status_as_error(false)
            .user_agent("masaustu-widget")
            // Windows'un kendi TLS'i (SChannel): ayrı TLS kütüphanesi gömülmez
            .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).build())
            .build()
            .into()
    })
}

fn do_fetch(req: FetchRequest) -> Result<FetchResponse, String> {
    if !(req.url.starts_with("http://") || req.url.starts_with("https://")) {
        return Err("yalnızca http/https adresleri".into());
    }
    let mut builder = ureq::http::Request::builder().method(req.method.to_uppercase().as_str()).uri(&req.url);
    for (k, v) in &req.headers {
        builder = builder.header(k, v);
    }
    let request = builder.body(req.body.unwrap_or_default()).map_err(|e| e.to_string())?;
    let mut res = agent().run(request).map_err(|e| e.to_string())?;
    let headers = res
        .headers()
        .iter()
        .filter_map(|(k, v)| Some((k.to_string(), v.to_str().ok()?.to_string())))
        .collect();
    let text = res.body_mut().with_config().limit(MAX_BODY).read_to_string().map_err(|e| e.to_string())?;
    Ok(FetchResponse { status: res.status().as_u16(), headers, text })
}

#[tauri::command]
pub async fn widget_fetch(request: FetchRequest) -> Result<FetchResponse, String> {
    tauri::async_runtime::spawn_blocking(move || do_fetch(request)).await.map_err(|e| e.to_string())?
}

#[derive(Serialize)]
pub struct SystemInfo {
    /// Tüm çekirdeklerin ortalaması, %
    cpu: f64,
    mem_used_mb: f64,
    mem_total_mb: f64,
    mem_percent: f64,
    uptime_s: u64,
}

#[cfg(windows)]
fn cpu_times() -> Option<(u64, u64)> {
    use windows::Win32::{Foundation::FILETIME, System::Threading::GetSystemTimes};
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()? };
    let v = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
    // Çekirdek süresi boşta geçen süreyi de içerir
    Some((v(idle), v(kernel) + v(user)))
}

/// Önceki ölçümle aradaki farktan CPU yüzdesi; ilk çağrıda kısa bir örnekleme yapılır.
#[cfg(windows)]
fn cpu_percent() -> f64 {
    static LAST: Mutex<Option<(u64, u64)>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap();
    let prev = match *last {
        Some(p) => p,
        None => {
            let p = cpu_times().unwrap_or_default();
            std::thread::sleep(Duration::from_millis(250));
            p
        }
    };
    let now = cpu_times().unwrap_or_default();
    *last = Some(now);
    let (idle, total) = (now.0.saturating_sub(prev.0), now.1.saturating_sub(prev.1));
    if total == 0 { 0.0 } else { ((1.0 - idle as f64 / total as f64) * 100.0).clamp(0.0, 100.0) }
}

#[tauri::command]
pub async fn system_info() -> Result<SystemInfo, String> {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::{GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX};
        tauri::async_runtime::spawn_blocking(|| {
            let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
            unsafe { GlobalMemoryStatusEx(&mut m).map_err(|e| e.to_string())? };
            let mb = |b: u64| b as f64 / 1024.0 / 1024.0;
            Ok(SystemInfo {
                cpu: cpu_percent(),
                mem_used_mb: mb(m.ullTotalPhys - m.ullAvailPhys),
                mem_total_mb: mb(m.ullTotalPhys),
                mem_percent: m.dwMemoryLoad as f64,
                uptime_s: unsafe { GetTickCount64() } / 1000,
            })
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    Err("yalnızca Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(url: &str) -> FetchRequest {
        FetchRequest { url: url.into(), method: get(), headers: HashMap::new(), body: None }
    }

    #[test]
    fn yalnizca_http_ve_https_kabul_edilir() {
        // Widget'lar yerel dosya okuyamamalı
        for url in ["file:///C:/Windows/win.ini", "ftp://example.com/a", r"C:\Windows\win.ini"] {
            assert!(do_fetch(req(url)).is_err(), "{url} reddedilmeliydi");
        }
    }

    #[test]
    fn gecersiz_yontem_hata_verir() {
        let mut r = req("https://example.com");
        r.method = "GE T".into();
        assert!(do_fetch(r).is_err());
    }
}
