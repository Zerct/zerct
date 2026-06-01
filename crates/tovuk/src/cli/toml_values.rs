pub(crate) fn get_section(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<toml::map::Map<String, toml::Value>, String> {
    match table.get(key) {
        None => Ok(toml::map::Map::new()),
        Some(value) => value
            .as_table()
            .cloned()
            .ok_or_else(|| format!("[{key}] must be a table")),
    }
}

pub(crate) fn get_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<String>, String> {
    table.get(key).map_or(Ok(None), |value| {
        value
            .as_str()
            .map(|value| Some(value.to_owned()))
            .ok_or_else(|| format!("{key} must be a string"))
    })
}

pub(crate) fn get_u16(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<u16>, String> {
    table.get(key).map_or(Ok(None), |value| {
        let integer = value
            .as_integer()
            .ok_or_else(|| format!("{key} must be a number"))?;
        u16::try_from(integer)
            .ok()
            .map(Some)
            .ok_or_else(|| format!("{key} must be between 0 and 65535"))
    })
}

pub(crate) fn get_bool(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<bool>, String> {
    table.get(key).map_or(Ok(None), |value| {
        value
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("{key} must be true or false"))
    })
}

pub(crate) fn reject_unknown_section_keys(
    table: &toml::map::Map<String, toml::Value>,
    section: &str,
    allowed: &[&str],
) -> std::result::Result<(), String> {
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported [{section}] key {key}"));
        }
    }
    Ok(())
}
