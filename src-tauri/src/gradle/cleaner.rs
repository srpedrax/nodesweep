use super::{Category, Target};
use crate::scan::directory_size;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cleaned {
    pub id: String,
    pub path: String,
    pub freed_bytes: u64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub deleted: Vec<Cleaned>,
    pub total_freed_bytes: u64,
}

fn has_link_component(path: &Path) -> bool {
    let mut current = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                current.push(c.as_os_str())
            }
            _ => return true,
        }
        if fs::symlink_metadata(&current).is_ok_and(|m| m.file_type().is_symlink()) {
            return true;
        }
    }
    false
}
fn allowed_name(target: &Target) -> bool {
    match target.category {
        Category::GlobalCache => target.path.file_name().is_some_and(|n| n == "caches"),
        Category::Daemon => target.path.file_name().is_some_and(|n| n == "daemon"),
        Category::ProjectCache => target.path.file_name().is_some_and(|n| n == ".gradle"),
        Category::BuildOutput => target.path.file_name().is_some_and(|n| n == "build"),
        Category::WrapperDistribution => target
            .path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|n| n == "dists"),
    }
}
fn validate(target: &Target) -> Result<(), String> {
    if !allowed_name(target) {
        return Err("Categoria fora da whitelist.".into());
    }
    if target.path == target.boundary || target.path.parent().is_none() {
        return Err("Roots nunca podem ser removidos.".into());
    }
    if has_link_component(&target.path) {
        return Err("Symlink ou junction detectado.".into());
    }
    let canonical =
        fs::canonicalize(&target.path).map_err(|_| "O alvo não existe mais.".to_string())?;
    if canonical != target.canonical {
        return Err("O alvo mudou desde a varredura.".into());
    }
    let canonical_boundary = fs::canonicalize(&target.boundary)
        .map_err(|_| "O limite de segurança não existe mais.".to_string())?;
    if !canonical.starts_with(&canonical_boundary) {
        return Err("O alvo saiu do limite permitido.".into());
    }
    let modified = fs::metadata(&target.path).and_then(|m| m.modified()).ok();
    if modified != target.modified {
        return Err("O alvo foi modificado depois da varredura; escaneie novamente.".into());
    }
    Ok(())
}

pub fn clean(
    ids: &[String],
    confirmed: bool,
    registry: &HashMap<String, Target>,
) -> Result<CleanResult, String> {
    if !confirmed {
        return Err("A limpeza exige confirmação explícita.".into());
    }
    if ids.is_empty() {
        return Err("Selecione ao menos um item.".into());
    }
    let unique: HashSet<_> = ids.iter().collect();
    if unique.len() != ids.len() {
        return Err("A seleção contém IDs duplicados.".into());
    }
    let targets: Vec<_> = ids
        .iter()
        .map(|id| {
            registry
                .get(id)
                .ok_or_else(|| "Item desconhecido ou snapshot expirado.".to_string())
                .map(|t| (id, t))
        })
        .collect::<Result<_, _>>()?;
    for (_, target) in &targets {
        validate(target)?
    }
    let mut deleted = Vec::new();
    for (id, target) in targets {
        validate(target)?;
        let before = directory_size(&target.path)?;
        fs::remove_dir_all(&target.path)
            .map_err(|e| format!("Falha ao remover {}: {e}", target.path.display()))?;
        let after = if target.path.exists() {
            directory_size(&target.path)?
        } else {
            0
        };
        deleted.push(Cleaned {
            id: id.clone(),
            path: target.path.to_string_lossy().into_owned(),
            freed_bytes: before.saturating_sub(after),
        });
    }
    let total = deleted.iter().map(|i| i.freed_bytes).sum();
    Ok(CleanResult {
        deleted,
        total_freed_bytes: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> (tempfile::TempDir, String, Target) {
        let t = tempfile::tempdir().unwrap();
        let project = t.path().join("p");
        let p = project.join("build");
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("x"), "12345").unwrap();
        let target = Target {
            canonical: fs::canonicalize(&p).unwrap(),
            path: p,
            boundary: project,
            category: Category::BuildOutput,
            modified: fs::metadata(t.path().join("p/build"))
                .unwrap()
                .modified()
                .ok(),
        };
        (t, "id".into(), target)
    }
    #[test]
    fn confirmation_required() {
        let (_t, id, x) = target();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], false, &r).is_err())
    }
    #[test]
    fn unknown_id_rejected() {
        assert!(clean(&["bad".into()], true, &HashMap::new()).is_err())
    }
    #[test]
    fn duplicate_ids_rejected() {
        let (_t, id, x) = target();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id.clone(), id], true, &r).is_err())
    }
    #[test]
    fn invalid_batch_deletes_nothing() {
        let (_t, id, x) = target();
        let path = x.path.clone();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id, "bad".into()], true, &r).is_err());
        assert!(path.exists())
    }
    #[test]
    fn valid_cleanup_reports_freed() {
        let (_t, id, x) = target();
        let p = x.path.clone();
        let r = HashMap::from([(id.clone(), x)]);
        let out = clean(&[id], true, &r).unwrap();
        assert_eq!(out.total_freed_bytes, 5);
        assert!(!p.exists())
    }
    #[test]
    fn project_root_is_rejected() {
        let (_t, id, mut x) = target();
        x.path = x.boundary.clone();
        x.canonical = fs::canonicalize(&x.path).unwrap();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], true, &r).is_err())
    }
    #[test]
    fn gradle_home_root_is_rejected() {
        let (_t, id, mut x) = target();
        x.category = Category::GlobalCache;
        x.path = x.boundary.clone();
        x.canonical = fs::canonicalize(&x.path).unwrap();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], true, &r).is_err())
    }
    #[test]
    fn protected_name_is_rejected() {
        let (_t, id, mut x) = target();
        x.path = x.boundary.join("gradle.properties");
        fs::write(&x.path, "").unwrap();
        x.canonical = fs::canonicalize(&x.path).unwrap();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], true, &r).is_err())
    }
    #[test]
    fn changed_target_is_rejected() {
        let (_t, id, mut x) = target();
        x.canonical = x.boundary.clone();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], true, &r).is_err())
    }
    #[test]
    fn whitelist_blocks_jdks() {
        let (_t, id, mut x) = target();
        x.path = x.boundary.join("jdks");
        fs::create_dir_all(&x.path).unwrap();
        x.canonical = fs::canonicalize(&x.path).unwrap();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], true, &r).is_err())
    }
    #[test]
    fn whitelist_blocks_init_d() {
        let (_t, id, mut x) = target();
        x.path = x.boundary.join("init.d");
        fs::create_dir_all(&x.path).unwrap();
        x.canonical = fs::canonicalize(&x.path).unwrap();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], true, &r).is_err())
    }
    #[test]
    fn whitelist_blocks_wrapper_root() {
        let (_t, id, mut x) = target();
        x.path = x.boundary.join("wrapper");
        fs::create_dir_all(&x.path).unwrap();
        x.canonical = fs::canonicalize(&x.path).unwrap();
        let r = HashMap::from([(id.clone(), x)]);
        assert!(clean(&[id], true, &r).is_err())
    }
    #[test]
    fn rejects_linked_path_components() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        let link = temp.path().join("linked");
        fs::create_dir(&real).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&link)
                .arg(&real)
                .status()
                .unwrap();
            assert!(status.success());
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(has_link_component(&link));
    }
}
