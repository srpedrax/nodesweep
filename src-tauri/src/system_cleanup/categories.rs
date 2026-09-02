#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemCategory {
    UserTemp,
    WindowsTemp,
    ThumbnailCache,
    ShaderCache,
    CrashDumps,
    ErrorReports,
    RecycleBin,
    DeliveryOptimization,
}

impl SystemCategory {
    pub fn key(self) -> &'static str {
        match self {
            Self::UserTemp => "user-temp",
            Self::WindowsTemp => "windows-temp",
            Self::ThumbnailCache => "thumbnails",
            Self::ShaderCache => "shader-cache",
            Self::CrashDumps => "crash-dumps",
            Self::ErrorReports => "error-reports",
            Self::RecycleBin => "recycle-bin",
            Self::DeliveryOptimization => "delivery-optimization",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::UserTemp => "Temporary files",
            Self::WindowsTemp => "Windows temporary files",
            Self::ThumbnailCache => "Thumbnail cache",
            Self::ShaderCache => "DirectX shader cache",
            Self::CrashDumps => "Crash dumps",
            Self::ErrorReports => "Windows error reports",
            Self::RecycleBin => "Recycle Bin",
            Self::DeliveryOptimization => "Delivery Optimization cache",
        }
    }
    pub fn risk(self) -> &'static str {
        match self {
            Self::RecycleBin | Self::DeliveryOptimization => "REVIEW",
            _ => "SAFE",
        }
    }
    pub fn consequence(self) -> &'static str {
        match self {
            Self::RecycleBin => "Files that can currently be restored will be permanently deleted.",
            Self::DeliveryOptimization => "Windows may download update content again.",
            Self::ThumbnailCache => "Windows will recreate image previews.",
            _ => "Applications or Windows may recreate these files.",
        }
    }
}
