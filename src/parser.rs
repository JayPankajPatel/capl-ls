use crate::model::{single_line_range, CaplSymbol, CaplSymbolKind};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParseResult {
    pub symbols: Vec<CaplSymbol>,
}

pub fn parse_document(text: &str) -> ParseResult {
    let mut symbols = Vec::new();
    let mut in_variables = false;
    let mut variable_depth = 0_i32;
    let mut nested_type_depth = 0_i32;

    for (line_index, line) in text.lines().enumerate() {
        let line_no = line_index as u32;
        let code = strip_line_comment(line);
        let trimmed = code.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some(symbol) = parse_macro(line_no, line) {
            symbols.push(symbol);
            continue;
        }

        if let Some(symbol) = parse_include(line_no, line) {
            symbols.push(symbol);
            continue;
        }

        if in_variables {
            if nested_type_depth > 0 {
                nested_type_depth += brace_delta(trimmed);
                if nested_type_depth <= 0 {
                    nested_type_depth = 0;
                }
                variable_depth += brace_delta(trimmed);
                if variable_depth <= 0 && trimmed.contains('}') {
                    in_variables = false;
                    variable_depth = 0;
                }
                continue;
            }

            if let Some((symbol, type_depth)) = parse_type_decl(line_no, line, trimmed) {
                symbols.push(symbol);
                nested_type_depth = type_depth;
                variable_depth += brace_delta(trimmed);
                if variable_depth <= 0 && trimmed.contains('}') {
                    in_variables = false;
                    variable_depth = 0;
                }
                continue;
            }

            if let Some(symbol) = parse_variable_decl(line_no, line, trimmed) {
                symbols.push(symbol);
            }

            variable_depth += brace_delta(trimmed);
            if variable_depth <= 0 && trimmed.contains('}') {
                in_variables = false;
                variable_depth = 0;
            }
            continue;
        }

        if is_section_start(trimmed, "variables") {
            in_variables = true;
            variable_depth = brace_delta(trimmed).max(0);
            continue;
        }

        if let Some(symbol) = parse_event_handler(line_no, line, trimmed) {
            symbols.push(symbol);
            continue;
        }

        if let Some((symbol, _)) = parse_type_decl(line_no, line, trimmed) {
            symbols.push(symbol);
            continue;
        }

        if let Some(symbol) = parse_function_decl(line_no, line, trimmed) {
            symbols.push(symbol);
        }
    }

    ParseResult { symbols }
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

fn brace_delta(line: &str) -> i32 {
    line.chars().filter(|c| *c == '{').count() as i32
        - line.chars().filter(|c| *c == '}').count() as i32
}

fn is_section_start(trimmed: &str, keyword: &str) -> bool {
    trimmed
        .split(|c: char| c.is_whitespace() || c == '{')
        .next()
        .is_some_and(|word| word.eq_ignore_ascii_case(keyword))
}

fn parse_macro(line_no: u32, line: &str) -> Option<CaplSymbol> {
    let define_start = line.find("#define")?;
    let after_define = define_start + "#define".len();
    let rest = &line[after_define..];
    let leading_ws = rest.len() - rest.trim_start().len();
    let name_start = after_define + leading_ws;
    let name = identifier_at(line, name_start)?;

    let range = single_line_range(line_no, define_start as u32, line.chars().count() as u32);
    let selection_range = single_line_range(line_no, name_start as u32, name.len() as u32);

    Some(CaplSymbol::new(
        name,
        CaplSymbolKind::Macro,
        range,
        selection_range,
        Some(line.trim().to_string()),
    ))
}

fn parse_include(line_no: u32, line: &str) -> Option<CaplSymbol> {
    let include_start = line.find("#include")?;
    let after_include = include_start + "#include".len();
    let rest = line[after_include..].trim_start();
    let delimiter = rest.chars().next()?;
    let closing = match delimiter {
        '"' => '"',
        '<' => '>',
        _ => return None,
    };

    let path_start = after_include + (line[after_include..].len() - rest.len()) + 1;
    let path_end = line[path_start..].find(closing)? + path_start;
    let path = &line[path_start..path_end];

    let range = single_line_range(line_no, include_start as u32, line.chars().count() as u32);
    let selection_range = single_line_range(line_no, path_start as u32, path.len() as u32);

    Some(CaplSymbol::new(
        path,
        CaplSymbolKind::Include,
        range,
        selection_range,
        Some("#include".to_string()),
    ))
}

fn parse_event_handler(line_no: u32, line: &str, trimmed: &str) -> Option<CaplSymbol> {
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("on ") {
        return None;
    }

    let signature = trimmed
        .split('{')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .to_string();
    let start = line.find("on").unwrap_or(0);
    let range = single_line_range(line_no, start as u32, line.chars().count() as u32);
    let selection_range = single_line_range(line_no, start as u32, signature.len() as u32);

    Some(CaplSymbol::new(
        signature,
        CaplSymbolKind::EventHandler,
        range,
        selection_range,
        None,
    ))
}

fn parse_type_decl(line_no: u32, line: &str, trimmed: &str) -> Option<(CaplSymbol, i32)> {
    let keyword = if trimmed.starts_with("struct ") {
        "struct"
    } else if trimmed.starts_with("enum ") {
        "enum"
    } else {
        return None;
    };

    let keyword_start = line.find(keyword).unwrap_or(0);
    let name_start = keyword_start + keyword.len() + 1;
    let name = identifier_at(line, name_start)?;
    let selection_start = line[name_start..].find(&name)? + name_start;
    let range = single_line_range(line_no, keyword_start as u32, line.chars().count() as u32);
    let selection_range = single_line_range(line_no, selection_start as u32, name.len() as u32);
    let type_depth = brace_delta(trimmed).max(0);

    Some((
        CaplSymbol::new(
            name,
            CaplSymbolKind::Type,
            range,
            selection_range,
            Some(keyword.to_string()),
        ),
        type_depth,
    ))
}

fn parse_variable_decl(line_no: u32, line: &str, trimmed: &str) -> Option<CaplSymbol> {
    if !trimmed.contains(';')
        || trimmed.starts_with('{')
        || trimmed.starts_with('}')
        || trimmed.starts_with('$')
        || trimmed.contains('<')
    {
        return None;
    }

    let declaration = trimmed.split(';').next()?.split('=').next()?.trim();
    let tokens = identifier_spans(declaration);
    let (name, relative_start) = tokens.last()?.clone();
    let declaration_start = line.find(declaration).unwrap_or(0);
    let absolute_start = declaration_start + relative_start;

    if is_type_keyword(&name) {
        return None;
    }

    let range = single_line_range(line_no, declaration_start as u32, declaration.len() as u32);
    let selection_range = single_line_range(line_no, absolute_start as u32, name.len() as u32);

    Some(CaplSymbol::new(
        name,
        CaplSymbolKind::Variable,
        range,
        selection_range,
        None,
    ))
}

fn parse_function_decl(line_no: u32, line: &str, trimmed: &str) -> Option<CaplSymbol> {
    if trimmed.ends_with(';') || trimmed.starts_with('#') || trimmed.contains('=') {
        return None;
    }

    let open_paren = trimmed.find('(')?;
    let before_paren = trimmed[..open_paren].trim_end();
    let tokens = identifier_spans(before_paren);
    let (name, relative_start) = tokens.last()?.clone();

    if is_control_keyword(&name) || is_type_keyword(&name) {
        return None;
    }

    let declaration_start = line.find(before_paren).unwrap_or(0);
    let absolute_start = declaration_start + relative_start;
    let range = single_line_range(
        line_no,
        declaration_start as u32,
        line.chars().count() as u32,
    );
    let selection_range = single_line_range(line_no, absolute_start as u32, name.len() as u32);

    Some(CaplSymbol::new(
        name,
        CaplSymbolKind::Function,
        range,
        selection_range,
        None,
    ))
}

fn identifier_at(line: &str, start: usize) -> Option<String> {
    let mut chars = line[start..].char_indices();
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

    Some(line[start..end].to_string())
}

fn identifier_spans(text: &str) -> Vec<(String, usize)> {
    let mut spans = Vec::new();
    let mut iter = text.char_indices().peekable();

    while let Some((start, ch)) = iter.next() {
        if !is_ident_start(ch) {
            continue;
        }

        let mut end = start + ch.len_utf8();
        while let Some((idx, next)) = iter.peek().copied() {
            if !is_ident_continue(next) {
                break;
            }
            iter.next();
            end = idx + next.len_utf8();
        }

        spans.push((text[start..end].to_string(), start));
    }

    spans
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_control_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "for" | "while" | "switch" | "return" | "sizeof" | "else"
    )
}

fn is_type_keyword(word: &str) -> bool {
    matches!(
        word,
        "byte"
            | "word"
            | "dword"
            | "qword"
            | "int"
            | "long"
            | "int64"
            | "float"
            | "double"
            | "char"
            | "message"
            | "timer"
            | "msTimer"
            | "enum"
            | "struct"
            | "const"
            | "void"
            | "testcase"
    )
}
