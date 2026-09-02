use std::{ffi::OsString, path::PathBuf};

pub fn resolve(
    gradle_user_home: Option<OsString>,
    user_profile: Option<OsString>,
) -> Option<PathBuf> {
    gradle_user_home
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            user_profile
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".gradle"))
        })
}

pub fn current() -> Option<PathBuf> {
    resolve(
        std::env::var_os("GRADLE_USER_HOME"),
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_home_wins() {
        assert_eq!(
            resolve(Some("X:/custom".into()), Some("X:/user".into())),
            Some(PathBuf::from("X:/custom"))
        );
    }
    #[test]
    fn falls_back_to_user_home() {
        assert_eq!(
            resolve(None, Some("X:/user".into())),
            Some(PathBuf::from("X:/user").join(".gradle"))
        );
    }
    #[test]
    fn no_home_returns_none() {
        assert_eq!(resolve(None, None), None);
    }
}
