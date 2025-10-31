use std::path::Path;

pub fn is_target_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("ts" | "tsx" | "js" | "jsx")
    )
}
