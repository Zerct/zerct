use crate::helpers::CheckResult;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0009] = [
    size_of_val(&collect_visible_text),
    size_of_val(&decode_entity),
    size_of_val(&decode_numeric_entity),
    size_of_val(&find_closing_tag),
    size_of_val(&find_tag_end),
    size_of_val(&html_body),
    size_of_val(&html_text_decode),
    size_of_val(&html_visible_copy),
    size_of_val(&collect_visible_text_hidden_depth),
];

/// Contract representation for `HtmlTag`.
struct HtmlTag {
    /// Whether the tag is paired or self-closing.
    closure: TagClosure,
    /// Whether the tag opens or closes an element.
    direction: TagDirection,
    /// Contract data stored in `name`.
    name: String,
}

/// Element closure behavior parsed from an HTML tag.
#[derive(Clone, Copy, Debug)]
enum TagClosure {
    /// The tag participates in an opening/closing pair.
    Paired,
    /// The tag closes itself.
    SelfClosing,
}

/// Direction parsed from an HTML tag.
#[derive(Clone, Copy, Debug)]
enum TagDirection {
    /// The tag closes an element.
    Closing,
    /// The tag opens an element.
    Opening,
}

/// Contract implementation for `collect_visible_text`.
pub(super) fn collect_visible_text(source: &str, output: &mut String) {
    let mut index = 0;
    let mut hidden_depth: usize = 0x0000;
    while index < source.len() {
        let Some(tag_start_offset) = string_suffix(source, index).find('<') else {
            push_text(string_suffix(source, index), hidden_depth, output);
            return;
        };
        let tag_start = index.saturating_add(tag_start_offset);
        push_text(string_range(source, index, tag_start), hidden_depth, output);

        let Some(tag_end_offset) = string_suffix(source, tag_start).find('>') else {
            push_text(string_suffix(source, tag_start), hidden_depth, output);
            return;
        };
        let tag_end = tag_start
            .saturating_add(tag_end_offset)
            .saturating_add(0x0001);
        if let Some(tag) = html_tag(string_range(
            source,
            tag_start.saturating_add(0x0001),
            tag_end.saturating_sub(0x0001),
        )) {
            hidden_depth = collect_visible_text_hidden_depth(&tag, hidden_depth);
        }
        index = tag_end;
    }
}

/// Update hidden-text nesting after one parsed tag.
fn collect_visible_text_hidden_depth(tag: &HtmlTag, hidden_depth: usize) -> usize {
    if matches!(tag.direction, TagDirection::Closing)
        && hidden_depth > 0
        && hidden_text_tag(tag.name.as_str())
    {
        return hidden_depth.saturating_sub(0x0001);
    }
    if matches!(tag.direction, TagDirection::Opening)
        && hidden_text_tag(tag.name.as_str())
        && matches!(tag.closure, TagClosure::Paired)
    {
        return hidden_depth.saturating_add(0x0001);
    }
    return hidden_depth;
}

/// Contract implementation for `decode_entity`.
pub(super) fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => return Some('&'),
        "lt" => return Some('<'),
        "gt" => return Some('>'),
        "quot" => return Some('"'),
        "apos" => return Some('\''),
        "nbsp" => return Some(' '),
        _unknown => return decode_numeric_entity(entity).ok().flatten(),
    }
}

/// Contract implementation for `decode_numeric_entity`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn decode_numeric_entity(entity: &str) -> CheckResult<Option<char>> {
    let Some(number) = entity.strip_prefix('#') else {
        return Ok(None);
    };
    let codepoint = if let Some(hex) = number
        .strip_prefix('x')
        .or_else(|| return number.strip_prefix('X'))
    {
        check_try!(
            u32::from_str_radix(hex, 0x0010)
                .map_err(|error| format!("decode HTML entity: {error}"))
        )
    } else {
        check_try!(
            number
                .parse::<u32>()
                .map_err(|error| format!("decode HTML entity: {error}"))
        )
    };
    return Ok(char::from_u32(codepoint));
}

/// Contract implementation for `find_closing_tag`.
pub(super) fn find_closing_tag(source: &str, tag_name: &str) -> Option<usize> {
    let mut index = 0;
    while let Some(tag_start_offset) = string_suffix(source, index).find('<') {
        let tag_start = index.saturating_add(tag_start_offset);
        let tag_end = tag_start
            .saturating_add(check_some!(string_suffix(source, tag_start).find('>')))
            .saturating_add(0x0001);
        if html_tag(string_range(
            source,
            tag_start.saturating_add(0x0001),
            tag_end.saturating_sub(0x0001),
        ))
        .as_ref()
        .is_some_and(|tag| {
            return matches!(tag.direction, TagDirection::Closing) && tag.name == tag_name;
        }) {
            return Some(tag_start);
        }
        index = tag_end;
    }
    return None;
}

