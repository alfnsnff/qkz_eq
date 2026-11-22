// EQ presets: built-ins plus user presets stored as JSON in %APPDATA%\qkz_eq\presets.json (PC only, never device flash).
use crate::device::{Band, BANDS};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub bands: [Band; BANDS],
    #[serde(skip)]
    pub builtin: bool,
}

fn b(freq: u16, gain: f32, q: f32) -> Band {
    Band { freq, gain, q, kind: 0 }
}

fn builtin(name: &str, bands: [Band; BANDS]) -> Preset {
    Preset {
        name: name.into(),
        bands,
        builtin: true,
    }
}

pub fn builtins() -> Vec<Preset> {
    vec![
        builtin(
            "Flat",
            [
                b(1000, 0.0, 0.707),
                b(2000, 0.0, 0.707),
                b(5000, 0.0, 0.707),
                b(8000, 0.0, 0.707),
                b(10000, 0.0, 0.707),
            ],
        ),
        builtin(
            "Bass Boost",
            [
                b(60, 5.0, 0.8),
                b(150, 3.0, 0.9),
                b(1000, 0.0, 0.707),
                b(4000, 0.0, 0.707),
                b(10000, 0.0, 0.707),
            ],
        ),
        builtin(
            "V-Shape",
            [
                b(70, 4.5, 0.8),
                b(300, 1.0, 0.9),
                b(1500, -2.5, 0.8),
                b(5000, 2.0, 1.0),
                b(11000, 4.0, 0.8),
            ],
        ),
        builtin(
            "Vocal",
            [
                b(100, -2.0, 0.8),
                b(400, -1.0, 1.0),
                b(2000, 3.0, 1.0),
                b(4000, 2.0, 1.2),
                b(10000, 0.0, 0.707),
            ],
        ),
        builtin(
            "Treble Soft",
            [
                b(1000, 0.0, 0.707),
                b(3000, -1.0, 1.0),
                b(6000, -2.5, 1.2),
                b(8000, -3.0, 1.0),
                b(12000, -3.0, 0.8),
            ],
        ),
    ]
}

fn path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("qkz_eq").join("presets.json")
}

pub fn load_user() -> Vec<Preset> {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_user(list: &[Preset]) -> Result<(), String> {
    let p = path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let user: Vec<&Preset> = list.iter().filter(|p| !p.builtin).collect();
    let s = serde_json::to_string_pretty(&user).map_err(|e| e.to_string())?;
    std::fs::write(p, s).map_err(|e| e.to_string())
}
