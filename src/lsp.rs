use crate::model::CaplSymbol;
use crate::parser::parse_document;
use dashmap::DashMap;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, InitializeParams, InitializeResult,
    InitializedParams, Location, MarkupContent, MarkupKind, MessageType, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer};

#[derive(Clone, Debug, Default)]
struct DocumentIndex {
    symbols: Vec<CaplSymbol>,
}

pub struct Backend {
    client: Client,
    documents: DashMap<Url, String>,
    indexes: DashMap<Url, DocumentIndex>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            indexes: DashMap::new(),
        }
    }

    fn index_document(&self, uri: Url, text: String) {
        let parsed = parse_document(&text);
        self.documents.insert(uri.clone(), text);
        self.indexes.insert(
            uri,
            DocumentIndex {
                symbols: parsed.symbols,
            },
        );
    }

    fn find_symbol(&self, uri: &Url, token: &str) -> Option<(Url, CaplSymbol)> {
        if let Some(index) = self.indexes.get(uri) {
            if let Some(symbol) = index.symbols.iter().find(|symbol| symbol.name == token) {
                return Some((uri.clone(), symbol.clone()));
            }
        }

        for entry in self.indexes.iter() {
            if let Some(symbol) = entry.symbols.iter().find(|symbol| symbol.name == token) {
                return Some((entry.key().clone(), symbol.clone()));
            }
        }

        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "capl-ls initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        self.index_document(params.text_document.uri, params.text_document.text);
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.index_document(params.text_document.uri, change.text);
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(index) = self.indexes.get(&params.text_document.uri) else {
            return Ok(Some(DocumentSymbolResponse::Nested(Vec::new())));
        };

        let symbols = index.symbols.iter().map(to_document_symbol).collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position_params = params.text_document_position_params;
        let uri = position_params.text_document.uri;
        let Some(text) = self.documents.get(&uri) else {
            return Ok(None);
        };

        let Some(token) = token_at_position(&text, position_params.position) else {
            return Ok(None);
        };

        let Some((definition_uri, symbol)) = self.find_symbol(&uri, &token) else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: definition_uri,
            range: symbol.selection_range,
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position_params = params.text_document_position_params;
        let uri = position_params.text_document.uri;
        let Some(text) = self.documents.get(&uri) else {
            return Ok(None);
        };

        let Some(token) = token_at_position(&text, position_params.position) else {
            return Ok(None);
        };

        let Some((_, symbol)) = self.find_symbol(&uri, &token) else {
            return Ok(None);
        };

        let mut value = format!("**{}** `{}`", symbol.kind.label(), symbol.name);
        if let Some(detail) = &symbol.detail {
            value.push_str("\n\n```capl\n");
            value.push_str(detail);
            value.push_str("\n```");
        }

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(symbol.selection_range),
        }))
    }
}

fn to_document_symbol(symbol: &CaplSymbol) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: symbol.name.clone(),
        detail: symbol
            .detail
            .clone()
            .or_else(|| Some(symbol.kind.label().to_string())),
        kind: symbol.kind.lsp_kind(),
        tags: None,
        deprecated: None,
        range: symbol.range,
        selection_range: symbol.selection_range,
        children: None,
    }
}

fn token_at_position(text: &str, position: tower_lsp::lsp_types::Position) -> Option<String> {
    let line = text.lines().nth(position.line as usize)?;
    let chars = line.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return None;
    }

    let mut cursor = (position.character as usize).min(chars.len());
    if cursor == chars.len() || !is_token_char(chars[cursor]) {
        if cursor == 0 || !is_token_char(chars[cursor - 1]) {
            return None;
        }
        cursor -= 1;
    }

    let mut start = cursor;
    while start > 0 && is_token_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = cursor;
    while end < chars.len() && is_token_char(chars[end]) {
        end += 1;
    }

    let raw = chars[start..end].iter().collect::<String>();
    let token = raw.trim_start_matches(['$', '@']).to_string();

    (!token.is_empty()).then_some(token)
}

fn is_token_char(ch: char) -> bool {
    ch == '_' || ch == '$' || ch == '@' || ch.is_ascii_alphanumeric()
}

#[allow(dead_code)]
fn _json_null() -> Option<Value> {
    None
}
