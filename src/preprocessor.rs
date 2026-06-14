#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacroDefinition {
    pub name: String,
    pub parameters: Option<Vec<String>>,
    pub body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMapSegment {
    pub expanded_start: usize,
    pub expanded_len: usize,
    pub original_start: usize,
    pub original_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedText {
    pub text: String,
    pub source_map: Vec<SourceMapSegment>,
}

pub fn collect_macros(text: &str) -> Vec<MacroDefinition> {
    text.lines().filter_map(parse_define).collect()
}

pub fn expand_object_like_once(text: &str, macros: &[MacroDefinition]) -> ExpandedText {
    let mut output = String::with_capacity(text.len());
    let mut source_map = Vec::new();
    let mut index = 0;

    while index < text.len() {
        let Some((name, start, end)) = next_identifier(text, index) else {
            output.push_str(&text[index..]);
            break;
        };

        output.push_str(&text[index..start]);

        if let Some(mac) = macros
            .iter()
            .find(|mac| mac.parameters.is_none() && mac.name == name)
        {
            let expanded_start = output.len();
            output.push_str(&mac.body);
            source_map.push(SourceMapSegment {
                expanded_start,
                expanded_len: mac.body.len(),
                original_start: start,
                original_len: end - start,
            });
        } else {
            output.push_str(&text[start..end]);
        }

        index = end;
    }

    ExpandedText {
        text: output,
        source_map,
    }
}

fn parse_define(line: &str) -> Option<MacroDefinition> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("#define")?.trim_start();
    let (name, name_end) = read_identifier(rest, 0)?;
    let after_name = &rest[name_end..];

    if let Some(param_text) = after_name.strip_prefix('(') {
        let close = param_text.find(')')?;
        let parameters = param_text[..close]
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        let body = param_text[close + 1..].trim_start().to_string();

        return Some(MacroDefinition {
            name,
            parameters: Some(parameters),
            body,
        });
    }

    Some(MacroDefinition {
        name,
        parameters: None,
        body: after_name.trim_start().to_string(),
    })
}

fn next_identifier(text: &str, from: usize) -> Option<(String, usize, usize)> {
    let mut offset = from;

    while offset < text.len() {
        let ch = text[offset..].chars().next()?;
        if is_ident_start(ch) {
            let (name, len) = read_identifier(text, offset)?;
            return Some((name, offset, offset + len));
        }
        offset += ch.len_utf8();
    }

    None
}

fn read_identifier(text: &str, start: usize) -> Option<(String, usize)> {
    let mut chars = text[start..].char_indices();
    let (_, first) = chars.next()?;
    if !is_ident_start(first) {
        return None;
    }

    let mut end = start + first.len_utf8();
    for (idx, ch) in chars {
        if !is_ident_continue(ch) {
            break;
        }
        end = start + idx + ch.len_utf8();
    }

    Some((text[start..end].to_string(), end - start))
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
