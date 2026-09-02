use super::categories::SystemCategory;
use std::{env, path::PathBuf};

pub fn known() -> Vec<(SystemCategory, PathBuf)> {
    let local = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let windows = env::var_os("WINDIR").map(PathBuf::from);
    let program_data = env::var_os("ProgramData").map(PathBuf::from);
    let mut out = Vec::new();
    if let Some(temp) = env::var_os("TEMP") {
        out.push((SystemCategory::UserTemp, PathBuf::from(temp)))
    }
    if let Some(local) = local {
        out.push((SystemCategory::UserTemp, local.join("Temp")));
        out.push((
            SystemCategory::ThumbnailCache,
            local.join("Microsoft/Windows/Explorer"),
        ));
        out.push((SystemCategory::ShaderCache, local.join("D3DSCache")));
        out.push((SystemCategory::CrashDumps, local.join("CrashDumps")));
        out.push((
            SystemCategory::ErrorReports,
            local.join("Microsoft/Windows/WER"),
        ));
    }
    if let Some(windows) = windows {
        out.push((SystemCategory::WindowsTemp, windows.join("Temp")));
        out.push((SystemCategory::DeliveryOptimization,windows.join("ServiceProfiles/NetworkService/AppData/Local/Microsoft/Windows/DeliveryOptimization/Cache")));
    }
    if let Some(data) = program_data {
        out.push((
            SystemCategory::ErrorReports,
            data.join("Microsoft/Windows/WER"),
        ));
    }
    out
}
