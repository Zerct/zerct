use super::super::{
    constants::{ARCHIVE_EXCLUDES, ARCHIVE_LIMIT_BYTES, WALK_EXCLUDED_DIRS},
    errors::{Result, agent_error},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use flate2::{Compression, write::GzEncoder};
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

pub(super) fn create_archive_base64(project_dir: &Path, json_output: bool) -> Result<String> {
    let mut archive = Vec::new();
    {
        let encoder = GzEncoder::new(&mut archive, Compression::default());
        let mut builder = tar::Builder::new(encoder);
        for entry in WalkDir::new(project_dir)
            .into_iter()
            .filter_entry(|entry| !is_archive_excluded_entry(project_dir, entry))
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = match entry.path().strip_prefix(project_dir) {
                Ok(relative) => relative,
                Err(_error) => continue,
            };
            builder
                .append_path_with_name(entry.path(), Path::new(".").join(relative))
                .map_err(|error| {
                    agent_error(
                        "archive_failed",
                        "Could not create source archive.",
                        format!("Check project files and retry: {error}"),
                        json_output,
                    )
                })?;
        }
        let encoder = builder.into_inner().map_err(|error| {
            agent_error(
                "archive_failed",
                "Could not create source archive.",
                format!("Check project files and retry: {error}"),
                json_output,
            )
        })?;
        encoder.finish().map_err(|error| {
            agent_error(
                "archive_failed",
                "Could not create source archive.",
                format!("Check project files and retry: {error}"),
                json_output,
            )
        })?;
    }
    if archive.len() > ARCHIVE_LIMIT_BYTES {
        return Err(agent_error(
            "archive_too_large",
            "Source archive is too large.",
            "Remove build outputs, target directories, logs, and local caches before deploying.",
            json_output,
        ));
    }
    Ok(BASE64.encode(archive))
}

fn is_archive_excluded_entry(project_dir: &Path, entry: &DirEntry) -> bool {
    if entry.path() == project_dir {
        return false;
    }
    let relative = match entry.path().strip_prefix(project_dir) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_error) => return true,
    };
    is_archive_excluded(&relative, entry.file_type().is_dir())
}

fn is_archive_excluded(relative: &str, is_dir: bool) -> bool {
    let basename = relative.rsplit('/').next().unwrap_or(relative);
    if is_dir && WALK_EXCLUDED_DIRS.contains(&basename) {
        return true;
    }
    ARCHIVE_EXCLUDES.iter().any(|pattern| match *pattern {
        "*.pem" => basename_has_extension(basename, "pem"),
        "*.key" => basename_has_extension(basename, "key"),
        "*.p12" => basename_has_extension(basename, "p12"),
        "*.pfx" => basename_has_extension(basename, "pfx"),
        "*.tfstate" => basename_has_extension(basename, "tfstate"),
        "*.tfstate.*" => basename.contains(".tfstate."),
        "*.sqlite" => basename_has_extension(basename, "sqlite"),
        "*.sqlite3" => basename_has_extension(basename, "sqlite3"),
        "*.db" => basename_has_extension(basename, "db"),
        "*.log" => basename_has_extension(basename, "log"),
        "._*" => basename.starts_with("._"),
        ".env.*" => basename.starts_with(".env."),
        pattern => relative == pattern || relative.starts_with(&format!("{pattern}/")),
    })
}

fn basename_has_extension(basename: &str, extension: &str) -> bool {
    Path::new(basename)
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

#[cfg(test)]
mod tests {
    use super::is_archive_excluded;

    #[test]
    fn excludes_common_frontend_build_outputs() {
        for path in [
            ".next/server/app/page.js",
            "out/index.html",
            "dist/assets/app.js",
            "build/static/app.js",
            ".cache/tool/state",
            ".turbo/cache/file",
        ] {
            assert!(
                is_archive_excluded(path, false),
                "{path} should be excluded"
            );
        }
    }
}
