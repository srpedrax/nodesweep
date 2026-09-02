mod categories;
mod cleaner;
mod paths;
mod scanner;

pub use cleaner::{clean, CleanupReport};
pub use scanner::{scan, SystemScan};

use categories::SystemCategory;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

#[derive(Clone)]
pub struct Target {
    pub path: PathBuf,
    pub canonical: PathBuf,
    pub category: SystemCategory,
    pub drive_id: String,
    pub requested_bytes: u64,
}
#[derive(Default)]
pub struct SystemState(pub Mutex<HashMap<String, Target>>);
