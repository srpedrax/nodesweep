mod cleaner;
mod paths;
mod project;
mod scanner;

pub use cleaner::{clean, CleanResult};
pub use scanner::{scan, GradleScan};

use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    GlobalCache,
    WrapperDistribution,
    Daemon,
    ProjectCache,
    BuildOutput,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub path: PathBuf,
    pub canonical: PathBuf,
    pub category: Category,
    pub boundary: PathBuf,
    pub modified: Option<SystemTime>,
}

#[derive(Default)]
pub struct GradleState(pub Mutex<HashMap<String, Target>>);
