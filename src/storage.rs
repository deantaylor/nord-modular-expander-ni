//! Versioned project JSON with validation and atomic replacement.
use crate::project::Project;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
const MAX_BYTES: u64 = 1024 * 1024;
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    version: u32,
    engine: String,
    project: Project,
}
pub fn load(path: &Path) -> Result<Option<Project>, String> {
    check_extension(path)?;
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Project file exceeds 1 MiB".into());
    }
    let doc: Document = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid project JSON ({}): {e}", path.display()))?;
    if !matches!(
        doc.format.as_str(),
        "nord-modular-ni-interface-project" | "modular-machine-project"
    ) || doc.version != 1
        || doc.engine != "mock_g1"
    {
        return Err("Unsupported project format/version/engine; expected mock project v1".into());
    }
    doc.project.validate()?;
    Ok(Some(doc.project))
}
pub fn save(path: &Path, project: &Project) -> Result<(), String> {
    check_extension(path)?;
    project.validate()?;
    let bytes = serde_json::to_vec_pretty(&Document {
        format: "nord-modular-ni-interface-project".into(),
        version: 1,
        engine: "mock_g1".into(),
        project: project.clone(),
    })
    .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Project exceeds 1 MiB".into());
    }
    atomic_write(path, &bytes)
}
pub(crate) fn save_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES as usize {
        return Err("Preferences exceed size limit".into());
    }
    atomic_write(path, &bytes)
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary: PathBuf = parent.join(format!(
        ".mk3-{}-{}.tmp",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
fn check_extension(path: &Path) -> Result<(), String> {
    if path.extension().is_some_and(|e| e == "json") {
        Ok(())
    } else {
        Err("Use a project-owned .json filename; Nord patch files are not supported".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temporary() -> PathBuf {
        std::env::temp_dir().join(format!(
            "mk3-project-{}-{}.json",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }
    #[test]
    fn round_trip_all_slots_and_invalid_save_preserves_existing_file() {
        let path = temporary();
        let mut project = Project::initial();
        project.selected_slot = crate::project::Slot::C;
        project.slots[2].patch.modules[1].parameters[0].value = 2500.;
        save(&path, &project).unwrap();
        assert_eq!(load(&path).unwrap(), Some(project.clone()));
        let before = fs::read(&path).unwrap();
        project.slots[0].patch.modules[0].parameters[0].value = f64::NAN;
        assert!(save(&path, &project).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        let mut previous: serde_json::Value = serde_json::from_slice(&before).unwrap();
        assert_eq!(previous["format"], "nord-modular-ni-interface-project");
        previous["format"] = "modular-machine-project".into();
        fs::write(&path, serde_json::to_vec(&previous).unwrap()).unwrap();
        assert!(load(&path).unwrap().is_some());
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejects_unknown_schema_duplicate_ids_and_corrupt_json() {
        let path = temporary();
        let project = Project::initial();
        save(&path, &project).unwrap();
        let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        doc["version"] = 2.into();
        fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
        assert!(load(&path).is_err());
        fs::write(&path, b"{broken").unwrap();
        assert!(load(&path).is_err());
        let mut project = project;
        project.slots[0].patch.modules[0].parameters[1].id =
            project.slots[0].patch.modules[0].parameters[0].id.clone();
        assert!(save(&path, &project).is_err());
        fs::remove_file(path).unwrap();
        assert!(save(Path::new("unsupported.pch"), &Project::initial()).is_err());
    }
}
