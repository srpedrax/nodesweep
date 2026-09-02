use crate::scan::directory_size;
use serde::Serialize;
use std::{
    collections::HashSet,
    fs,
    path::{Component, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedItem {
    pub path: String,
    pub freed_bytes: u64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub deleted: Vec<DeletedItem>,
    pub total_freed_bytes: u64,
}
#[derive(Debug)]
struct SafeTarget {
    resolved: PathBuf,
    canonical: PathBuf,
}

fn is_protected_system_path(path: &PathBuf) -> bool {
    #[cfg(windows)]
    let protected: Vec<PathBuf> = ["WINDIR", "ProgramFiles", "ProgramFiles(x86)", "ProgramData"]
        .iter()
        .filter_map(|name| std::env::var_os(name).map(PathBuf::from))
        .collect();
    #[cfg(not(windows))]
    let protected: Vec<PathBuf> = ["/bin", "/boot", "/etc", "/root", "/sbin", "/usr", "/var"]
        .iter()
        .map(PathBuf::from)
        .collect();
    let candidate = path.to_string_lossy().to_lowercase();
    protected.iter().any(|base| {
        let base = base
            .to_string_lossy()
            .trim_end_matches(['/', '\\'])
            .to_lowercase();
        candidate == base || candidate.starts_with(&format!("{base}{}", std::path::MAIN_SEPARATOR))
    })
}

fn validate(target: &str) -> Result<SafeTarget, String> {
    if target.trim().is_empty() {
        return Err("Todo caminho de limpeza deve ser preenchido.".into());
    }
    let resolved = PathBuf::from(target);
    if !resolved.is_absolute() {
        return Err(format!("O caminho precisa ser absoluto: {target}"));
    }
    if is_protected_system_path(&resolved) {
        return Err(format!("Caminho de sistema protegido: {target}"));
    }
    if !resolved
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("node_modules"))
    {
        return Err(format!(
            "Somente pastas node_modules podem ser removidas: {target}"
        ));
    }
    let mut current = PathBuf::new();
    for component in resolved.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                current.push(component.as_os_str())
            }
            Component::CurDir | Component::ParentDir => {
                return Err(format!("O caminho não pode conter . ou ..: {target}"))
            }
        }
        let metadata = fs::symlink_metadata(&current)
            .map_err(|_| format!("Caminho não encontrado: {target}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("Symlinks e junctions não são permitidos: {target}"));
        }
    }
    if !fs::symlink_metadata(&resolved)
        .map_err(|_| format!("Caminho não encontrado: {target}"))?
        .is_dir()
    {
        return Err(format!("O alvo precisa ser uma pasta real: {target}"));
    }
    let manifest = resolved
        .parent()
        .ok_or_else(|| format!("Projeto inválido: {target}"))?
        .join("package.json");
    let manifest_metadata = fs::symlink_metadata(&manifest)
        .map_err(|_| format!("O projeto não possui package.json regular: {target}"))?;
    if !manifest_metadata.is_file() || manifest_metadata.file_type().is_symlink() {
        return Err(format!(
            "O projeto não possui package.json regular: {target}"
        ));
    }
    let canonical = fs::canonicalize(&resolved)
        .map_err(|error| format!("Não foi possível validar {target}: {error}"))?;
    Ok(SafeTarget {
        resolved,
        canonical,
    })
}

pub fn cleanup(paths: &[String], confirmed: bool) -> Result<CleanupResult, String> {
    if !confirmed {
        return Err("A limpeza exige confirmação explícita.".into());
    }
    if paths.is_empty() {
        return Err("Selecione ao menos uma pasta para limpar.".into());
    }
    let targets: Vec<_> = paths
        .iter()
        .map(|path| validate(path))
        .collect::<Result<_, _>>()?;
    let unique: HashSet<_> = targets
        .iter()
        .map(|target| target.canonical.to_string_lossy().to_lowercase())
        .collect();
    if unique.len() != targets.len() {
        return Err("A seleção contém caminhos duplicados.".into());
    }
    let mut deleted = Vec::new();
    for target in targets {
        let size_before = directory_size(&target.resolved)?;
        let revalidated = validate(&target.resolved.to_string_lossy())?;
        if revalidated.canonical != target.canonical {
            return Err(format!(
                "O alvo mudou durante a operação: {}",
                target.resolved.display()
            ));
        }
        fs::remove_dir_all(&target.resolved).map_err(|error| {
            format!(
                "Não foi possível remover {}: {error}",
                target.resolved.display()
            )
        })?;
        deleted.push(DeletedItem {
            path: target.resolved.to_string_lossy().into_owned(),
            freed_bytes: size_before,
        });
    }
    let total_freed_bytes = deleted.iter().map(|item| item.freed_bytes).sum();
    Ok(CleanupResult {
        deleted,
        total_freed_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("app");
        let modules = project.join("node_modules");
        fs::create_dir_all(&modules).unwrap();
        fs::write(project.join("package.json"), "{}").unwrap();
        fs::write(modules.join("file.txt"), "12345").unwrap();
        (temp, modules)
    }
    #[test]
    fn requires_confirmation() {
        let (_t, m) = fixture();
        assert!(cleanup(&[m.to_string_lossy().into_owned()], false).is_err());
        assert!(m.exists());
    }
    #[test]
    fn validates_batch_before_deleting() {
        let (t, m) = fixture();
        let invalid = t.path().join("src");
        assert!(cleanup(
            &[
                m.to_string_lossy().into_owned(),
                invalid.to_string_lossy().into_owned()
            ],
            true
        )
        .is_err());
        assert!(m.exists());
    }
    #[test]
    fn deletes_valid_modules_and_reports_size() {
        let (_t, m) = fixture();
        let result = cleanup(&[m.to_string_lossy().into_owned()], true).unwrap();
        assert_eq!(result.total_freed_bytes, 5);
        assert!(!m.exists());
    }
    #[test]
    fn rejects_duplicates_and_non_modules_targets() {
        let (_t, m) = fixture();
        let value = m.to_string_lossy().into_owned();
        assert!(cleanup(&[value.clone(), value], true).is_err());
        assert!(validate(m.parent().unwrap().to_str().unwrap()).is_err());
        assert!(m.exists());
    }
}
