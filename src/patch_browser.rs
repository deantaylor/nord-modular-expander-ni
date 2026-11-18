//! Local .pch selection only; parsing and loading stay in G1-Emu.
use std::{fs, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub directory: bool,
}
pub struct Browser {
    root: PathBuf,
    current: PathBuf,
    pub entries: Vec<Entry>,
    pub cursor: usize,
    pub revision: u64,
}
#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum SortPart {
    Number(usize, String, usize),
    Text(String),
}
fn natural_key(name: &str) -> Vec<SortPart> {
    let name = name.to_lowercase();
    let bytes = name.as_bytes();
    let mut start = 0;
    let mut parts = Vec::new();
    while start < bytes.len() {
        let digits = bytes[start].is_ascii_digit();
        let mut end = start + 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() == digits {
            end += 1;
        }
        let part = &name[start..end];
        parts.push(if digits {
            let number = part.trim_start_matches('0');
            SortPart::Number(number.len(), number.into(), part.len())
        } else {
            SortPart::Text(part.into())
        });
        start = end;
    }
    parts
}
impl Browser {
    pub fn location(&self) -> (PathBuf, Option<String>) {
        (
            self.current
                .strip_prefix(&self.root)
                .unwrap_or(std::path::Path::new(""))
                .into(),
            self.entries.get(self.cursor).map(|e| e.name.clone()),
        )
    }
    pub fn restore_location(
        &mut self,
        folder: &std::path::Path,
        selected: Option<&str>,
    ) -> Result<(), String> {
        if folder.components().any(|c| {
            !matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        }) {
            return Err("Saved folder is outside the patch library".into());
        }
        let root = fs::canonicalize(&self.root).map_err(|e| e.to_string())?;
        let target = fs::canonicalize(root.join(folder)).unwrap_or_else(|_| root.clone());
        self.current = if target.starts_with(&root) && target.is_dir() {
            target
        } else {
            root
        };
        self.refresh()?;
        self.cursor = selected
            .and_then(|name| self.entries.iter().position(|e| e.name == name))
            .unwrap_or(0);
        Ok(())
    }
    pub fn new(root: PathBuf) -> Self {
        let root = fs::canonicalize(&root).unwrap_or(root);
        Self {
            current: root.clone(),
            root,
            entries: Vec::new(),
            cursor: 0,
            revision: 0,
        }
    }
    pub fn refresh(&mut self) -> Result<(), String> {
        let selected = self.entries.get(self.cursor).map(|e| e.path.clone());
        let mut entries = Vec::new();
        for entry in
            fs::read_dir(&self.current).map_err(|e| format!("Cannot read patch folder: {e}"))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            // Do not follow symlinks outside the chosen library or into directory loops.
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir()
                || (kind.is_file()
                    && path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("pch")))
            {
                if entries.len() >= 4096 {
                    return Err("Patch folder has more than 4096 entries; use subfolders".into());
                }
                entries.push(Entry {
                    name: entry.file_name().to_string_lossy().into(),
                    path,
                    directory: kind.is_dir(),
                });
            }
        }
        entries.sort_by_cached_key(|e| (!e.directory, natural_key(&e.name), e.name.clone()));
        self.cursor = selected
            .and_then(|path| entries.iter().position(|e| e.path == path))
            .unwrap_or(0);
        self.entries = entries;
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub fn move_by(&mut self, delta: i32) {
        self.cursor = (self.cursor as i64 + i64::from(delta))
            .clamp(0, self.entries.len().saturating_sub(1) as i64) as usize;
    }
    pub fn enter(&mut self) -> Result<Option<PathBuf>, String> {
        let entry = self
            .entries
            .get(self.cursor)
            .ok_or("No .pch file selected")?
            .clone();
        // Recheck at activation too: a file can change after the list was read.
        let path = fs::canonicalize(&entry.path).map_err(|e| e.to_string())?;
        let root = fs::canonicalize(&self.root).map_err(|e| e.to_string())?;
        if !path.starts_with(root) {
            return Err("Selected path is outside the patch folder".into());
        }
        if entry.directory {
            let before = self.current.clone();
            self.current = path;
            if let Err(error) = self.refresh() {
                self.current = before;
                return Err(error);
            }
            Ok(None)
        } else {
            Ok(Some(path))
        }
    }
    pub fn parent(&mut self) -> Result<(), String> {
        if self.current != self.root {
            let before = self.current.clone();
            self.current = self.current.parent().unwrap_or(&self.root).to_path_buf();
            if let Err(error) = self.refresh() {
                self.current = before;
                return Err(error);
            }
            self.cursor = self
                .entries
                .iter()
                .position(|e| e.path == before)
                .unwrap_or(0);
        }
        Ok(())
    }
    pub fn folder(&self) -> String {
        let relative = self
            .current
            .strip_prefix(&self.root)
            .unwrap_or(&self.current);
        if relative.as_os_str().is_empty() {
            self.root
                .file_name()
                .unwrap_or(self.root.as_os_str())
                .to_string_lossy()
                .into()
        } else {
            relative.to_string_lossy().into()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn natural_sort_large_library_and_clamped_jumps() {
        let root = std::env::temp_dir().join(format!(
            "mk3-large-library-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        for i in (1..=1000).rev() {
            std::fs::write(root.join(format!("Bass{i}.pch")), "").unwrap();
        }
        let mut browser = super::Browser::new(root.clone());
        browser.refresh().unwrap();
        assert_eq!(browser.entries[1].name, "Bass2.pch");
        assert_eq!(browser.entries[999].name, "Bass1000.pch");
        browser.move_by(i32::MAX);
        assert_eq!(browser.cursor, 999);
        browser.move_by(i32::MIN);
        assert_eq!(browser.cursor, 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    use super::*;
    #[test]
    fn nested_files_refresh_selection_and_root_boundary() {
        let root = std::env::temp_dir().join(format!(
            "mk3-browser-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("Bass")).unwrap();
        let root = root.canonicalize().unwrap();
        fs::write(root.join("Bass/deep.PCH"), "fixture").unwrap();
        fs::write(root.join("Lead.pch"), "fixture").unwrap();
        fs::write(root.join("ignored.txt"), "fixture").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.parent().unwrap(), root.join("outside")).unwrap();
        let mut browser = Browser::new(root.clone());
        browser.refresh().unwrap();
        assert_eq!(browser.entries.len(), 2);
        assert_eq!(browser.entries[0].name, "Bass");
        assert!(browser.enter().unwrap().is_none());
        assert_eq!(
            browser.enter().unwrap(),
            Some(root.join("Bass/deep.PCH").canonicalize().unwrap())
        );
        browser.parent().unwrap();
        browser.parent().unwrap();
        assert_eq!(browser.current, root);
        browser.move_by(i32::MAX);
        assert_eq!(browser.entries[browser.cursor].name, "Lead.pch");
        fs::write(root.join("Earlier.pch"), "fixture").unwrap();
        browser.refresh().unwrap();
        assert_eq!(browser.entries[browser.cursor].name, "Lead.pch");
        fs::remove_file(root.join("Lead.pch")).unwrap();
        assert!(browser.enter().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
