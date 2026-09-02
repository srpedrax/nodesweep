use super::{categories::SystemCategory, Target};
use crate::storage::DriveInfo;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupReport {
    pub requested_bytes: u64,
    pub freed_bytes: u64,
    pub removed: u64,
    pub skipped: u64,
    pub failed: u64,
}
fn current_drive<'a>(target: &Target, drives: &'a [DriveInfo]) -> Option<&'a DriveInfo> {
    drives.iter().find(|drive| {
        drive.id == target.drive_id
            && target
                .path
                .to_string_lossy()
                .to_lowercase()
                .starts_with(&drive.mount_point.to_lowercase())
    })
}
fn protected_root(path: &Path) -> bool {
    let lower = path
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_lowercase();
    let mut roots = vec![
        std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".into())
            .to_lowercase(),
        std::env::var("WINDIR")
            .unwrap_or_else(|_| "C:\\Windows".into())
            .to_lowercase(),
        std::env::var("USERPROFILE")
            .unwrap_or_default()
            .to_lowercase(),
        std::env::var("ProgramFiles")
            .unwrap_or_default()
            .to_lowercase(),
        std::env::var("ProgramFiles(x86)")
            .unwrap_or_default()
            .to_lowercase(),
        std::env::var("ProgramData")
            .unwrap_or_default()
            .to_lowercase(),
    ];
    roots.retain(|r| !r.is_empty());
    roots.contains(&lower)
}
fn validate(target: &Target, drives: &[DriveInfo]) -> Result<(), String> {
    if current_drive(target, drives).is_none() {
        return Err("Drive unavailable or volume identity changed. No files were removed.".into());
    }
    if target.category == SystemCategory::RecycleBin {
        return Ok(());
    }
    if protected_root(&target.path) {
        return Err("Uma raiz protegida nunca pode ser removida.".into());
    }
    let canonical = fs::canonicalize(&target.path)
        .map_err(|_| "O alvo não está mais disponível.".to_string())?;
    if canonical != target.canonical {
        return Err("O alvo mudou depois da varredura.".into());
    }
    Ok(())
}
fn eligible(path: &Path, category: SystemCategory) -> bool {
    category != SystemCategory::ThumbnailCache
        || path.file_name().is_some_and(|n| {
            n.to_string_lossy()
                .to_lowercase()
                .starts_with("thumbcache_")
        })
}
fn remove_contents(root: &Path, category: SystemCategory, report: &mut CleanupReport) {
    // The Explorer directory contains more than thumbnail data. Keep this
    // category deliberately flat and remove only the files counted by scan.
    if category == SystemCategory::ThumbnailCache {
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(_) => {
                report.skipped += 1;
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_file()) && eligible(&path, category) {
                match fs::remove_file(path) {
                    Ok(_) => report.removed += 1,
                    Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                        report.skipped += 1
                    }
                    Err(_) => report.failed += 1,
                }
            }
        }
        return;
    }
    let mut dirs = Vec::<PathBuf>::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        for e in entries.flatten() {
            let path = e.path();
            let kind = match e.file_type() {
                Ok(k) => k,
                Err(_) => {
                    report.skipped += 1;
                    continue;
                }
            };
            if kind.is_symlink() {
                match fs::remove_file(&path).or_else(|_| fs::remove_dir(&path)) {
                    Ok(_) => report.removed += 1,
                    Err(_) => report.skipped += 1,
                }
            } else if kind.is_dir() {
                pending.push(path.clone());
                dirs.push(path)
            } else if eligible(&path, category) {
                match fs::remove_file(&path) {
                    Ok(_) => report.removed += 1,
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) => {
                        report.skipped += 1
                    }
                    Err(_) => report.failed += 1,
                }
            }
        }
    }
    for dir in dirs.into_iter().rev() {
        if fs::remove_dir(dir).is_ok() {
            report.removed += 1
        }
    }
}
pub fn clean(
    ids: &[String],
    confirmed: bool,
    registry: &HashMap<String, Target>,
    drives: &[DriveInfo],
) -> Result<CleanupReport, String> {
    if !confirmed {
        return Err("A limpeza exige confirmação explícita.".into());
    }
    if ids.is_empty() {
        return Err("Selecione ao menos um item.".into());
    }
    if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err("A seleção contém IDs duplicados.".into());
    }
    let targets: Vec<_> = ids
        .iter()
        .map(|id| {
            registry
                .get(id)
                .ok_or_else(|| "Item desconhecido ou snapshot expirado.".to_string())
        })
        .collect::<Result<_, _>>()?;
    for target in &targets {
        validate(target, drives)?
    }
    let requested = targets.iter().map(|t| t.requested_bytes).sum();
    let mut report = CleanupReport {
        requested_bytes: requested,
        freed_bytes: 0,
        removed: 0,
        skipped: 0,
        failed: 0,
    };
    for target in targets {
        validate(target, drives)?;
        if target.category == SystemCategory::RecycleBin {
            #[cfg(windows)]
            {
                use windows_sys::Win32::UI::Shell::{
                    SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
                };
                let wide: Vec<u16> = target
                    .path
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                if unsafe {
                    SHEmptyRecycleBinW(
                        std::ptr::null_mut(),
                        wide.as_ptr(),
                        SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
                    )
                } == 0
                {
                    report.removed += 1
                } else {
                    report.failed += 1
                }
            }
        } else {
            remove_contents(&target.path, target.category, &mut report)
        }
    }
    let refreshed = crate::storage::discover().unwrap_or_default();
    let before: u64 = drives.iter().map(|d| d.free_bytes).sum();
    let after: u64 = refreshed
        .iter()
        .filter(|d| drives.iter().any(|old| old.id == d.id))
        .map(|d| d.free_bytes)
        .sum();
    report.freed_bytes = after.saturating_sub(before).min(report.requested_bytes);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DriveInfo;
    fn fixture() -> (tempfile::TempDir, String, Target, DriveInfo) {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Temp");
        fs::create_dir(&p).unwrap();
        fs::write(p.join("a.tmp"), "12345").unwrap();
        let mount = t
            .path()
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy()
            .into_owned();
        let drive = DriveInfo {
            id: "volume-test".into(),
            mount_point: mount,
            label: "".into(),
            filesystem: "".into(),
            drive_type: "Fixed".into(),
            media_type: "".into(),
            total_bytes: 1,
            free_bytes: 0,
            is_system: false,
            is_removable: false,
            auto_selected: false,
        };
        let target = Target {
            path: p.clone(),
            canonical: fs::canonicalize(&p).unwrap(),
            category: SystemCategory::UserTemp,
            drive_id: drive.id.clone(),
            requested_bytes: 5,
        };
        (t, "id".into(), target, drive)
    }
    #[test]
    fn confirmation_required() {
        let (_t, id, target, drive) = fixture();
        let registry = HashMap::from([(id.clone(), target)]);
        assert!(clean(std::slice::from_ref(&id), false, &registry, &[drive]).is_err())
    }
    #[test]
    fn invalid_batch_removes_nothing() {
        let (_t, id, target, drive) = fixture();
        let p = target.path.clone();
        assert!(clean(
            &[id.clone(), "bad".into()],
            true,
            &HashMap::from([(id, target)]),
            &[drive]
        )
        .is_err());
        assert!(p.join("a.tmp").exists())
    }
    #[test]
    fn changed_volume_rejected() {
        let (_t, id, target, _) = fixture();
        let registry = HashMap::from([(id.clone(), target)]);
        assert!(clean(std::slice::from_ref(&id), true, &registry, &[]).is_err())
    }
    #[test]
    fn duplicate_ids_rejected() {
        let (_t, id, target, drive) = fixture();
        assert!(clean(
            &[id.clone(), id.clone()],
            true,
            &HashMap::from([(id, target)]),
            &[drive]
        )
        .is_err())
    }
    #[test]
    fn thumbnail_cleanup_preserves_unrelated_explorer_content() {
        let (_t, id, mut target, drive) = fixture();
        target.category = SystemCategory::ThumbnailCache;
        fs::write(target.path.join("thumbcache_32.db"), "cache").unwrap();
        fs::write(target.path.join("unrelated.db"), "keep").unwrap();
        fs::create_dir(target.path.join("unrelated-directory")).unwrap();
        let registry = HashMap::from([(id.clone(), target.clone())]);
        clean(&[id], true, &registry, &[drive]).unwrap();
        assert!(!target.path.join("thumbcache_32.db").exists());
        assert!(target.path.join("unrelated.db").exists());
        assert!(target.path.join("unrelated-directory").is_dir());
    }
}
