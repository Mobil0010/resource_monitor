use std::path::PathBuf;

pub(super) fn shutdown_file() -> PathBuf {
    std::env::temp_dir().join("resource_monitor.shutdown")
}

pub(super) fn popup_file() -> PathBuf {
    std::env::temp_dir().join("resource_monitor.popup.conf")
}

pub(super) fn settings_file() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library/Application Support/Resource Monitor/settings.conf");
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Resource Monitor/settings.conf");
    }
    #[allow(unreachable_code)]
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".config")
        })
        .join("resource-monitor/settings.conf")
}

pub(super) fn save(value: &str) -> Result<(), String> {
    let path = settings_file();
    let parent = path
        .parent()
        .ok_or_else(|| "설정 파일 경로를 만들 수 없습니다.".to_owned())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(path, value).map_err(|error| error.to_string())
}
