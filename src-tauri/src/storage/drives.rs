use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub id: String,
    pub mount_point: String,
    pub label: String,
    pub filesystem: String,
    pub drive_type: String,
    pub media_type: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub is_system: bool,
    pub is_removable: bool,
    pub auto_selected: bool,
}

#[cfg(windows)]
pub fn discover() -> Result<Vec<DriveInfo>, String> {
    use std::{env, ffi::OsString, os::windows::ffi::OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };
    use windows_sys::Win32::System::WindowsProgramming::{
        DRIVE_CDROM, DRIVE_FIXED, DRIVE_NO_ROOT_DIR, DRIVE_RAMDISK, DRIVE_REMOTE, DRIVE_REMOVABLE,
    };
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn text(buffer: &[u16]) -> String {
        let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        OsString::from_wide(&buffer[..len])
            .to_string_lossy()
            .into_owned()
    }
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        return Err("O Windows não informou as unidades disponíveis.".into());
    }
    let system = env::var("SystemDrive")
        .unwrap_or_else(|_| "C:".into())
        .to_uppercase();
    let mut result = Vec::new();
    for index in 0..26_u32 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let mount = format!("{}:\\", (b'A' + index as u8) as char);
        let root = wide(&mount);
        let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
        if kind == DRIVE_NO_ROOT_DIR || kind == DRIVE_CDROM {
            continue;
        }
        let mut available = 0_u64;
        let mut total = 0_u64;
        let mut free = 0_u64;
        if unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut available, &mut total, &mut free) } == 0
        {
            continue;
        }
        let mut label = [0_u16; 261];
        let mut fs_name = [0_u16; 261];
        let mut serial = 0_u32;
        let mut max_component = 0_u32;
        let mut flags = 0_u32;
        unsafe {
            GetVolumeInformationW(
                root.as_ptr(),
                label.as_mut_ptr(),
                label.len() as u32,
                &mut serial,
                &mut max_component,
                &mut flags,
                fs_name.as_mut_ptr(),
                fs_name.len() as u32,
            );
        }
        let drive_type = match kind {
            DRIVE_FIXED => "Fixed",
            DRIVE_REMOVABLE => "Removable",
            DRIVE_REMOTE => "Network",
            DRIVE_RAMDISK => "RAM disk",
            _ => "Unknown",
        }
        .to_string();
        let is_system = mount[..2].eq_ignore_ascii_case(&system);
        let is_removable = kind == DRIVE_REMOVABLE;
        result.push(DriveInfo {
            id: format!("volume-{serial:08X}"),
            mount_point: mount,
            label: text(&label),
            filesystem: text(&fs_name),
            drive_type: drive_type.clone(),
            media_type: if is_removable {
                "USB / removable"
            } else if kind == DRIVE_FIXED {
                "Fixed storage"
            } else {
                &drive_type
            }
            .into(),
            total_bytes: total,
            free_bytes: free,
            is_system,
            is_removable,
            auto_selected: kind == DRIVE_FIXED,
        });
    }
    Ok(result)
}

#[cfg(not(windows))]
pub fn discover() -> Result<Vec<DriveInfo>, String> {
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn discovers_system_drive_with_stable_identity() {
        let drives = discover().unwrap();
        assert!(drives.iter().any(|drive| drive.is_system
            && drive.id.starts_with("volume-")
            && drive.total_bytes > 0));
        assert!(drives
            .iter()
            .filter(|drive| drive.drive_type == "Network")
            .all(|drive| !drive.auto_selected));
        assert!(drives
            .iter()
            .filter(|drive| drive.is_removable)
            .all(|drive| !drive.auto_selected));
        assert!(drives
            .iter()
            .filter(|drive| drive.drive_type == "Fixed")
            .all(|drive| drive.auto_selected));
    }
}
