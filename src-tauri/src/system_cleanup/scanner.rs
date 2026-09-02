use super::{categories::SystemCategory, paths, Target};
use crate::{scan::directory_size, storage::DriveInfo};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub id: String,
    pub source: String,
    pub category: String,
    pub display_name: String,
    pub drive_id: String,
    pub path: String,
    pub target_type: String,
    pub size_bytes: u64,
    pub risk_level: String,
    pub deletable: bool,
    pub consequence: String,
    pub recommendation: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemScan {
    pub items: Vec<CleanupItem>,
    pub total_size_bytes: u64,
    pub recommended_size_bytes: u64,
}
fn id(path: &Path, category: SystemCategory, drive: &str) -> String {
    let mut h = DefaultHasher::new();
    path.to_string_lossy().to_lowercase().hash(&mut h);
    category.hash(&mut h);
    drive.hash(&mut h);
    format!("system-{:016x}", h.finish())
}
fn size_for(path: &Path, category: SystemCategory) -> u64 {
    if category == SystemCategory::ThumbnailCache {
        fs::read_dir(path)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with("thumbcache_")
            })
            .filter_map(|e| e.metadata().ok().map(|m| m.len()))
            .sum()
    } else {
        directory_size(path).unwrap_or(0)
    }
}
fn drive_for<'a>(path: &Path, drives: &'a [DriveInfo]) -> Option<&'a DriveInfo> {
    drives.iter().find(|d| {
        path.to_string_lossy()
            .to_lowercase()
            .starts_with(&d.mount_point.to_lowercase())
    })
}
pub fn scan(drives: &[DriveInfo]) -> Result<(SystemScan, HashMap<String, Target>), String> {
    let mut items = Vec::new();
    let mut targets = HashMap::new();
    let mut seen = HashSet::new();
    for (category, path) in paths::known() {
        if !path.is_dir() {
            continue;
        }
        let canonical = match fs::canonicalize(&path) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let key = canonical.to_string_lossy().to_lowercase();
        if !seen.insert((category, key)) {
            continue;
        }
        let drive = match drive_for(&canonical, drives) {
            Some(d) => d,
            None => continue,
        };
        let size = size_for(&canonical, category);
        if size == 0 {
            continue;
        }
        let item_id = id(&canonical, category, &drive.id);
        let recommended = category.risk() == "SAFE" && size >= 10 * 1024 * 1024;
        targets.insert(
            item_id.clone(),
            Target {
                path: path.clone(),
                canonical,
                category,
                drive_id: drive.id.clone(),
                requested_bytes: size,
            },
        );
        items.push(CleanupItem {
            id: item_id,
            source: "System".into(),
            category: category.key().into(),
            display_name: category.name().into(),
            drive_id: drive.id.clone(),
            path: path.to_string_lossy().into_owned(),
            target_type: "Managed".into(),
            size_bytes: size,
            risk_level: category.risk().into(),
            deletable: true,
            consequence: category.consequence().into(),
            recommendation: if recommended {
                "Recommended"
            } else {
                "Optional"
            }
            .into(),
        });
    }
    #[cfg(windows)]
    for drive in drives.iter().filter(|d| d.drive_type != "Network") {
        use std::mem::size_of;
        use windows_sys::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
        let wide: Vec<u16> = drive.mount_point.encode_utf16().chain(Some(0)).collect();
        let mut info = SHQUERYRBINFO {
            cbSize: size_of::<SHQUERYRBINFO>() as u32,
            i64Size: 0,
            i64NumItems: 0,
        };
        if unsafe { SHQueryRecycleBinW(wide.as_ptr(), &mut info) } == 0 && info.i64Size > 0 {
            let category = SystemCategory::RecycleBin;
            let item_id = format!("recycle-{}", drive.id);
            let size = info.i64Size as u64;
            targets.insert(
                item_id.clone(),
                Target {
                    path: PathBuf::from(&drive.mount_point),
                    canonical: PathBuf::from(&drive.mount_point),
                    category,
                    drive_id: drive.id.clone(),
                    requested_bytes: size,
                },
            );
            items.push(CleanupItem {
                id: item_id,
                source: "System".into(),
                category: category.key().into(),
                display_name: category.name().into(),
                drive_id: drive.id.clone(),
                path: drive.mount_point.clone(),
                target_type: "Managed".into(),
                size_bytes: size,
                risk_level: "REVIEW".into(),
                deletable: true,
                consequence: category.consequence().into(),
                recommendation: "Optional".into(),
            });
        }
    }
    let total = items.iter().map(|i| i.size_bytes).sum();
    let recommended = items
        .iter()
        .filter(|i| i.recommendation == "Recommended")
        .map(|i| i.size_bytes)
        .sum();
    Ok((
        SystemScan {
            items,
            total_size_bytes: total,
            recommended_size_bytes: recommended,
        },
        targets,
    ))
}
