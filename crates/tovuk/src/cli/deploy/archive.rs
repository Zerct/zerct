use super::super::{
    constants::{ARCHIVE_EXCLUDES, ARCHIVE_LIMIT_BYTES, WALK_EXCLUDED_DIRS},
    errors::{Result, agent_error},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use flate2::{Compression, write::GzEncoder};
use std::{fs, io::Cursor, path::Path};
use tar::Header;
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
            append_archive_file(&mut builder, entry.path(), relative, json_output)?;
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

fn append_archive_file<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    source_path: &Path,
    relative: &Path,
    json_output: bool,
) -> Result<()> {
    let archive_path = Path::new(".").join(relative);
    if relative == Path::new("tovuk.toml") {
        let source = fs::read_to_string(source_path).map_err(|error| {
            archive_error(
                format!("Could not read tovuk.toml for deploy: {error}"),
                json_output,
            )
        })?;
        let sanitized = deploy_tovuk_toml(&source, json_output)?;
        return append_archive_bytes(builder, &archive_path, sanitized.as_bytes(), json_output);
    }

    builder
        .append_path_with_name(source_path, archive_path)
        .map_err(|error| {
            archive_error(
                format!("Check project files and retry: {error}"),
                json_output,
            )
        })
}

fn deploy_tovuk_toml(source: &str, json_output: bool) -> Result<String> {
    let mut table = source.parse::<toml::Table>().map_err(|error| {
        archive_error(
            format!("tovuk.toml became invalid before deploy archiving: {error}"),
            json_output,
        )
    })?;
    table.remove("dev");
    toml::to_string_pretty(&table).map_err(|error| {
        archive_error(
            format!("Could not serialize deploy tovuk.toml: {error}"),
            json_output,
        )
    })
}

fn append_archive_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    archive_path: &Path,
    bytes: &[u8],
    json_output: bool,
) -> Result<()> {
    let mut header = Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, archive_path, Cursor::new(bytes))
        .map_err(|error| {
            archive_error(
                format!("Check project files and retry: {error}"),
                json_output,
            )
        })
}

fn archive_error(instruction: String, json_output: bool) -> super::super::errors::CliError {
    agent_error(
        "archive_failed",
        "Could not create source archive.",
        instruction,
        json_output,
    )
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
    use super::{create_archive_base64, deploy_tovuk_toml, is_archive_excluded};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use flate2::read::GzDecoder;
    use std::{
        collections::BTreeMap,
        fs,
        io::Read as _,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

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

    #[test]
    fn deploy_tovuk_toml_strips_local_dev_config() -> Result<(), Box<dyn std::error::Error>> {
        let sanitized = deploy_tovuk_toml(
            r#"
name = "demo"
kind = "fullstack"

[dev]
worker_port = 3001
frontend_port = 5174

[capabilities]
static_frontend = true
worker = true
"#,
            true,
        )?;

        let table = sanitized.parse::<toml::Table>()?;
        if table.get("dev").is_some() {
            return Err("deploy tovuk.toml should not include [dev]".into());
        }
        if table.get("name").and_then(toml::Value::as_str) != Some("demo") {
            return Err(format!("unexpected sanitized name: {sanitized}").into());
        }
        if table.get("capabilities").is_none() {
            return Err("deploy tovuk.toml should keep [capabilities]".into());
        }
        Ok(())
    }

    #[test]
    fn source_archive_strips_local_dev_config() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project_dir("archive-strips-dev")?;
        fs::write(
            project_dir.join("tovuk.toml"),
            r#"
name = "demo"

[dev]
worker_port = 3001

[capabilities]
static_frontend = false
worker = true
"#,
        )?;
        fs::write(project_dir.join("main.rs"), "fn main() {}\n")?;

        let archive = create_archive_base64(&project_dir, true)?;
        let files = unpack_archive(&archive)?;
        let tovuk_toml =
            archive_file(&files, "tovuk.toml").ok_or("archive should include tovuk.toml")?;
        let table = tovuk_toml.parse::<toml::Table>()?;

        if table.get("dev").is_some() {
            return Err("archived tovuk.toml should not include [dev]".into());
        }
        if archive_file(&files, "main.rs") != Some("fn main() {}\n") {
            return Err(format!("archive files were wrong: {:?}", files.keys()).into());
        }

        let _ignore = fs::remove_dir_all(project_dir);
        Ok(())
    }

    fn unpack_archive(
        archive_base64: &str,
    ) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
        let bytes = BASE64.decode(archive_base64)?;
        let decoder = GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(decoder);
        let mut files = BTreeMap::new();
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().into_owned();
            let mut contents = String::new();
            entry.read_to_string(&mut contents)?;
            files.insert(path, contents);
        }
        Ok(files)
    }

    fn archive_file<'a>(files: &'a BTreeMap<String, String>, path: &str) -> Option<&'a str> {
        files
            .iter()
            .find(|(archive_path, _contents)| archive_path.trim_start_matches("./") == path)
            .map(|(_archive_path, contents)| contents.as_str())
    }

    fn temp_project_dir(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("tovuk-{name}-{nanos}"));
        let _ignore = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
