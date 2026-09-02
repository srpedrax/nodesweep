use serde::Serialize;
use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub node_modules_path: String,
    pub size_bytes: u64,
    pub last_modified: u64,
    pub risk_level: String,
    pub deletable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub projects: Vec<Project>,
    pub total_size_bytes: u64,
}

pub fn directory_size(root: &Path) -> Result<u64, String> {
    let mut total = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "Não foi possível ler {}: {error}",
                    current.display()
                ))
            }
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(value) => value,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total = total.saturating_add(metadata.len());
                }
            }
        }
    }
    Ok(total)
}

fn modified_millis(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

pub fn scan(root_path: &str) -> Result<ScanResult, String> {
    if root_path.trim().is_empty() {
        return Err("Informe uma pasta para escanear.".into());
    }
    let root = PathBuf::from(root_path);
    let metadata = fs::metadata(&root).map_err(|_| {
        "Pasta não encontrada. Verifique o caminho informado e tente novamente.".to_string()
    })?;
    if !metadata.is_dir() {
        return Err("O caminho informado precisa ser uma pasta.".into());
    }
    let mut projects = Vec::new();
    let mut pending = vec![root];
    while let Some(current) = pending.pop() {
        let entries: Vec<_> = match fs::read_dir(&current) {
            Ok(entries) => entries.filter_map(Result::ok).collect(),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound
                ) =>
            {
                continue
            }
            Err(error) => {
                return Err(format!(
                    "Não foi possível ler {}: {error}",
                    current.display()
                ))
            }
        };
        let has_package = entries.iter().any(|entry| {
            entry.file_name() == "package.json"
                && entry.file_type().is_ok_and(|kind| kind.is_file())
        });
        let modules = entries.iter().find(|entry| {
            entry.file_name() == "node_modules" && entry.file_type().is_ok_and(|kind| kind.is_dir())
        });
        if has_package {
            if let Some(modules) = modules {
                let modules_path = modules.path();
                projects.push(Project {
                    id: {
                        let mut hasher = DefaultHasher::new();
                        fs::canonicalize(&modules_path)
                            .unwrap_or_else(|_| modules_path.clone())
                            .to_string_lossy()
                            .to_lowercase()
                            .hash(&mut hasher);
                        format!("node-{:016x}", hasher.finish())
                    },
                    name: current
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    path: current.to_string_lossy().into_owned(),
                    node_modules_path: modules_path.to_string_lossy().into_owned(),
                    size_bytes: directory_size(&modules_path)?,
                    last_modified: modified_millis(&modules_path),
                    risk_level: "SAFE".into(),
                    deletable: true,
                });
            }
        }
        for entry in entries {
            if entry.file_name() != "node_modules"
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
            {
                pending.push(entry.path());
            }
        }
    }
    projects.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.path.cmp(&b.path))
    });
    let total_size_bytes = projects.iter().map(|project| project.size_bytes).sum();
    Ok(ScanResult {
        projects,
        total_size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_project_and_skips_nested_project_inside_modules() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("app");
        let modules = project.join("node_modules");
        fs::create_dir_all(modules.join("dependency/nested/node_modules")).unwrap();
        fs::write(project.join("package.json"), "{}").unwrap();
        fs::write(modules.join("dependency/file.txt"), "12345").unwrap();
        fs::write(modules.join("dependency/nested/package.json"), "{}").unwrap();
        let result = scan(temp.path().to_str().unwrap()).unwrap();
        assert_eq!(result.projects.len(), 1);
        assert_eq!(result.total_size_bytes, 7);
    }
    #[test]
    fn rejects_missing_and_non_directory_roots() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.txt");
        fs::write(&file, "x").unwrap();
        assert!(scan("").is_err());
        assert!(scan(temp.path().join("missing").to_str().unwrap()).is_err());
        assert!(scan(file.to_str().unwrap()).is_err());
    }
}
