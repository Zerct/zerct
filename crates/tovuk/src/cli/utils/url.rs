use core::fmt::Write as _;

/// Percent-encodes a value for use in one URL path component.
pub(in crate::cli) fn encode_component(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(char::from(byte));
            continue;
        }
        if write!(output, "%{byte:02X}").is_err() {
            return output;
        }
    }
    return output;
}
