//! widgets.json: tüm widget'lar tek dosyada tutulur.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Widget {
    pub id: String,
    pub name: String,
    /// Monitör adı (ör. `\\.\DISPLAY1`). Yoksa ya da bulunamazsa ana monitör.
    pub monitor: Option<String>,
    /// Monitörün sol üstüne göre CSS pikseli
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub html: String,
    pub css: String,
    pub js: String,
    pub opacity: f64,
    pub enabled: bool,
}

impl Default for Widget {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Yeni widget".into(),
            monitor: None,
            x: 80.0,
            y: 80.0,
            w: 320.0,
            h: 180.0,
            html: String::new(),
            css: String::new(),
            js: String::new(),
            opacity: 1.0,
            enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Config {
    pub widgets: Vec<Widget>,
}

fn sample() -> Config {
    Config {
        widgets: vec![Widget {
            id: "saat".into(),
            name: "Saat".into(),
            html: r#"<div id="t"></div><div id="d"></div>"#.into(),
            css: "body{display:grid;place-content:center;height:100vh;color:#fff;font-family:system-ui;\
                  background:rgba(20,30,60,.45);border-radius:16px;text-align:center}\
                  #t{font:600 56px system-ui}#d{opacity:.7;font-size:18px}"
                .into(),
            js: "const f=()=>{const n=new Date();t.textContent=n.toLocaleTimeString('tr-TR');\
                 d.textContent=n.toLocaleDateString('tr-TR',{weekday:'long',day:'numeric',month:'long'})};\
                 f();setInterval(f,1000);"
                .into(),
            ..Default::default()
        }],
    }
}

pub fn load(path: &PathBuf) -> Config {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("widgets.json okunamadı, boş başlanıyor: {e}");
            Config::default()
        }),
        Err(_) => {
            let c = sample();
            let _ = save(path, &c);
            c
        }
    }
}

pub fn save(path: &PathBuf, c: &Config) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    // Yarım yazılmış dosya kalmasın diye önce geçici dosyaya
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(c).unwrap()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}
