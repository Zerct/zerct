use super::toml_values::{get_string, get_u16};
use serde::Serialize;

const WORKER_MEMORY: &str = "128mb";
const WORKER_MEMORY_MIB: u32 = 128;
const WORKER_CPU: &str = "1";
const WORKER_CPU_MILLIS: u32 = 1_000;

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
        memory: get_string(table, "memory")?.unwrap_or_else(|| WORKER_MEMORY.to_owned()),
        cpu: get_string(table, "cpu")?.unwrap_or_else(|| WORKER_CPU.to_owned()),
        idle_timeout_minutes: get_u16(table, "idle_timeout_minutes")?.unwrap_or(15),
    })
}

pub(crate) fn validate_resource_config(
    resources: &ResourceConfig,
) -> std::result::Result<(), String> {
    let memory_mib = memory_to_mib(&resources.memory)?;
    if memory_mib != WORKER_MEMORY_MIB {
        return Err(
            "[resources].memory must be 128mb for Cloudflare-compatible workers".to_owned(),
        );
    }
    let cpu_millis = cpu_to_millis(&resources.cpu)?;
    if cpu_millis != WORKER_CPU_MILLIS {
        return Err(
            "[resources].cpu must be 1; control CPU budget with worker_cpu_ms usage caps"
                .to_owned(),
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
        .map_err(|_error| "[resources].memory must look like 128mb".to_owned())?;
    match unit {
        "mb" | "mib" => Ok(amount),
        "gb" | "gib" => Ok(amount * 1024),
        _ => Err("[resources].memory must look like 128mb".to_owned()),
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
        return Err("[resources].cpu must be 1".to_owned());
    }
    let mut parts = clean.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<u32>()
        .map_err(|_error| "[resources].cpu must be 1".to_owned())?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 3 {
        return Err("[resources].cpu must be 1".to_owned());
    }
    let mut fractional_millis = 0u32;
    for (index, digit) in fraction.bytes().enumerate() {
        fractional_millis += u32::from(digit - b'0') * [100, 10, 1][index];
    }
    Ok(whole * 1000 + fractional_millis)
}