/// Contract implementation for `find_tag_end`.
pub(super) fn find_tag_end(source: &str, tag_name: &str) -> Option<usize> {
    let mut index = 0;
    while let Some(tag_start_offset) = string_suffix(source, index).find('<') {
        let tag_start = index.saturating_add(tag_start_offset);
        let tag_end = tag_start
            .saturating_add(check_some!(string_suffix(source, tag_start).find('>')))
            .saturating_add(0x0001);
        if html_tag(string_range(
            source,
            tag_start.saturating_add(0x0001),
            tag_end.saturating_sub(0x0001),
        ))
        .as_ref()
        .is_some_and(|tag| {
            return matches!(tag.direction, TagDirection::Opening) && tag.name == tag_name;
        }) {
            return Some(tag_end);
        }
        index = tag_end;
    }
    return None;
}

/// Contract implementation for `hidden_text_tag`.
fn hidden_text_tag(tag_name: &str) -> bool {
    return matches!(
        tag_name,
        "head" | "script" | "style" | "noscript" | "template" | "svg"
    );
}

/// Contract implementation for `html_body`.
pub(super) fn html_body(source: &str) -> &str {
    let Some(body_start) = find_tag_end(source, "body") else {
        return source;
    };
    let body_source = string_suffix(source, body_start);
    let Some(body_end) = find_closing_tag(body_source, "body") else {
        return body_source;
    };
    return string_prefix(body_source, body_end);
}

/// Contract implementation for `html_tag`.
fn html_tag(raw: &str) -> Option<HtmlTag> {
    let trimmed = raw.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('?') {
        return None;
    }
    let direction = if trimmed.starts_with('/') {
        TagDirection::Closing
    } else {
        TagDirection::Opening
    };
    let name_source = if matches!(direction, TagDirection::Closing) {
        string_suffix(trimmed, 0x0001).trim_start()
    } else {
        trimmed
    };
    let name_end = name_source
        .find(|character: char| {
            return !(character.is_ascii_alphanumeric() || character == '-' || character == ':');
        })
        .unwrap_or(name_source.len());
    if name_end == 0 {
        return None;
    }
    return Some(HtmlTag {
        closure: if trimmed.trim_end().ends_with('/') {
            TagClosure::SelfClosing
        } else {
            TagClosure::Paired
        },
        direction,
        name: string_prefix(name_source, name_end).to_ascii_lowercase(),
    });
}

/// Contract implementation for `html_text_decode`.
pub(super) fn html_text_decode(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(entity_start) = rest.find('&') {
        output.push_str(string_prefix(rest, entity_start));
        let entity_source = string_suffix(rest, entity_start);
        let Some(entity_end) = entity_source.find(';').filter(|end| return *end <= 12) else {
            output.push('&');
            rest = string_suffix(entity_source, 0x0001);
            continue;
        };
        let entity = string_range(entity_source, 0x0001, entity_end);
        if let Some(decoded) = decode_entity(entity) {
            output.push(decoded);
            rest = string_suffix(entity_source, entity_end.saturating_add(0x0001));
        } else {
            output.push('&');
            rest = string_suffix(entity_source, 0x0001);
        }
    }
    output.push_str(rest);
    return output;
}

/// Contract implementation for `html_visible_copy`.
pub(super) fn html_visible_copy(source: &str) -> String {
    let body = html_body(source);
    let mut output = String::new();
    collect_visible_text(body, &mut output);
    return output;
}

/// Contract implementation for `push_text`.
fn push_text(source: &str, hidden_depth: usize, output: &mut String) {
    if hidden_depth == 0 {
        let text = html_text_decode(source);
        if !text.trim().is_empty() {
            output.push_str(text.as_str());
            output.push(' ');
        }
    }
}

/// Return the prefix ending at a known character boundary.
const fn string_prefix(source: &str, end: usize) -> &str {
    return source.split_at(end).0;
}

/// Return the substring between two known character boundaries.
const fn string_range(source: &str, start: usize, end: usize) -> &str {
    return string_prefix(string_suffix(source, start), end.saturating_sub(start));
}

/// Return the suffix starting at a known character boundary.
const fn string_suffix(source: &str, start: usize) -> &str {
    return source.split_at(start).1;
}
#[cfg(test)]
#[path = "html_visible_copy_tests/verification.rs"]
mod tests;
