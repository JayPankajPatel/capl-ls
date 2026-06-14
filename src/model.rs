use tower_lsp::lsp_types::{Position, Range, SymbolKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaplSymbolKind {
    Function,
    EventHandler,
    Variable,
    Type,
    Macro,
    Include,
}

impl CaplSymbolKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::EventHandler => "event handler",
            Self::Variable => "variable",
            Self::Type => "type",
            Self::Macro => "macro",
            Self::Include => "include",
        }
    }

    pub fn lsp_kind(&self) -> SymbolKind {
        match self {
            Self::Function => SymbolKind::FUNCTION,
            Self::EventHandler => SymbolKind::EVENT,
            Self::Variable => SymbolKind::VARIABLE,
            Self::Type => SymbolKind::STRUCT,
            Self::Macro => SymbolKind::CONSTANT,
            Self::Include => SymbolKind::FILE,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaplSymbol {
    pub name: String,
    pub kind: CaplSymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub detail: Option<String>,
}

impl CaplSymbol {
    pub fn new(
        name: impl Into<String>,
        kind: CaplSymbolKind,
        range: Range,
        selection_range: Range,
        detail: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            range,
            selection_range,
            detail,
        }
    }
}

pub fn single_line_range(line: u32, start_character: u32, length: u32) -> Range {
    Range {
        start: Position {
            line,
            character: start_character,
        },
        end: Position {
            line,
            character: start_character.saturating_add(length),
        },
    }
}
