use std::path::Path;

pub(crate) fn is_safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && !value.contains('\\')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

pub(crate) fn is_safe_relative_directory(value: &str) -> bool {
    value == "." || is_safe_relative_path(value)
}

pub(crate) fn path_relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map_or_else(
            || ".".to_owned(),
            |relative| relative.to_string_lossy().replace('\\', "/"),
        )
}
