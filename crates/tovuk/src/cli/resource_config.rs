use super::toml_values::{get_string, get_u16};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ResourceConfig {
    pub(crate) memory: String,
    pub(crate) cpu: String,
    pub(crate) idle_timeout_minutes: u16,
}

pub(crate) fn parse_resource_config(
    table: &toml::Table,
) -> std::result::Result<ResourceConfig, String> {
    Ok(ResourceConfig {
        memory: get_string(table, "memory")?.unwrap_or_else(|| "512mb".to_owned()),
        cpu: get_string(table, "cpu")?.unwrap_or_else(|| "0.25".to_owned()),
        idle_timeout_minutes: get_u16(table, "idle_timeout_minutes")?.unwrap_or(15),
    })
}

pub(crate) fn validate_resource_config(
    resources: &ResourceConfig,
) -> std::result::Result<(), String> {
    let memory_mib = memory_to_mib(&resources.memory)?;
    if !(128..=2048).contains(&memory_mib) {
        return Err(
            "[resources].memory must be between 128mb and 2gb; use the smallest working value"
                .to_owned(),
        );
    }
    let cpu_millis = cpu_to_millis(&resources.cpu)?;
    if !(50..=2000).contains(&cpu_millis) {
        return Err(
            "[resources].cpu must be between 0.05 and 2; use the smallest working value".to_owned(),
        );
    }
    if !(1..=60).contains(&resources.idle_timeout_minutes) {
        return Err("[resources].idle_timeout_minutes must be between 1 and 60".to_owned());
    }
    Ok(())
}

fn memory_to_mib(value: &str) -> std::result::Result<u32, String> {
    let clean = value.trim().to_ascii_lowercase();
    let amount = clean
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let unit = clean[amount.len()..].trim();
    let amount = amount
        .parse::<u32>()
        .map_err(|_error| "[resources].memory must look like 256mb, 512mb, or 1gb".to_owned())?;
    match unit {
        "mb" | "mib" => Ok(amount),
        "gb" | "gib" => Ok(amount * 1024),
        _ => Err("[resources].memory must look like 256mb, 512mb, or 1gb".to_owned()),
    }
}

fn cpu_to_millis(value: &str) -> std::result::Result<u32, String> {
    let clean = value.trim();
    if clean.is_empty()
        || clean
            .chars()
            .any(|character| !character.is_ascii_digit() && character != '.')
        || clean.matches('.').count() > 1
    {
        return Err("[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned());
    }
    let mut parts = clean.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<u32>()
        .map_err(|_error| "[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned())?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 3 {
        return Err("[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned());
    }
    let mut fractional_millis = 0u32;
    for (index, digit) in fraction.bytes().enumerate() {
        fractional_millis += u32::from(digit - b'0') * [100, 10, 1][index];
    }
    Ok(whole * 1000 + fractional_millis)
}
