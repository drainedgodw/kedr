use kedr_ui::{Settings, Theme};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: String,
    pub font_size: f32,
    pub recent: Vec<PathBuf>,
}
impl Preferences {
    pub fn read() -> Self {
        let Some(path) = path() else {
            return Self::default();
        };
        let mut data = Vec::new();
        let result = fs::File::open(path).and_then(|f| f.take(65537).read_to_end(&mut data));
        if result.is_err() || data.len() > 65536 {
            return Self::default();
        }
        serde_json::from_slice(&data).unwrap_or_default()
    }
    pub fn settings(&self) -> Settings {
        Settings {
            theme: if self.theme == "light" {
                Theme::Light
            } else {
                Theme::Dark
            },
            font_size: if self.font_size.is_finite() && (14.0..=24.0).contains(&self.font_size) {
                self.font_size
            } else {
                15.
            },
            ..Default::default()
        }
    }
    pub fn write(&self) -> io::Result<()> {
        let Some(path) = path() else {
            return Ok(());
        };
        fs::create_dir_all(path.parent().unwrap())?;
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        let data = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        std::io::Write::write_all(&mut file, &data)?;
        drop(file);
        let result = fs::rename(&temp, path);
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
}
fn path() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("Kedr/preferences.json"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join("Library/Application Support/Kedr/preferences.json"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join("kedr/preferences.json"))
    }
}
