use super::{paths, project, Category, Target};
use crate::scan::directory_size;
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub category: String,
    pub scope: String,
    pub size_bytes: u64,
    pub risk_level: String,
    pub deletable: bool,
    pub description: String,
    pub used_by: Vec<String>,
    pub last_modified: u64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleProject {
    pub name: String,
    pub path: String,
    pub wrapper_version: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradleScan {
    pub gradle_home: Option<String>,
    pub items: Vec<GradleItem>,
    pub projects: Vec<GradleProject>,
    pub total_size_bytes: u64,
    pub recoverable_size_bytes: u64,
}

fn millis(time: Option<SystemTime>) -> u64 {
    time.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
fn identity(path: &Path, category: &Category) -> String {
    let mut h = DefaultHasher::new();
    path.to_string_lossy().to_lowercase().hash(&mut h);
    format!("{category:?}").hash(&mut h);
    format!("gradle-{:016x}", h.finish())
}
fn contains_lock(root: &Path) -> bool {
    let mut p = vec![root.to_path_buf()];
    while let Some(d) = p.pop() {
        if let Ok(es) = fs::read_dir(d) {
            for e in es.flatten() {
                if e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("lock"))
                {
                    return true;
                }
                if e.file_type().is_ok_and(|x| x.is_dir()) {
                    p.push(e.path())
                }
            }
        }
    }
    false
}
#[allow(clippy::too_many_arguments)]
fn push_item(
    items: &mut Vec<GradleItem>,
    targets: &mut HashMap<String, Target>,
    path: PathBuf,
    boundary: &Path,
    category: Category,
    name: String,
    scope: &str,
    risk: &str,
    description: &str,
    deletable: bool,
    used_by: Vec<String>,
) {
    if !path.is_dir() {
        return;
    }
    let canonical = match fs::canonicalize(&path) {
        Ok(p) => p,
        Err(_) => return,
    };
    let modified = fs::metadata(&path).and_then(|m| m.modified()).ok();
    let size = directory_size(&path).unwrap_or(0);
    let id = identity(&canonical, &category);
    if deletable {
        targets.insert(
            id.clone(),
            Target {
                path: path.clone(),
                canonical,
                category: category.clone(),
                boundary: fs::canonicalize(boundary).unwrap_or_else(|_| boundary.to_path_buf()),
                modified,
            },
        );
    }
    items.push(GradleItem {
        id,
        name,
        path: path.to_string_lossy().into_owned(),
        category: format!("{category:?}"),
        scope: scope.into(),
        size_bytes: size,
        risk_level: risk.into(),
        deletable,
        description: description.into(),
        used_by,
        last_modified: millis(modified),
    });
}
fn find_projects(root: &Path) -> Vec<GradleProject> {
    let mut result = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(current) = pending.pop() {
        if project::is_gradle_project(&current) {
            result.push(GradleProject {
                name: current
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                path: current.to_string_lossy().into_owned(),
                wrapper_version: project::wrapper_version(&current),
            });
        }
        let entries = match fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for e in entries.flatten() {
            let n = e.file_name();
            if e.file_type().is_ok_and(|t| t.is_dir())
                && n != "node_modules"
                && n != ".gradle"
                && n != "build"
                && n != ".git"
            {
                pending.push(e.path())
            }
        }
    }
    result
}

pub fn scan(root: &str) -> Result<(GradleScan, HashMap<String, Target>), String> {
    scan_with_home(root, paths::current())
}

fn scan_with_home(
    root: &str,
    gradle_home: Option<PathBuf>,
) -> Result<(GradleScan, HashMap<String, Target>), String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err("A pasta de projetos não existe.".into());
    }
    let projects = find_projects(&root);
    let used: HashMap<String, Vec<String>> = projects
        .iter()
        .filter_map(|p| {
            p.wrapper_version
                .as_ref()
                .map(|v| (v.clone(), p.name.clone()))
        })
        .fold(HashMap::new(), |mut m, (v, n)| {
            m.entry(v).or_default().push(n);
            m
        });
    let mut items = Vec::new();
    let mut targets = HashMap::new();
    for p in &projects {
        let base = PathBuf::from(&p.path);
        push_item(
            &mut items,
            &mut targets,
            base.join(".gradle"),
            &base,
            Category::ProjectCache,
            format!("{} · .gradle", p.name),
            "Project",
            "SAFE",
            "Project cache; Gradle can recreate it.",
            true,
            vec![p.name.clone()],
        );
        push_item(
            &mut items,
            &mut targets,
            base.join("build"),
            &base,
            Category::BuildOutput,
            format!("{} · build", p.name),
            "Project",
            "SAFE",
            "Build artifacts; recompilation may be required.",
            true,
            vec![p.name.clone()],
        );
    }
    if let Some(home) = &gradle_home {
        push_item(
            &mut items,
            &mut targets,
            home.join("caches"),
            home,
            Category::GlobalCache,
            "Gradle cache".into(),
            "Global",
            "REVIEW",
            "Dependencies and generated caches; downloads may be required.",
            true,
            vec![],
        );
        let daemon = home.join("daemon");
        let active = contains_lock(&daemon);
        push_item(
            &mut items,
            &mut targets,
            daemon,
            home,
            Category::Daemon,
            "Gradle daemon".into(),
            "Global",
            "SAFE",
            if active {
                "Daemon lock detected; cleanup unavailable."
            } else {
                "Daemon logs and state; mostly managed by Gradle."
            },
            !active,
            vec![],
        );
        push_item(
            &mut items,
            &mut targets,
            home.join("jdks"),
            home,
            Category::GlobalCache,
            "Gradle-managed JDKs".into(),
            "Global",
            "PROTECTED",
            "Managed runtimes; cleanup unavailable in v2.1.",
            false,
            vec![],
        );
        let dists = home.join("wrapper/dists");
        if let Ok(entries) = fs::read_dir(&dists) {
            for e in entries
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            {
                let raw = e.file_name().to_string_lossy().into_owned();
                let version = raw
                    .strip_prefix("gradle-")
                    .and_then(|s| s.strip_suffix("-bin").or_else(|| s.strip_suffix("-all")))
                    .unwrap_or(&raw)
                    .to_string();
                let users = used.get(&version).cloned().unwrap_or_default();
                let cleanable = users.is_empty();
                push_item(
                    &mut items,
                    &mut targets,
                    e.path(),
                    home,
                    Category::WrapperDistribution,
                    format!("Gradle {version}"),
                    "Global",
                    if cleanable { "REVIEW" } else { "PROTECTED" },
                    if cleanable {
                        "No known project uses this distribution."
                    } else {
                        "Used by known projects; kept."
                    },
                    cleanable,
                    users,
                );
            }
        }
    }
    let total = items.iter().map(|i| i.size_bytes).sum();
    let recoverable = items
        .iter()
        .filter(|i| i.deletable)
        .map(|i| i.size_bytes)
        .sum();
    Ok((
        GradleScan {
            gradle_home: gradle_home.map(|p| p.to_string_lossy().into_owned()),
            items,
            projects,
            total_size_bytes: total,
            recoverable_size_bytes: recoverable,
        },
        targets,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project(root: &Path, name: &str, kotlin: bool) {
        let p = root.join(name);
        fs::create_dir_all(p.join("build")).unwrap();
        fs::create_dir_all(p.join(".gradle")).unwrap();
        fs::write(
            p.join(if kotlin {
                "settings.gradle.kts"
            } else {
                "settings.gradle"
            }),
            "",
        )
        .unwrap();
        fs::write(
            p.join(if kotlin {
                "build.gradle.kts"
            } else {
                "build.gradle"
            }),
            "",
        )
        .unwrap();
        fs::write(p.join("build/a"), "123").unwrap();
    }
    #[test]
    fn detects_local_cache_and_build() {
        let t = tempfile::tempdir().unwrap();
        project(t.path(), "groovy", false);
        let ps = find_projects(t.path());
        assert_eq!(ps.len(), 1);
        let mut i = vec![];
        let mut targets = HashMap::new();
        let b = PathBuf::from(&ps[0].path);
        push_item(
            &mut i,
            &mut targets,
            b.join("build"),
            &b,
            Category::BuildOutput,
            "b".into(),
            "Project",
            "SAFE",
            "",
            true,
            vec![],
        );
        assert_eq!(i[0].size_bytes, 3);
    }
    #[test]
    fn project_without_outputs_has_no_items() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("settings.gradle"), "").unwrap();
        fs::write(t.path().join("build.gradle"), "").unwrap();
        let ps = find_projects(t.path());
        assert_eq!(ps.len(), 1);
        assert!(!t.path().join("build").exists());
    }
    #[test]
    fn skips_fake_build_folder() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir(t.path().join("build")).unwrap();
        assert!(find_projects(t.path()).is_empty());
    }
    #[test]
    fn ids_are_stable() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(
            identity(t.path(), &Category::BuildOutput),
            identity(t.path(), &Category::BuildOutput)
        );
    }
    #[test]
    fn lock_detection() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("registry.bin.lock"), "").unwrap();
        assert!(contains_lock(t.path()));
    }
    #[test]
    fn scans_custom_home_and_missing_categories() {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join("jdks")).unwrap();
        let (result, targets) = scan_with_home(
            root.path().to_str().unwrap(),
            Some(home.path().to_path_buf()),
        )
        .unwrap();
        assert_eq!(result.gradle_home.as_deref(), home.path().to_str());
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].risk_level, "PROTECTED");
        assert!(targets.is_empty());
    }
    #[test]
    fn correlates_used_and_unused_wrapper_distributions() {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir_all(app.join("gradle/wrapper")).unwrap();
        fs::write(app.join("gradlew"), "").unwrap();
        fs::write(
            app.join("gradle/wrapper/gradle-wrapper.properties"),
            "distributionUrl=https\\://services.gradle.org/distributions/gradle-8.11.1-bin.zip",
        )
        .unwrap();
        fs::create_dir_all(home.path().join("wrapper/dists/gradle-8.11.1-bin/hash")).unwrap();
        fs::create_dir_all(home.path().join("wrapper/dists/gradle-7.6-bin/hash")).unwrap();
        let (result, _) = scan_with_home(
            root.path().to_str().unwrap(),
            Some(home.path().to_path_buf()),
        )
        .unwrap();
        let used = result
            .items
            .iter()
            .find(|item| item.name == "Gradle 8.11.1")
            .unwrap();
        let unused = result
            .items
            .iter()
            .find(|item| item.name == "Gradle 7.6")
            .unwrap();
        assert!(!used.deletable);
        assert_eq!(used.used_by, vec!["app"]);
        assert!(unused.deletable);
    }
}
