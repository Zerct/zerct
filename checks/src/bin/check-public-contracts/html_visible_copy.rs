use crate::helpers::CheckResult;

pub(crate) fn html_visible_copy(source: &str) -> String {
    let body = html_body(source);
    let mut output = String::new();
    collect_visible_text(body, &mut output);
    output
}

fn html_body(source: &str) -> &str {
    let Some(body_start) = find_tag_end(source, "body") else {
        return source;
    };
    let body_source = &source[body_start..];
    let Some(body_end) = find_closing_tag(body_source, "body") else {
        return body_source;
    };
    &body_source[..body_end]
}

fn collect_visible_text(source: &str, output: &mut String) {
    let mut index = 0;
    let mut hidden_depth = 0usize;
    while index < source.len() {
        let Some(tag_start_offset) = source[index..].find('<') else {
            push_text(&source[index..], hidden_depth, output);
            return;
        };
        let tag_start = index + tag_start_offset;
        push_text(&source[index..tag_start], hidden_depth, output);

        let Some(tag_end_offset) = source[tag_start..].find('>') else {
            push_text(&source[tag_start..], hidden_depth, output);
            return;
        };
        let tag_end = tag_start + tag_end_offset + 1;
        if let Some(tag) = html_tag(&source[tag_start + 1..tag_end - 1]) {
            if tag.closing {
                if hidden_depth > 0 && hidden_text_tag(tag.name.as_str()) {
                    hidden_depth -= 1;
                }
            } else if hidden_text_tag(tag.name.as_str()) && !tag.self_closing {
                hidden_depth += 1;
            }
        }
        index = tag_end;
    }
}

fn push_text(source: &str, hidden_depth: usize, output: &mut String) {
    if hidden_depth == 0 {
        let text = html_text_decode(source);
        if !text.trim().is_empty() {
            output.push_str(text.as_str());
            output.push(' ');
        }
    }
}

fn html_tag(raw: &str) -> Option<HtmlTag> {
    let trimmed = raw.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('?') {
        return None;
    }
    let closing = trimmed.starts_with('/');
    let name_source = if closing {
        trimmed[1..].trim_start()
    } else {
        trimmed
    };
    let name_end = name_source
        .find(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '-' || character == ':')
        })
        .unwrap_or(name_source.len());
    if name_end == 0 {
        return None;
    }
    Some(HtmlTag {
        name: name_source[..name_end].to_ascii_lowercase(),
        closing,
        self_closing: trimmed.trim_end().ends_with('/'),
    })
}

struct HtmlTag {
    name: String,
    closing: bool,
    self_closing: bool,
}

fn hidden_text_tag(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "head" | "script" | "style" | "noscript" | "template" | "svg"
    )
}

fn find_tag_end(source: &str, tag_name: &str) -> Option<usize> {
    let mut index = 0;
    while let Some(tag_start_offset) = source[index..].find('<') {
        let tag_start = index + tag_start_offset;
        let tag_end = tag_start + source[tag_start..].find('>')? + 1;
        if html_tag(&source[tag_start + 1..tag_end - 1])
            .as_ref()
            .is_some_and(|tag| !tag.closing && tag.name == tag_name)
        {
            return Some(tag_end);
        }
        index = tag_end;
    }
    None
}

fn find_closing_tag(source: &str, tag_name: &str) -> Option<usize> {
    let mut index = 0;
    while let Some(tag_start_offset) = source[index..].find('<') {
        let tag_start = index + tag_start_offset;
        let tag_end = tag_start + source[tag_start..].find('>')? + 1;
        if html_tag(&source[tag_start + 1..tag_end - 1])
            .as_ref()
            .is_some_and(|tag| tag.closing && tag.name == tag_name)
        {
            return Some(tag_start);
        }
        index = tag_end;
    }
    None
}

fn html_text_decode(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(entity_start) = rest.find('&') {
        output.push_str(&rest[..entity_start]);
        let entity_source = &rest[entity_start..];
        let Some(entity_end) = entity_source.find(';').filter(|end| *end <= 12) else {
            output.push('&');
            rest = &entity_source[1..];
            continue;
        };
        let entity = &entity_source[1..entity_end];
        if let Some(decoded) = decode_entity(entity) {
            output.push(decoded);
            rest = &entity_source[entity_end + 1..];
        } else {
            output.push('&');
            rest = &entity_source[1..];
        }
    }
    output.push_str(rest);
    output
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _unknown => decode_numeric_entity(entity).ok().flatten(),
    }
}

fn decode_numeric_entity(entity: &str) -> CheckResult<Option<char>> {
    let Some(number) = entity.strip_prefix('#') else {
        return Ok(None);
    };
    let codepoint = if let Some(hex) = number
        .strip_prefix('x')
        .or_else(|| number.strip_prefix('X'))
    {
        u32::from_str_radix(hex, 16).map_err(|error| format!("decode HTML entity: {error}"))?
    } else {
        number
            .parse::<u32>()
            .map_err(|error| format!("decode HTML entity: {error}"))?
    };
    Ok(char::from_u32(codepoint))
}

#[cfg(test)]
mod tests {
    use super::html_visible_copy;

    #[test]
    fn visible_copy_ignores_head_assets_and_hidden_tags() {
        let source = r"
            <html>
              <head><title>Zerct</title></head>
              <body>
                <h1>Tovuk &amp; agents</h1>
                <script>Zerct</script>
                <svg><text>Zerct</text></svg>
              </body>
            </html>
        ";

        let visible = html_visible_copy(source);
        assert!(visible.contains("Tovuk & agents"));
        assert!(!visible.contains("Zerct"));
    }
}
