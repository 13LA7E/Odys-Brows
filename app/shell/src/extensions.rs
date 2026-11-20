//! Extensions management (Phase 6: P6-01, P6-02, P6-03 / EDGE_CASES §10).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub permissions: Vec<String>,
    pub icon: Option<String>,
    pub memory_mb: f64,
    pub cpu_pct: f64,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct ManifestV3 {
    pub manifest_version: Option<u32>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub optional_permissions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ExtensionsStore {
    pub extensions: HashMap<String, ExtensionInfo>,
}

impl ExtensionsStore {
    pub fn load_or_default(dir: &Path) -> Self {
        let file = dir.join("extensions.json");
        let Ok(content) = std::fs::read_to_string(&file) else {
            return Self::default();
        };
        serde_json::from_str::<ExtensionsStore>(&content).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let file = dir.join("extensions.json");
        let tmp = dir.join("extensions.json.tmp");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, file)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<ExtensionInfo> {
        let mut list: Vec<_> = self.extensions.values().cloned().collect();
        list.sort_by_key(|a| a.name.to_lowercase());
        list
    }

    /// Folders of enabled unpacked extensions, passed to Chromium at start (`--load-extension`).
    pub fn enabled_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .extensions
            .values()
            .filter(|e| e.enabled)
            .filter_map(|e| e.path.clone())
            .collect();
        paths.sort();
        paths
    }

    pub fn toggle(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(ext) = self.extensions.get_mut(id) {
            ext.enabled = enabled;
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        self.extensions.remove(id).is_some()
    }

    pub fn load_unpacked(&mut self, path: &Path) -> Result<ExtensionInfo, String> {
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            return Err("manifest.json not found in the selected folder".to_string());
        }

        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest.json: {e}"))?;

        let manifest: ManifestV3 =
            serde_json::from_str(&content).map_err(|e| format!("Invalid manifest.json: {e}"))?;

        if let Some(mv) = manifest.manifest_version.filter(|&v| v < 3) {
            return Err(format!(
                "Manifest V{mv} is deprecated. Constellations requires Manifest V3."
            ));
        }

        let name = manifest
            .name
            .unwrap_or_else(|| "Unnamed Extension".to_string());
        let version = manifest.version.unwrap_or_else(|| "1.0.0".to_string());
        let description = manifest.description.unwrap_or_default();
        let permissions = manifest.permissions;

        // Deterministic ID derived from path
        let id = format!("{:x}", md5_hash(path.to_string_lossy().as_bytes()));

        let info = ExtensionInfo {
            id: id.clone(),
            name,
            version,
            description,
            enabled: true,
            permissions,
            icon: None,
            // Not measured yet: CEF gives no per-extension cost, so no number is invented.
            memory_mb: 0.0,
            cpu_pct: 0.0,
            path: Some(path.to_string_lossy().to_string()),
        };

        self.extensions.insert(id, info.clone());
        Ok(info)
    }
}

fn md5_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_unpacked_v3_manifest() {
        let tmp = std::env::temp_dir().join(format!("ext-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let manifest = r#"{
            "manifest_version": 3,
            "name": "Test Extension",
            "version": "2.1.0",
            "description": "A helpful test extension",
            "permissions": ["storage", "tabs"]
        }"#;
        std::fs::write(tmp.join("manifest.json"), manifest).unwrap();

        let mut store = ExtensionsStore::default();
        let res = store.load_unpacked(&tmp);
        assert!(res.is_ok());
        let info = res.unwrap();
        assert_eq!(info.name, "Test Extension");
        assert_eq!(info.version, "2.1.0");
        assert_eq!(info.permissions, vec!["storage", "tabs"]);
        assert!(info.enabled);

        // Toggle and remove
        assert!(store.toggle(&info.id, false));
        assert!(!store.extensions.get(&info.id).unwrap().enabled);
        assert!(store.remove(&info.id));
        assert!(store.extensions.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rejects_v2_manifest() {
        let tmp = std::env::temp_dir().join(format!("ext-test-v2-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let manifest = r#"{
            "manifest_version": 2,
            "name": "Old Extension",
            "version": "1.0"
        }"#;
        std::fs::write(tmp.join("manifest.json"), manifest).unwrap();

        let mut store = ExtensionsStore::default();
        let res = store.load_unpacked(&tmp);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Manifest V2 is deprecated"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
