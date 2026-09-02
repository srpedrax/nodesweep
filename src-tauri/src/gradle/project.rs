use std::{fs, path::Path};

pub fn is_gradle_project(path: &Path) -> bool {
    let settings =
        path.join("settings.gradle").is_file() || path.join("settings.gradle.kts").is_file();
    let build = path.join("build.gradle").is_file() || path.join("build.gradle.kts").is_file();
    let launcher = path.join("gradlew").is_file() || path.join("gradlew.bat").is_file();
    let wrapper = path
        .join("gradle/wrapper/gradle-wrapper.properties")
        .is_file();
    (settings && build) || (launcher && wrapper)
}

pub fn wrapper_version(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path.join("gradle/wrapper/gradle-wrapper.properties")).ok()?;
    let url = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("distributionUrl="))?;
    let filename = url.rsplit('/').next()?.replace("\\:", ":");
    let versioned = filename.strip_prefix("gradle-")?;
    versioned
        .strip_suffix("-bin.zip")
        .or_else(|| versioned.strip_suffix("-all.zip"))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_is_not_project() {
        let t = tempfile::tempdir().unwrap();
        assert!(!is_gradle_project(t.path()));
    }
    #[test]
    fn groovy_project() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("settings.gradle"), "").unwrap();
        fs::write(t.path().join("build.gradle"), "").unwrap();
        assert!(is_gradle_project(t.path()));
    }
    #[test]
    fn kotlin_project() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("settings.gradle.kts"), "").unwrap();
        fs::write(t.path().join("build.gradle.kts"), "").unwrap();
        assert!(is_gradle_project(t.path()));
    }
    #[test]
    fn wrapper_project() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("gradle/wrapper")).unwrap();
        fs::write(t.path().join("gradlew"), "").unwrap();
        fs::write(
            t.path().join("gradle/wrapper/gradle-wrapper.properties"),
            "distributionUrl=https\\://services.gradle.org/distributions/gradle-8.11.1-bin.zip",
        )
        .unwrap();
        assert!(is_gradle_project(t.path()));
        assert_eq!(wrapper_version(t.path()).as_deref(), Some("8.11.1"));
    }
}
