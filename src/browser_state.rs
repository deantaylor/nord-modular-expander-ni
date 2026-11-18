//! Last clean browser position; controller preferences, never synth patch data.
use crate::{patch_browser::Browser, storage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Serialize, Deserialize)]
pub struct Preferences {
    version: u8,
    library: PathBuf,
    folder: PathBuf,
    selected: Option<String>,
    #[serde(default="default_ui_colour_intensity")]
    pub ui_colour_intensity: u8,
    #[serde(default)]
    pub ui_theme:u8,
    #[serde(default="default_on")]
    pub blobs_enabled:bool,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub glitches_enabled:Option<bool>,
    #[serde(default)]
    pub glitch_intensity:Option<u8>,
    pub files: bool,
    pub bank: Option<(u8, u8)>,
}
fn default_on()->bool{true}
fn default_ui_colour_intensity()->u8{0}
impl Preferences {
    pub fn glitch_level(&self)->u8{self.glitch_intensity.unwrap_or(if self.glitches_enabled==Some(false){0}else{50}).min(100)}
    pub fn load(
        path: &Path,
        library: &Path,
        browser: &mut Browser,
    ) -> Result<Option<Self>, String> {
        if !path.exists() {
            return Ok(None);
        }
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 16384 {
            return Err("Browser preferences too large".into());
        }
        let data: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if data.version != 1 {
            return Err("Unsupported browser preference version".into());
        }
        let root = std::fs::canonicalize(library).unwrap_or(library.into());
        if data.library != root {
            return Ok(None);
        }
        browser.restore_location(&data.folder, data.selected.as_deref())?;
        Ok(Some(data))
    }
    pub fn save(
        path: &Path,
        library: &Path,
        browser: &Browser,
        files: bool,
        bank: Option<(u8, u8)>,
        ui_colour_intensity: u8,
        ui_theme:u8,
        blobs_enabled:bool,
        glitch_intensity:u8,
    ) -> Result<(), String> {
        let (folder, selected) = browser.location();
        let data = Self {
            version: 1,
            library: std::fs::canonicalize(library).unwrap_or(library.into()),
            folder,
            selected,
            files,
            bank,
            ui_colour_intensity:ui_colour_intensity.min(100),
            ui_theme:ui_theme.min(2),
            blobs_enabled,glitches_enabled:None,glitch_intensity:Some(glitch_intensity.min(100)),
        };
        storage::save_json(
            path,
            &serde_json::to_value(data).map_err(|e| e.to_string())?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_position_restore_removed_folder_and_root_escape() {
        let root = std::env::temp_dir().join(format!(
            "mk3-browser-prefs-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("Sub")).unwrap();
        std::fs::write(root.join("Root.pch"), "").unwrap();
        std::fs::write(root.join("Sub/Selected.pch"), "").unwrap();
        let mut browser = Browser::new(root.clone());
        browser
            .restore_location(Path::new("Sub"), Some("Selected.pch"))
            .unwrap();
        let path = root.join("prefs.json");
        Preferences::save(&path, &root, &browser, false, Some((2, 17)), 65, 2, false, 0).unwrap();
        let mut restored = Browser::new(root.clone());
        let p = Preferences::load(&path, &root, &mut restored)
            .unwrap()
            .unwrap();
        assert!(!p.files);assert!(!p.blobs_enabled);assert_eq!(p.glitch_intensity,Some(0));assert_eq!(p.ui_theme,2);assert_eq!(p.ui_colour_intensity,65);
        assert_eq!(p.bank, Some((2, 17)));
        let mut legacy:serde_json::Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        legacy.as_object_mut().unwrap().remove("blobs_enabled");legacy.as_object_mut().unwrap().remove("glitches_enabled");legacy.as_object_mut().unwrap().remove("glitch_intensity");legacy.as_object_mut().unwrap().remove("ui_theme");legacy.as_object_mut().unwrap().remove("ui_colour_intensity");std::fs::write(&path,serde_json::to_vec(&legacy).unwrap()).unwrap();
        let legacy=Preferences::load(&path,&root,&mut restored).unwrap().unwrap();assert_eq!(legacy.ui_colour_intensity,0);assert_eq!(legacy.ui_theme,0);assert!(legacy.blobs_enabled);assert_eq!(legacy.glitch_intensity,None);assert_eq!(legacy.glitch_level(),50);
        let mut old:serde_json::Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();old["glitches_enabled"]=serde_json::json!(false);std::fs::write(&path,serde_json::to_vec(&old).unwrap()).unwrap();assert_eq!(Preferences::load(&path,&root,&mut restored).unwrap().unwrap().glitch_level(),0);
        assert_eq!(
            restored.location(),
            (PathBuf::from("Sub"), Some("Selected.pch".into()))
        );
        std::fs::remove_dir_all(root.join("Sub")).unwrap();
        Preferences::load(&path, &root, &mut restored).unwrap();
        assert_eq!(restored.location().0, PathBuf::new());
        assert!(restored.restore_location(Path::new("../"), None).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
