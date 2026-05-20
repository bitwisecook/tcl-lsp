//! Native LSP server backend for Tcl.
//!
//! Exposes a [`Backend`] that implements [`tower_lsp::LanguageServer`]
//! and is wrapped in an `LspService` by the binary. This crate is the
//! second consumer of [`tcl_lsp_core`] (the first is `tcl-lsp-rust`),
//! so the pure-Rust crate boundary now has both production drivers
//! exercising it.
//!
//! ARCH8 ships the bootstrap with the folding-range provider wired
//! end-to-end; the `S-document-symbols` follow-up adds the
//! document-symbol provider on top.  Every other LSP method returns
//! [`tower_lsp::jsonrpc::ErrorCode::MethodNotFound`] until later
//! chunks extend the provider set.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::call_hierarchy as core_call_hierarchy;
use tcl_lsp_core::code_actions as core_code_actions;
use tcl_lsp_core::code_lens as core_code_lens;
use tcl_lsp_core::completion::{
    self as core_completion, CompletionItem as CoreCompletionItem,
    CompletionKind as CoreCompletionKind,
};
use tcl_lsp_core::definition::{self as core_definition, LspRange as CoreLspRange};
use tcl_lsp_core::document_links as core_document_links;
use tcl_lsp_core::document_symbols::{self as core_symbols, SymbolKind as CoreSymbolKind};
use tcl_lsp_core::folding::FoldKind;
use tcl_lsp_core::formatting as core_formatting;
use tcl_lsp_core::hover::{self as core_hover, Hover as CoreHover, HoverKind as CoreHoverKind};
use tcl_lsp_core::inlay_hints as core_inlay_hints;
use tcl_lsp_core::linked_editing_range as core_linked_editing_range;
use tcl_lsp_core::minify as core_minify;
use tcl_lsp_core::references as core_references;
use tcl_lsp_core::rename as core_rename;
use tcl_lsp_core::selection_range as core_selection_range;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tcl_lsp_core::signature_help::{
    self as core_sig, ParameterInformation as CoreParameterInformation,
    SignatureHelp as CoreSignatureHelp, SignatureInformation as CoreSignatureInformation,
};
use tcl_lsp_core::workspace_index as core_workspace_index;
// type_hierarchy core provider lands when tower-lsp's
// LanguageServer trait exposes the type-hierarchy methods.
// Module is registered in `tcl_lsp_core` for downstream
// callers; intentionally not imported here yet.
// use tcl_lsp_core::type_hierarchy as core_type_hierarchy;
use tcl_lsp_core::workspace_symbols::{
    self as core_workspace_symbols, WorkspaceSymbolKind as CoreWorkspaceSymbolKind,
};
use tcl_registry::dialects::DialectSet;
use tcl_registry::CommandRegistry;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::{
    request::{
        GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
        GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
    },
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CallHierarchyServerCapability, CodeAction, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeLens, CodeLensOptions, CodeLensParams, CompletionItem,
    CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    DeclarationCapability, DiagnosticOptions, DiagnosticServerCapabilities,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, DocumentFormattingParams, DocumentHighlight,
    DocumentHighlightKind, DocumentHighlightParams, DocumentLink, DocumentLinkOptions,
    DocumentLinkParams, DocumentRangeFormattingParams, DocumentSymbol, DocumentSymbolParams,
    DocumentSymbolResponse, Documentation, ExecuteCommandOptions, ExecuteCommandParams,
    FoldingRange, FoldingRangeKind, FoldingRangeParams, FoldingRangeProviderCapability,
    FullDocumentDiagnosticReport, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, ImplementationProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, InlayHint, InlayHintKind,
    InlayHintLabel, InlayHintParams, LinkedEditingRangeParams, LinkedEditingRanges, Location,
    MarkupContent, MarkupKind, MessageType, OneOf, ParameterInformation, ParameterLabel, Position,
    PrepareRenameResponse, Range, ReferenceParams, RelatedFullDocumentDiagnosticReport,
    RenameOptions, RenameParams, SelectionRange, SelectionRangeParams,
    SelectionRangeProviderCapability, SemanticTokens as LspSemanticTokens, SemanticTokensDelta,
    SemanticTokensDeltaParams, SemanticTokensEdit, SemanticTokensFullDeltaResult,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensRangeParams, SemanticTokensRangeResult, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, SignatureHelp,
    SignatureHelpOptions, SignatureHelpParams, SignatureInformation, SymbolInformation, SymbolKind,
    TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit,
    TypeDefinitionProviderCapability, Url, WorkDoneProgressOptions, WorkspaceEdit,
    WorkspaceSymbolParams,
};
use tower_lsp::{Client, LanguageServer};

/// Document store value: source text + dialect string.
#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    dialect: String,
}

impl DocumentState {
    fn new(text: String, dialect: String) -> Self {
        Self { text, dialect }
    }
}

/// LSP server backend.
///
/// Holds the LSP `Client` for outbound notifications, a document
/// store keyed on URL, and a per-dialect [`CommandRegistry`]
/// cache. Constructed once per LSP session by
/// [`tower_lsp::LspService::new`].
///
/// Each dialect gets its own cached registry built lazily on the
/// first request that names it — feature providers in
/// `tcl-lsp-core` take a `&CommandRegistry` so no per-request
/// rebuild is needed. The cache also owns the plain-Tcl base
/// registry under the empty-string key, so registries are dropped
/// with the `Backend` rather than leaked for the process lifetime.
pub struct Backend {
    client: Client,
    documents: Mutex<HashMap<Url, DocumentState>>,
    /// Fallback dialect string used when ``did_open`` cannot derive
    /// one from the ``languageId`` and no per-session
    /// ``workspace/didChangeConfiguration`` has been received yet.
    /// Updated by ``did_change_configuration`` so editor reconfigures
    /// take effect for subsequently-opened documents.
    default_dialect: Mutex<String>,
    dialect_registries: Mutex<HashMap<String, Arc<CommandRegistry>>>,
    /// Workspace folder roots received from `initialize` /
    /// `workspace/didChangeWorkspaceFolders`.  Stored as
    /// `Url` (typically `file://...` directories).
    /// `S-workspace-init` minimal port — the folder list
    /// supports cross-document features (workspace symbols
    /// already walks every cached document; future
    /// `S-workspace-symbols-rich` extends this by scanning
    /// folder contents on disk).
    workspace_folders: Mutex<Vec<Url>>,
    /// Optional per-folder dialect override map keyed on the
    /// folder URL prefix (typically `file://...`).  Populated
    /// from the `initializationOptions.folderDialects` JSON
    /// object when present.  `S-workspace-init` / `SYNC-MAY19-
    /// dialect-contextvar`: enables multi-folder workspaces
    /// with mixed dialects to parse correctly by selecting the
    /// longest-prefix folder's dialect when a document is
    /// opened.
    folder_dialects: Mutex<Vec<(Url, String)>>,
    /// Cached `AnalysisResult` per document, populated by
    /// `did_open` / `did_change` and consumed by request
    /// handlers that previously re-analysed on every call.
    /// `S-async-diagnostics` cached-analysis surface — once
    /// this entry exists, request handlers consult it before
    /// falling back to a fresh `analyser.analyse(...)`.
    analyses: Mutex<HashMap<Url, tcl_compiler::analyser::AnalysisResult>>,
    /// `S-hover-sync11`: LRU(256) cache of hover responses
    /// keyed on `(uri, line, character)`.  Invalidated per
    /// URI on `did_change` / `did_close` so stale answers
    /// don't outlive the document version that produced them.
    /// `None` entries record "no hover here" so repeated
    /// requests for the same empty position are also fast.
    hover_cache: Mutex<HoverCache>,
    /// Per-URI semantic-tokens delta cache.  Records the last
    /// `result_id` and packed token stream we returned for
    /// each document so `semanticTokens/full/delta` can short-
    /// circuit when nothing changed.  Invalidated on
    /// `did_change` / `did_close`.
    semantic_tokens_cache: Mutex<HashMap<Url, SemanticTokensEntry>>,
    /// Cross-document proc / class definition index, maintained
    /// incrementally as documents open / change / close.  Lets
    /// completion enumerate procs from sibling files and
    /// (later) cross-document go-to-definition resolve symbols
    /// defined elsewhere.
    workspace_index: Mutex<core_workspace_index::WorkspaceIndex>,
}

/// Cached semantic-tokens result keyed on the URI.
#[derive(Debug, Clone)]
struct SemanticTokensEntry {
    /// `result_id` returned to the client; bumped on every
    /// recompute so clients can request deltas against the
    /// stored snapshot.
    result_id: String,
    /// Packed `(deltaLine, deltaCol, length, type, modifiers)`
    /// stream — same shape `tcl_lsp_core::semantic_tokens`
    /// emits.
    data: Vec<u32>,
}

/// Result of a `HoverCache::get` lookup.
#[derive(Debug, Clone)]
enum HoverLookup {
    /// Cache miss — the caller must compute the hover.
    Miss,
    /// Cached "no hover here" answer (the provider returned
    /// `None` for this position last time).
    HitEmpty,
    /// Cached hover response.
    Hit(tcl_lsp_core::hover::Hover),
}

/// LRU cache of hover responses bounded at 256 entries.
/// Keys are `(uri, line, character)`.  Stored values
/// distinguish between "no entry" and "entry with empty
/// hover" so repeated requests on positions that have no
/// hover are also fast.  On capacity overflow the oldest
/// entry is evicted.
#[derive(Debug, Default)]
struct HoverCache {
    /// FIFO queue of cache keys in insertion order.  Doubles
    /// as the eviction order — bounded LRU; reads don't
    /// promote.  256 is the SYNC11-mandated cap.
    order: std::collections::VecDeque<(Url, u32, u32)>,
    entries: HashMap<(Url, u32, u32), Option<tcl_lsp_core::hover::Hover>>,
}

impl HoverCache {
    const CAP: usize = 256;

    fn get(&self, key: &(Url, u32, u32)) -> HoverLookup {
        match self.entries.get(key) {
            None => HoverLookup::Miss,
            Some(None) => HoverLookup::HitEmpty,
            Some(Some(h)) => HoverLookup::Hit(h.clone()),
        }
    }

    fn put(&mut self, key: (Url, u32, u32), value: Option<tcl_lsp_core::hover::Hover>) {
        use std::collections::hash_map::Entry;
        match self.entries.entry(key.clone()) {
            Entry::Occupied(mut e) => {
                e.insert(value);
            }
            Entry::Vacant(slot) => {
                if self.order.len() >= Self::CAP {
                    if let Some(old) = self.order.pop_front() {
                        // The slot we're about to fill matches
                        // `key`; the popped key is different, so
                        // remove it via a direct lookup.
                        if old != key {
                            // The Vacant binding holds the entry,
                            // so we can't remove via the same map
                            // without giving it up.  Stash the key,
                            // drop the entry, then evict, then
                            // re-insert.
                            drop(slot);
                            self.entries.remove(&old);
                            self.order.push_back(key.clone());
                            self.entries.insert(key, value);
                            return;
                        }
                    }
                }
                slot.insert(value);
                self.order.push_back(key);
            }
        }
    }

    fn invalidate_uri(&mut self, uri: &Url) {
        self.order.retain(|(u, _, _)| u != uri);
        self.entries.retain(|(u, _, _), _| u != uri);
    }
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish_non_exhaustive()
    }
}

impl Backend {
    /// Build a fresh backend wrapping the given client.
    ///
    /// Dialect-specific registries (including the plain-Tcl base)
    /// are loaded lazily from [`Backend::registry_for_dialect`] on
    /// the first request that names them. Pre-loading every
    /// dialect into one shared registry would let dialect-specific
    /// command overrides (e.g. iRules's `proc` shape) shadow the
    /// Tcl spec for requests on unrelated dialects.
    ///
    /// `default_dialect` is the dialect string forwarded to
    /// providers when the document store has no override.
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            dialect_registries: Mutex::new(HashMap::new()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            analyses: Mutex::new(HashMap::new()),
            hover_cache: Mutex::new(HoverCache::default()),
            semantic_tokens_cache: Mutex::new(HashMap::new()),
            workspace_index: Mutex::new(core_workspace_index::WorkspaceIndex::new()),
        }
    }

    /// Read the cached `AnalysisResult` for `uri` if one
    /// exists.  Returns a clone so the caller can run on a
    /// `spawn_blocking` worker without holding the mutex.
    /// Falls back to `None` when no cache entry exists; the
    /// caller is expected to compute a fresh analysis in
    /// that case.
    async fn cached_analysis(&self, uri: &Url) -> Option<tcl_compiler::analyser::AnalysisResult> {
        self.analyses.lock().await.get(uri).cloned()
    }

    /// Resolve an analysis for the document.  Consults the
    /// cache first; computes a fresh analysis when no entry
    /// exists.  Returns owned data the caller can move into
    /// a `spawn_blocking` worker.
    async fn analysis_for(
        &self,
        uri: &Url,
        text: String,
        dialect: String,
    ) -> tcl_compiler::analyser::AnalysisResult {
        if let Some(cached) = self.cached_analysis(uri).await {
            return cached;
        }
        tokio::task::spawn_blocking(move || {
            let mut analyser = Analyser::new();
            analyser.analyse(&text, &dialect).clone()
        })
        .await
        .unwrap_or_default()
    }

    /// Return a snapshot of the current workspace folder
    /// URLs.  Used by cross-document features (workspace
    /// symbols, cross-doc references / rename / call-
    /// hierarchy).
    pub async fn workspace_folder_urls(&self) -> Vec<Url> {
        self.workspace_folders.lock().await.clone()
    }

    /// `S-workspace-init`: copy the workspace folders the
    /// editor sent into `self.workspace_folders` so cross-
    /// document features can resolve relative paths and (in
    /// the future) scan folder contents for a workspace index.
    /// Both the newer `workspace_folders` field and the
    /// single-root `root_uri` fallback are supported.
    async fn apply_workspace_folders(&self, params: &InitializeParams) {
        if let Some(folders) = &params.workspace_folders {
            let urls: Vec<Url> = folders.iter().map(|f| f.uri.clone()).collect();
            *self.workspace_folders.lock().await = urls;
        } else if let Some(root) = &params.root_uri {
            *self.workspace_folders.lock().await = vec![root.clone()];
        }
    }

    /// `SYNC-MAY19-dialect-contextvar`: read per-folder dialect
    /// overrides from `params.initialization_options`.
    ///
    /// Expected shape:
    /// `{ "folderDialects": { "file:///path": "f5-irules",
    ///                        "file:///other": "tcl9.0" } }`.
    ///
    /// Unknown dialect names and unparseable folder URLs are
    /// dropped silently rather than failing the entire
    /// initialise — the server keeps starting up with whatever
    /// valid entries it could pull from the editor.
    async fn apply_initialization_options(&self, params: &InitializeParams) {
        let Some(opts) = &params.initialization_options else {
            return;
        };
        let Some(entries) = opts
            .as_object()
            .and_then(|m| m.get("folderDialects"))
            .and_then(serde_json::Value::as_object)
        else {
            return;
        };
        let mut parsed: Vec<(Url, String)> = Vec::new();
        for (folder_url, dialect_val) in entries {
            let Ok(url) = Url::parse(folder_url) else {
                continue;
            };
            let Some(dialect) = dialect_val.as_str() else {
                continue;
            };
            if DialectSet::parse(dialect).is_none() {
                continue;
            }
            parsed.push((url, dialect.to_owned()));
        }
        *self.folder_dialects.lock().await = parsed;
    }

    /// Resolve the dialect string a freshly opened document should
    /// be tagged with.
    ///
    /// Resolution order (`SYNC-MAY19-dialect-contextvar`):
    ///
    /// 1. The LSP ``languageId`` field — when it names a known
    ///    dialect (``"tcl-irule"`` / ``"f5-irules"`` / ``"tcl9.0"``
    ///    / etc.), use it directly.
    /// 2. The per-folder override map (`folder_dialects`) — when
    ///    the document URI sits under one of the configured folder
    ///    URLs, use the deepest-matching folder's dialect.
    /// 3. The session-wide ``default_dialect`` fallback.
    async fn dialect_for_open(&self, uri: &Url, language_id: &str) -> String {
        if let Some(d) = Self::dialect_from_language_id(language_id) {
            return d.to_owned();
        }
        if let Some(d) = self.resolve_folder_dialect(uri).await {
            return d;
        }
        self.default_dialect.lock().await.clone()
    }

    /// Look up the per-folder dialect override for `uri`,
    /// preferring the deepest (longest-prefix) match so nested
    /// folders shadow their parents.
    async fn resolve_folder_dialect(&self, uri: &Url) -> Option<String> {
        let folders = self.folder_dialects.lock().await;
        folder_dialect_for(uri, &folders)
    }

    /// Map an LSP ``languageId`` string to a dialect name accepted
    /// by ``DialectSet::parse`` / the providers' ``dialect`` arg.
    ///
    /// Recognises the editor-extension language ids
    /// (``tcl-irule`` → ``f5-irules``, etc.) plus the canonical
    /// dialect names used by the MCP bridge / direct callers
    /// (``f5-irules``, ``tcl9.0``, …).  Returns ``None`` when the
    /// language id does not name a known dialect — the caller falls
    /// back to the session default.
    fn dialect_from_language_id(language_id: &str) -> Option<&'static str> {
        // Group editor language ids alongside the canonical dialect
        // name they map to so callers in either world land on the
        // string the registry / provider trait expects.
        let mapped = match language_id {
            "tcl" | "tcl8.6" => "tcl8.6",
            "tcl8.4" => "tcl8.4",
            "tcl8.5" => "tcl8.5",
            "tcl9.0" => "tcl9.0",
            "tcl-irule" | "f5-irules" => "f5-irules",
            "tcl-iapp" | "f5-iapps" => "f5-iapps",
            "tcl-expect" | "expect" => "expect",
            "tcl-synopsys" | "synopsys-eda-tcl" => "synopsys-eda-tcl",
            "tcl-cadence" | "cadence-eda-tcl" => "cadence-eda-tcl",
            "tcl-xilinx" | "xilinx-eda-tcl" => "xilinx-eda-tcl",
            "tcl-quartus" | "intel-quartus-eda-tcl" => "intel-quartus-eda-tcl",
            "tcl-mentor" | "mentor-eda-tcl" => "mentor-eda-tcl",
            "tk" => "tk",
            _ => return None,
        };
        // Sanity-check the mapped string is something the registry
        // accepts; guards against typos in the table.
        debug_assert!(DialectSet::parse(mapped).is_some());
        Some(mapped)
    }

    async fn read_document(&self, url: &Url) -> Option<DocumentState> {
        if let Some(doc) = self.documents.lock().await.get(url).cloned() {
            return Some(doc);
        }
        // On-disk fallback: files the folder scan indexed but the
        // editor hasn't opened aren't in the open-document map.
        // Read them from disk so cross-document span→range
        // resolution (references / rename / call-hierarchy) can
        // reach their sources.  Non-`file://` URLs and unreadable
        // paths fall through to `None`.
        let path = url.to_file_path().ok()?;
        let text = tokio::task::spawn_blocking(move || std::fs::read_to_string(path))
            .await
            .ok()?
            .ok()?;
        let dialect = match self.resolve_folder_dialect(url).await {
            Some(d) => d,
            None => self.default_dialect.lock().await.clone(),
        };
        Some(DocumentState::new(text, dialect))
    }

    /// Shared helper for the goto-definition family — runs the
    /// pure-CPU `tcl_lsp_core::definition::definition` provider
    /// off the LSP event loop and returns the matched ranges.
    async fn compute_definition(&self, uri: &Url, pos: Position) -> jsonrpc::Result<Vec<Location>> {
        let Some(doc) = self.read_document(uri).await else {
            return Ok(Vec::new());
        };
        let analysis = self
            .analysis_for(uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let in_doc = tokio::task::spawn_blocking(move || {
            core_definition::definition(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("definition worker panicked: {err}").into(),
            data: None,
        })?;
        if !in_doc.is_empty() {
            // Resolved within the current document.
            return Ok(in_doc
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect());
        }
        // Cross-document fallback: resolve a proc / class
        // defined in a sibling document via the workspace index.
        self.cross_document_definition(uri, &doc.text, pos).await
    }

    /// Resolve the symbol at `pos` against the workspace index
    /// when the current document has no local definition.  Only
    /// fires on bare command words (not `$var` references), and
    /// returns `Location`s pointing into the *defining*
    /// documents.
    async fn cross_document_definition(
        &self,
        uri: &Url,
        source: &str,
        pos: Position,
    ) -> jsonrpc::Result<Vec<Location>> {
        // A `$var` reference can't resolve to a cross-document
        // proc / class.
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return Ok(Vec::new());
        }
        let Some((word, _, _)) =
            core_hover::find_word_span_at_position(source, pos.line, pos.character)
        else {
            return Ok(Vec::new());
        };
        // Collect (uri, name_span) targets from the index.
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.lock().await;
            index
                .proc_definitions(&word, uri.as_str())
                .into_iter()
                .map(|p| (p.uri.clone(), p.name_span))
                .chain(
                    index
                        .class_definitions(&word, uri.as_str())
                        .into_iter()
                        .map(|c| (c.uri.clone(), c.name_span)),
                )
                .collect()
        };
        Ok(self.resolve_target_locations(targets).await)
    }

    /// Resolve a list of `(target-uri, byte-span)` pairs to LSP
    /// `Location`s, converting each span against the *target*
    /// document's source.  Targets whose document isn't in the
    /// store, or whose URI doesn't parse, are dropped.
    async fn resolve_target_locations(
        &self,
        targets: Vec<(String, tcl_lexer::Span)>,
    ) -> Vec<Location> {
        let mut locations = Vec::new();
        for (target_uri, span) in targets {
            let Ok(parsed) = Url::parse(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = tcl_lexer::LineIndex::new(&target_doc.text);
            let start = line_index.position_at(span.start());
            let end = line_index.position_at(span.end());
            locations.push(Location {
                uri: parsed,
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
            });
        }
        locations
    }

    /// Resolve the proc / class symbol at `pos` (a bare command
    /// word, not a `$var`) to its `(simple_name,
    /// qualified_name)`, consulting the current document's
    /// analysis first and the workspace index second.  Used by
    /// cross-document references / rename to identify which
    /// symbol's call sites to gather.
    async fn resolve_workspace_symbol(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Option<(String, String)> {
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return None;
        }
        let (word, _, _) = core_hover::find_word_span_at_position(source, pos.line, pos.character)?;
        // Current document: proc, then class.
        for (qname, proc_def) in &analysis.all_procs {
            if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
                return Some((proc_def.name.clone(), proc_def.qualified_name.clone()));
            }
        }
        for class_def in analysis.all_classes.values() {
            if class_def.name == word
                || class_def.qualified_name == word
                || class_def.qualified_name == format!("::{word}")
            {
                return Some((class_def.name.clone(), class_def.qualified_name.clone()));
            }
        }
        // Otherwise the symbol may be defined in a sibling
        // document — resolve its qualified name from the index.
        let index = self.workspace_index.lock().await;
        if let Some(p) = index.proc_definitions(&word, uri.as_str()).first() {
            return Some((p.name.clone(), p.qualified_name.clone()));
        }
        if let Some(c) = index.class_definitions(&word, uri.as_str()).first() {
            return Some((c.name.clone(), c.qualified_name.clone()));
        }
        None
    }

    /// Cross-document references for the proc / class at `pos`:
    /// every invocation site in *other* documents, plus the
    /// definition sites in other documents when
    /// `include_declaration`.  Returns `Location`s resolved
    /// against their defining documents.
    async fn cross_document_references(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
    ) -> Vec<Location> {
        let Some((simple, qualified)) = self
            .resolve_workspace_symbol(uri, source, analysis, pos)
            .await
        else {
            return Vec::new();
        };
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.lock().await;
            let mut t: Vec<(String, tcl_lexer::Span)> = index
                .invocations_of(&simple, &qualified, uri.as_str())
                .into_iter()
                .map(|i| (i.uri.clone(), i.range))
                .collect();
            if include_declaration {
                for p in index.proc_definitions(&simple, uri.as_str()) {
                    t.push((p.uri.clone(), p.name_span));
                }
                for c in index.class_definitions(&simple, uri.as_str()) {
                    t.push((c.uri.clone(), c.name_span));
                }
            }
            t
        };
        self.resolve_target_locations(targets).await
    }

    /// Add cross-document rename edits for the proc / class at
    /// `pos` into `changes`.  Resolves the symbol, asks the core
    /// rename provider for the namespace-aware sibling-document
    /// edit intents (call sites + definition sites), converts
    /// each byte span to a range against its target document,
    /// and merges into the per-URI edit map (deduped).
    async fn add_cross_document_rename_edits(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
        changes: &mut std::collections::HashMap<Url, Vec<TextEdit>>,
    ) {
        let Some((simple, qualified)) = self
            .resolve_workspace_symbol(uri, source, analysis, pos)
            .await
        else {
            return;
        };
        let intents = {
            let index = self.workspace_index.lock().await;
            core_rename::cross_document_symbol_edits(
                &simple,
                &qualified,
                new_name,
                &index,
                uri.as_str(),
            )
        };
        for intent in intents {
            let Ok(parsed) = Url::parse(&intent.uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = tcl_lexer::LineIndex::new(&target_doc.text);
            let start = line_index.position_at(intent.span.start());
            let end = line_index.position_at(intent.span.end());
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
                new_text: intent.new_text,
            };
            let bucket = changes.entry(parsed).or_default();
            if !bucket.iter().any(|e| e.range == edit.range) {
                bucket.push(edit);
            }
        }
    }

    /// Cross-document incoming calls to the proc whose
    /// qualified name is `qualified`: run the externally-
    /// targeted incoming-calls pass over every analysed
    /// document *other than* `current_uri`, tagging each result
    /// with its document's URI.  Documents that don't define
    /// the proc still contribute their call sites (passing
    /// `None` for the declaration span so nothing is skipped).
    async fn cross_document_incoming_calls(
        &self,
        current_uri: &Url,
        qualified: &str,
    ) -> Vec<(Url, core_call_hierarchy::IncomingCall)> {
        let simple = qualified.rsplit("::").next().unwrap_or(qualified);
        // Snapshot (uri, source) for every other open document.
        let docs: Vec<(Url, String)> = {
            let store = self.documents.lock().await;
            store
                .iter()
                .filter(|(u, _)| *u != current_uri)
                .map(|(u, d)| (u.clone(), d.text.clone()))
                .collect()
        };
        let mut out = Vec::new();
        for (doc_uri, source) in docs {
            let analysis = self
                .cached_analysis(&doc_uri)
                .await
                .unwrap_or_else(|| Analyser::new().analyse(&source, "tcl8.6").clone());
            let calls = core_call_hierarchy::incoming_calls_for_target(
                &source, &analysis, simple, qualified, None,
            );
            for c in calls {
                out.push((doc_uri.clone(), c));
            }
        }
        out
    }

    /// Cross-document outgoing calls from the queried proc /
    /// method: the call sites inside its body whose callee is
    /// defined in a *sibling* document.  The local
    /// [`core_call_hierarchy::outgoing_calls`] pass only resolves
    /// callees in the current document; this resolves the
    /// remaining heads against the workspace index, builds a `to`
    /// item pointing at the sibling definition (URI + name span →
    /// range against that document's source), and keeps the call
    /// ranges from the *current* document.
    async fn cross_document_outgoing_calls(
        &self,
        current_uri: &Url,
        source: &str,
        item: &core_call_hierarchy::CallHierarchyItem,
        analysis: &AnalysisResult,
    ) -> Vec<CallHierarchyOutgoingCall> {
        let unresolved = {
            let source = source.to_owned();
            let item = item.clone();
            let analysis = analysis.clone();
            tokio::task::spawn_blocking(move || {
                core_call_hierarchy::unresolved_outgoing_calls(&source, &item, &analysis)
            })
            .await
            .unwrap_or_default()
        };
        let mut out = Vec::new();
        for call in unresolved {
            // Resolve the callee head against the index, preferring
            // the analyser's resolved qualified name when present.
            let Some((target_uri, qualified, name_span, detail)) = ({
                let index = self.workspace_index.lock().await;
                let mut found = None;
                for key in [
                    call.resolved_qualified_name.as_deref(),
                    Some(call.name.as_str()),
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(p) = index.proc_definitions(key, current_uri.as_str()).first() {
                        found = Some((
                            p.uri.clone(),
                            p.qualified_name.clone(),
                            p.name_span,
                            Some(format!("({} params)", p.param_count)),
                        ));
                        break;
                    }
                    if let Some(c) = index.class_definitions(key, current_uri.as_str()).first() {
                        found = Some((c.uri.clone(), c.qualified_name.clone(), c.name_span, None));
                        break;
                    }
                }
                found
            }) else {
                continue;
            };
            // Convert the sibling definition's name span to a range
            // against that document's source.
            let Ok(parsed) = Url::parse(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = tcl_lexer::LineIndex::new(&target_doc.text);
            let start = line_index.position_at(name_span.start());
            let end = line_index.position_at(name_span.end());
            let name_range = Range {
                start: Position {
                    line: start.line,
                    character: start.character,
                },
                end: Position {
                    line: end.line,
                    character: end.character,
                },
            };
            out.push(CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: qualified,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail,
                    uri: parsed,
                    range: name_range,
                    selection_range: name_range,
                    data: None,
                },
                from_ranges: call.from_ranges.into_iter().map(lift_lsp_range).collect(),
            });
        }
        out
    }

    /// Handle the `tcl-lsp.minifyDocument` workspace command.
    ///
    /// Arguments (positional, matching the editor extensions):
    /// `[uri: string, compact: bool, aggressive: bool, isolated: bool]`.
    /// Returns a JSON object with `source`, `originalLength`,
    /// `minifiedLength`, and — for the compact / aggressive tiers —
    /// `symbolMap` (and `optimisationsApplied` for aggressive),
    /// mirroring `lsp/commands.py::on_minify_document`.
    async fn minify_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let compact = args
            .get(1)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let aggressive = args
            .get(2)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let isolated = args
            .get(3)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let registry = self.registry_for_dialect(&doc.dialect).await;

        // Pure-CPU work; run off the LSP event loop.
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let value = tokio::task::spawn_blocking(move || {
            if aggressive {
                let res = core_minify::minify_tcl_aggressive(&text, &dialect, isolated, &registry);
                serde_json::json!({
                    "source": res.source,
                    "originalLength": res.original_length,
                    "minifiedLength": res.minified_length(),
                    "symbolMap": res.symbol_map.format(),
                    "optimisationsApplied": res.optimisations_applied,
                })
            } else if compact {
                let (minified, symbol_map) =
                    core_minify::minify_tcl_compact(&text, &dialect, isolated, &registry);
                serde_json::json!({
                    "source": minified,
                    "originalLength": text.len(),
                    "minifiedLength": minified.len(),
                    "symbolMap": symbol_map.format(),
                })
            } else {
                let minified = core_minify::minify_tcl(&text, &dialect, &registry);
                serde_json::json!({
                    "source": minified,
                    "originalLength": text.len(),
                    "minifiedLength": minified.len(),
                })
            }
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("minify worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(value))
    }

    /// Return an `Arc<CommandRegistry>` with `dialect` loaded on
    /// top of the default Tcl + stdlib + tcllib specs.
    ///
    /// The result is cached per canonical dialect key so each
    /// session builds at most one registry per requested dialect.
    /// Unparseable dialect strings collapse to the empty-string
    /// key so they share a single cached "plain Tcl" registry
    /// rather than leaking a fresh allocation per typo.
    async fn registry_for_dialect(&self, dialect: &str) -> Arc<CommandRegistry> {
        let parsed = DialectSet::parse(dialect);
        // Canonicalise the cache key: parseable dialects keep
        // their string; unparseable / plain-Tcl collapse to "".
        let key = if parsed.is_some() { dialect } else { "" };
        self.cached_registry(key, parsed).await
    }

    /// Cache helper for [`Self::registry_for_dialect`].
    ///
    /// Builds (or fetches) a registry holding the base specs plus,
    /// when `dialect` is `Some`, the dialect-loaded specs. The
    /// cache owns the registries via `Arc`, so they are dropped
    /// when the `Backend` is dropped instead of living for the
    /// process lifetime.
    async fn cached_registry(
        &self,
        key: &str,
        dialect: Option<DialectSet>,
    ) -> Arc<CommandRegistry> {
        let mut cache = self.dialect_registries.lock().await;
        if let Some(r) = cache.get(key) {
            return Arc::clone(r);
        }
        let mut r = CommandRegistry::build_default();
        if let Some(d) = dialect {
            r.load_dialect(d);
        }
        let arc = Arc::new(r);
        cache.insert(key.to_owned(), Arc::clone(&arc));
        arc
    }

    /// Run the analyser on `text` and push the resulting
    /// diagnostics to the LSP client for `uri`.  Mirrors
    /// the Python server's `publish_diagnostics` flow.  Runs
    /// the analyser on a `spawn_blocking` worker so the LSP
    /// event loop stays responsive.  `S-async-diagnostics`
    /// minimal port — the cached-analysis surface and
    /// debounced streaming contract lands in a follow-up.
    async fn publish_analyser_diagnostics(&self, uri: Url, text: String, dialect: String) {
        let result = tokio::task::spawn_blocking(move || {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse(&text, &dialect).clone();
            let diagnostics = lift_analyser_diagnostics(&text, &analysis.diagnostics);
            (analysis, diagnostics)
        })
        .await;
        match result {
            Ok((analysis, diags)) => {
                // Cache the analysis so the per-method
                // handlers don't have to re-run it on every
                // request.
                {
                    // Refresh the cross-document workspace index
                    // for this URI (remove stale entries, then
                    // re-add the fresh definitions).
                    let mut index = self.workspace_index.lock().await;
                    index.remove_document(uri.as_str());
                    index.add_document(uri.as_str(), &analysis);
                }
                self.analyses.lock().await.insert(uri.clone(), analysis);
                self.client.publish_diagnostics(uri, diags, None).await;
            }
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("diagnostics worker panicked: {err}"),
                    )
                    .await;
            }
        }
    }

    /// On-disk workspace-folder scan: analyse `.tcl` / `.tm`
    /// files in the workspace folders that the editor hasn't
    /// opened, merging their definitions / invocation sites into
    /// the cross-document workspace index.
    ///
    /// Editor-opened documents already feed the index through
    /// `publish_analyser_diagnostics`; this fills the gap for
    /// project files the user hasn't opened yet so cross-document
    /// features (references / rename / call-hierarchy /
    /// completion) see the whole project, not just open tabs.
    ///
    /// The directory walk, file reads and per-file analysis run on
    /// a `spawn_blocking` worker so the LSP event loop stays
    /// responsive.  URIs already present in `self.documents` are
    /// skipped so the on-disk copy never clobbers a live (and
    /// possibly unsaved) editor buffer.  The walk is capped at
    /// [`WORKSPACE_SCAN_FILE_CAP`] files so a large tree can't
    /// stall start-up.
    async fn scan_workspace_folders(&self) {
        let folders = self.workspace_folder_urls().await;
        let roots: Vec<PathBuf> = folders
            .iter()
            .filter_map(|f| f.to_file_path().ok())
            .collect();
        if roots.is_empty() {
            return;
        }
        // Snapshot the dialect-resolution inputs and the set of
        // open documents so the blocking worker can run without
        // touching any async mutex.
        let open: HashSet<Url> = self.documents.lock().await.keys().cloned().collect();
        let folder_dialects = self.folder_dialects.lock().await.clone();
        let default_dialect = self.default_dialect.lock().await.clone();

        let analysed = tokio::task::spawn_blocking(move || {
            let mut files: Vec<PathBuf> = Vec::new();
            for root in &roots {
                collect_tcl_files(root, WORKSPACE_SCAN_FILE_CAP, &mut files);
            }
            let mut out: Vec<(String, AnalysisResult)> = Vec::new();
            for path in files {
                let Ok(uri) = Url::from_file_path(&path) else {
                    continue;
                };
                if open.contains(&uri) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let dialect = folder_dialect_for(&uri, &folder_dialects)
                    .unwrap_or_else(|| default_dialect.clone());
                let mut analyser = Analyser::new();
                let analysis = analyser.analyse(&text, &dialect).clone();
                out.push((uri.to_string(), analysis));
            }
            out
        })
        .await
        .unwrap_or_default();

        let mut index = self.workspace_index.lock().await;
        for (uri, analysis) in &analysed {
            index.remove_document(uri);
            index.add_document(uri, analysis);
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.apply_workspace_folders(&params).await;
        self.apply_initialization_options(&params).await;
        Ok(InitializeResult {
            capabilities: build_server_capabilities(),
            server_info: Some(ServerInfo {
                name: "tcl-lsp-server".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tcl-lsp-server initialised")
            .await;
        // Seed the cross-document index with on-disk project files
        // the editor hasn't opened yet.
        self.scan_workspace_folders().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let dialect = self
            .dialect_for_open(&params.text_document.uri, &params.text_document.language_id)
            .await;
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        let dialect_for_diags = dialect.clone();
        let mut docs = self.documents.lock().await;
        docs.insert(
            params.text_document.uri,
            DocumentState::new(params.text_document.text, dialect),
        );
        drop(docs);
        self.publish_analyser_diagnostics(uri, text, dialect_for_diags)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // FULL sync — the last content-change carries the entire
        // document. INCREMENTAL sync is a follow-up chunk.
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        let uri = params.text_document.uri.clone();
        let mut docs = self.documents.lock().await;
        let (text, dialect) = if let Some(doc) = docs.get_mut(&uri) {
            // Preserve the document's dialect across edits; only the
            // text content changes here.
            doc.text.clone_from(&change.text);
            (change.text, doc.dialect.clone())
        } else {
            // didChange before didOpen — fall back to the session
            // default dialect; the languageId is not available here.
            let dialect = self.default_dialect.lock().await.clone();
            docs.insert(
                uri.clone(),
                DocumentState::new(change.text.clone(), dialect.clone()),
            );
            (change.text, dialect)
        };
        drop(docs);
        // `S-hover-sync11`: drop every cached hover response
        // for this URI so subsequent requests return answers
        // against the freshly-edited source rather than stale
        // pre-edit results.
        self.hover_cache.lock().await.invalidate_uri(&uri);
        // `S-semantic-tokens-rich` delta: drop the cached
        // token snapshot so the next `semanticTokens/full/delta`
        // returns a fresh full result instead of an empty edit
        // list against an outdated baseline.
        self.semantic_tokens_cache.lock().await.remove(&uri);
        // Evict the stale `AnalysisResult` so any request that
        // arrives before `publish_analyser_diagnostics` finishes
        // re-running the analyser falls through to a fresh
        // run via `analysis_for` rather than serving pre-edit
        // results (PR #454 Codex review P1).  `publish_*` will
        // reinsert the fresh entry when it completes.
        self.analyses.lock().await.remove(&uri);
        self.publish_analyser_diagnostics(uri, text, dialect).await;
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        // Accept ``{"tclLsp": {"dialect": "<name>"}}`` in either the
        // VS Code-style nested shape or the flat ``{"dialect":
        // "<name>"}`` shape (used by the MCP bridge). Update the
        // session default so newly opened documents pick up the
        // change; existing documents keep the dialect they were
        // opened with.
        let dialect = params
            .settings
            .get("tclLsp")
            .and_then(|v| v.get("dialect"))
            .or_else(|| params.settings.get("dialect"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if let Some(d) = dialect {
            *self.default_dialect.lock().await = d;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = &params.text_document.uri;
        self.documents.lock().await.remove(uri);
        self.analyses.lock().await.remove(uri);
        self.workspace_index
            .lock()
            .await
            .remove_document(uri.as_str());
        self.hover_cache.lock().await.invalidate_uri(uri);
        self.semantic_tokens_cache.lock().await.remove(uri);
        // Clear any previously-published diagnostics so the
        // editor's problem panel doesn't keep showing them
        // for a closed file.
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let ranges = tcl_lsp_core::folding::folding_ranges(&doc.text, &doc.dialect, &registry);
        Ok(Some(ranges.into_iter().map(lift_folding_range).collect()))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let symbols = core_symbols::document_symbols(&doc.text, &doc.dialect);
        let lifted: Vec<DocumentSymbol> = symbols.into_iter().map(lift_document_symbol).collect();
        Ok(Some(DocumentSymbolResponse::Nested(lifted)))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position.position;
        // Fetch the cached per-dialect registry on the async
        // path before spawning so the worker can read built-in
        // command names alongside the user-defined procs the
        // analyser surfaces.  Threads through as
        // `S-completion-rich`: minimal completion only knows
        // user procs / vars; rich completion adds the registry's
        // command set.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Snapshot the cross-document index so the worker can
        // enumerate procs from sibling files.  Cloning is the
        // price of moving it into `spawn_blocking`; the index
        // holds only definition metadata, not document text.
        let workspace = self.workspace_index.lock().await.clone();
        // Pure-CPU work; spawn_blocking off the LSP event loop.
        let items = tokio::task::spawn_blocking(move || {
            core_completion::completions(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
                Some(&workspace),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("completion worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<CompletionItem> = items.into_iter().map(lift_completion_item).collect();
        Ok(Some(CompletionResponse::Array(lifted)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let locations = self.compute_definition(&uri, pos).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<GotoDeclarationResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let locations = self.compute_definition(&uri, pos).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDeclarationResponse::Array(locations)))
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let locations = self.compute_definition(&uri, pos).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoTypeDefinitionResponse::Array(locations)))
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> jsonrpc::Result<Option<GotoImplementationResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let locations = self.compute_definition(&uri, pos).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoImplementationResponse::Array(locations)))
    }

    async fn references(&self, params: ReferenceParams) -> jsonrpc::Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let analysis_for_worker = analysis.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_references::references(
                &text,
                pos.line,
                pos.character,
                &analysis_for_worker,
                include_decl,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("references worker panicked: {err}").into(),
            data: None,
        })?;
        // Current-document hits.
        let mut locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        // Cross-document call sites (and, when requested,
        // sibling-document definition sites).
        let cross = self
            .cross_document_references(&uri, &doc.text, &analysis, pos, include_decl)
            .await;
        locations.extend(cross);
        dedup_locations(&mut locations);
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(locations))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let entries = tokio::task::spawn_blocking(move || {
            // `S-document-highlight-rich`: the kinded entry
            // point tags variable defining spans as `Write` and
            // their reads as `Read`; command-invocation heads
            // stay `Text`.
            core_references::document_highlights(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_highlight worker panicked: {err}").into(),
            data: None,
        })?;
        if entries.is_empty() {
            return Ok(None);
        }
        let highlights = entries
            .into_iter()
            .map(|(r, kind)| DocumentHighlight {
                range: lift_lsp_range(r),
                kind: Some(match kind {
                    core_references::HighlightKind::Read => DocumentHighlightKind::READ,
                    core_references::HighlightKind::Write => DocumentHighlightKind::WRITE,
                    core_references::HighlightKind::Text => DocumentHighlightKind::TEXT,
                }),
            })
            .collect();
        Ok(Some(highlights))
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyItem>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let items = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::prepare(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("call_hierarchy worker panicked: {err}").into(),
            data: None,
        })?;
        if items.is_empty() {
            return Ok(None);
        }
        let lifted = items
            .into_iter()
            .map(|i| CallHierarchyItem {
                name: i.name,
                kind: SymbolKind::FUNCTION,
                tags: None,
                detail: i.detail,
                uri: uri.clone(),
                range: lift_lsp_range(i.range),
                selection_range: lift_lsp_range(i.selection_range),
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let item = params.item;
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let core_item = core_call_hierarchy::CallHierarchyItem {
            name: item.name,
            detail: item.detail,
            range: CoreLspRange {
                start_line: item.range.start.line,
                start_character: item.range.start.character,
                end_line: item.range.end.line,
                end_character: item.range.end.character,
            },
            selection_range: CoreLspRange {
                start_line: item.selection_range.start.line,
                start_character: item.selection_range.start.character,
                end_line: item.selection_range.end.line,
                end_character: item.selection_range.end.character,
            },
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let qualified = core_item.name.clone();
        let doc_text = doc.text.clone();
        let local_item = core_item.clone();
        let local_analysis = analysis.clone();
        let local = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::incoming_calls(&doc_text, &local_item, &local_analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("incoming_calls worker panicked: {err}").into(),
            data: None,
        })?;
        // Tag local results with the request URI.
        let mut tagged: Vec<(Url, core_call_hierarchy::IncomingCall)> =
            local.into_iter().map(|c| (uri.clone(), c)).collect();
        // Cross-document callers: run the externally-targeted
        // incoming-calls pass over every *other* analysed doc.
        tagged.extend(self.cross_document_incoming_calls(&uri, &qualified).await);
        let lifted = tagged
            .into_iter()
            .map(|(doc_uri, c)| CallHierarchyIncomingCall {
                from: CallHierarchyItem {
                    name: c.from.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.from.detail,
                    uri: doc_uri,
                    range: lift_lsp_range(c.from.range),
                    selection_range: lift_lsp_range(c.from.selection_range),
                    data: None,
                },
                from_ranges: c.from_ranges.into_iter().map(lift_lsp_range).collect(),
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let item = params.item;
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let core_item = core_call_hierarchy::CallHierarchyItem {
            name: item.name,
            detail: item.detail,
            range: CoreLspRange {
                start_line: item.range.start.line,
                start_character: item.range.start.character,
                end_line: item.range.end.line,
                end_character: item.range.end.character,
            },
            selection_range: CoreLspRange {
                start_line: item.selection_range.start.line,
                start_character: item.selection_range.start.character,
                end_line: item.selection_range.end.line,
                end_character: item.selection_range.end.character,
            },
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Cross-document edges: callees defined in sibling files.
        let cross = self
            .cross_document_outgoing_calls(&uri, &doc.text, &core_item, &analysis)
            .await;
        let local_uri = uri.clone();
        let local_analysis = analysis.clone();
        let outgoing = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::outgoing_calls(&doc.text, &core_item, &local_analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("outgoing_calls worker panicked: {err}").into(),
            data: None,
        })?;
        let mut lifted: Vec<CallHierarchyOutgoingCall> = outgoing
            .into_iter()
            .map(|c| CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: c.to.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.to.detail,
                    uri: local_uri.clone(),
                    range: lift_lsp_range(c.to.range),
                    selection_range: lift_lsp_range(c.to.selection_range),
                    data: None,
                },
                from_ranges: c.from_ranges.into_iter().map(lift_lsp_range).collect(),
            })
            .collect();
        lifted.extend(cross);
        Ok(Some(lifted))
    }

    // Note: type_hierarchy LSP methods aren't exposed by
    // tower-lsp 0.20's `LanguageServer` trait. The core
    // provider in `tcl_lsp_core::type_hierarchy` is ready;
    // wiring lands once we move to a tower-lsp version
    // that surfaces these methods (tracked under
    // `S-type-hierarchy-rich`).

    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> jsonrpc::Result<Option<LinkedEditingRanges>> {
        // `S-linked-editing-range-rich`: when the cursor sits
        // on a proc declaration's name or inside the proc's
        // body, return every range that should be edited in
        // lock-step (the declaration plus any self-call sites
        // inside the body).  Editors then paint these as
        // linked-edit chips so renaming one updates the others
        // as the user types.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_linked_editing_range::linked_editing_ranges(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("linked_editing_range worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(linked) = result else {
            return Ok(None);
        };
        Ok(Some(LinkedEditingRanges {
            ranges: linked.ranges.into_iter().map(lift_lsp_range).collect(),
            word_pattern: Some(linked.word_pattern),
        }))
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult> {
        // `S-diagnostics-pipeline`: pull-based diagnostic
        // report.  Editors that support
        // `textDocument/diagnostic` request diagnostics on
        // demand rather than relying on the
        // `publish_diagnostics` push.  We run the analyser via
        // the cached-analysis surface (the same shape every
        // other request uses) and lift each analyser
        // diagnostic to the LSP wire shape.
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(empty_diagnostic_report());
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let items = tokio::task::spawn_blocking(move || {
            lift_analyser_diagnostics(&doc.text, &analysis.diagnostics)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("diagnostic worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            }),
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: real classification.  The
        // packed integer stream is 5 ints per token
        // `[deltaLine, deltaCol, length, type, modifiers]`.
        let core_data = core_semantic_tokens::full(&doc.text).data;
        let result_id = next_semantic_tokens_id();
        self.semantic_tokens_cache.lock().await.insert(
            uri,
            SemanticTokensEntry {
                result_id: result_id.clone(),
                data: core_data.clone(),
            },
        );
        Ok(Some(SemanticTokensResult::Tokens(LspSemanticTokens {
            result_id: Some(result_id),
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let previous_result_id = params.previous_result_id;
        let new_data = core_semantic_tokens::full(&doc.text).data;
        let new_result_id = next_semantic_tokens_id();

        // Compare against the cached snapshot.  When the client's
        // `previous_result_id` matches the stream we last handed
        // out for this URI, return the minimal token-aligned edit
        // that turns the cached stream into the new one (an empty
        // edit list when nothing changed).  When the ids don't
        // line up (stale / unknown previous result) fall back to a
        // fresh full token set, which the LSP spec accepts.
        let mut cache = self.semantic_tokens_cache.lock().await;
        let prev_match = cache
            .get(&uri)
            .filter(|entry| entry.result_id == previous_result_id)
            .map(|entry| entry.data.clone());
        cache.insert(
            uri,
            SemanticTokensEntry {
                result_id: new_result_id.clone(),
                data: new_data.clone(),
            },
        );
        drop(cache);
        if let Some(prev_data) = prev_match {
            let edits = match core_semantic_tokens::diff(&prev_data, &new_data) {
                None => Vec::new(),
                Some(edit) => vec![SemanticTokensEdit {
                    start: edit.start,
                    delete_count: edit.delete_count,
                    data: Some(lift_semantic_token_data(&edit.data)),
                }],
            };
            return Ok(Some(SemanticTokensFullDeltaResult::TokensDelta(
                SemanticTokensDelta {
                    result_id: Some(new_result_id),
                    edits,
                },
            )));
        }
        Ok(Some(SemanticTokensFullDeltaResult::Tokens(
            LspSemanticTokens {
                result_id: Some(new_result_id),
                data: lift_semantic_token_data(&new_data),
            },
        )))
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<SemanticTokensRangeResult>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: filter tokens to those that
        // start inside the requested range.  Editors call this
        // when only the visible viewport needs colouring.
        let core_range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let core_data = core_semantic_tokens::range(&doc.text, core_range).data;
        Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
            result_id: None,
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        // Walk every cached document and collect matching
        // symbols.  The minimal port iterates the document
        // store on the LSP loop (acquiring the mutex briefly);
        // a future workspace-index chunk will move this to a
        // pre-computed index.
        let docs = self.documents.lock().await.clone();
        if docs.is_empty() {
            return Ok(None);
        }
        let query = params.query;
        let mut all: Vec<SymbolInformation> = Vec::new();
        for (uri, doc) in docs {
            let text = doc.text.clone();
            let dialect = doc.dialect.clone();
            let q = query.clone();
            let analysis = self.analysis_for(&uri, text.clone(), dialect.clone()).await;
            let symbols = tokio::task::spawn_blocking(move || {
                core_workspace_symbols::workspace_symbols(&text, &q, &analysis)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("workspace_symbols worker panicked: {err}").into(),
                data: None,
            })?;
            for s in symbols {
                #[allow(deprecated)]
                all.push(SymbolInformation {
                    name: s.name,
                    kind: match s.kind {
                        CoreWorkspaceSymbolKind::Function => SymbolKind::FUNCTION,
                        CoreWorkspaceSymbolKind::Class => SymbolKind::CLASS,
                        CoreWorkspaceSymbolKind::Method => SymbolKind::METHOD,
                        CoreWorkspaceSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
                    },
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: lift_lsp_range(s.range),
                    },
                    container_name: s.container_name,
                });
            }
        }
        if all.is_empty() {
            return Ok(None);
        }
        Ok(Some(all))
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentLink>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // `S-document-links-rich`: pass the document's
        // enclosing directory as the workspace root so
        // relative `source <path>` arguments resolve.  When
        // the URI isn't a `file://` URL we leave the workspace
        // root unset and only absolute paths surface as links.
        let workspace_root = uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .and_then(|p| p.to_str().map(str::to_owned));
        let links = core_document_links::document_links(&doc.text, workspace_root.as_deref());
        if links.is_empty() {
            return Ok(None);
        }
        let lifted = links
            .into_iter()
            .map(|l| DocumentLink {
                range: Range {
                    start: Position {
                        line: l.start_line,
                        character: l.start_character,
                    },
                    end: Position {
                        line: l.end_line,
                        character: l.end_character,
                    },
                },
                target: Url::parse(&l.target).ok(),
                tooltip: None,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> jsonrpc::Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Build analysis on a worker so the inlay-hints
        // provider can surface parameter-name hints at user-
        // proc call sites (`S-inlay-hints-rich`) plus built-in
        // command synopsis hints.  When the analyser surfaces an
        // empty all_procs map (no user procs in the document),
        // the provider still returns built-in hints from the
        // registry.
        let hints = tokio::task::spawn_blocking(move || {
            core_inlay_hints::inlay_hints(&doc.text, range, Some(&analysis), Some(&registry))
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("inlay_hint worker panicked: {err}").into(),
            data: None,
        })?;
        if hints.is_empty() {
            return Ok(None);
        }
        let lifted = hints
            .into_iter()
            .map(|h| InlayHint {
                position: Position {
                    line: h.position_line,
                    character: h.position_character,
                },
                label: InlayHintLabel::String(h.label),
                kind: Some(InlayHintKind::PARAMETER),
                text_edits: None,
                tooltip: None,
                padding_left: None,
                padding_right: None,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn code_lens(&self, params: CodeLensParams) -> jsonrpc::Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // S-code-lens-rich: surface per-proc reference counts
        // above each definition.  The provider walks
        // `analysis.command_invocations` per proc, plus the
        // workspace index for cross-document call sites, so the
        // count reflects workspace-wide usage.
        let workspace = self.workspace_index.lock().await.clone();
        let uri_str = uri.to_string();
        let lenses = tokio::task::spawn_blocking(move || {
            core_code_lens::code_lenses(&doc.text, Some(&analysis), Some(&workspace), &uri_str)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_lens worker panicked: {err}").into(),
            data: None,
        })?;
        if lenses.is_empty() {
            return Ok(None);
        }
        let lifted = lenses
            .into_iter()
            .map(|l| CodeLens {
                range: lift_lsp_range(l.range),
                command: Some(tower_lsp::lsp_types::Command {
                    title: l.command_title,
                    command: l.command,
                    arguments: None,
                }),
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // `S-code-actions-rich`: walks the analyser's
        // diagnostics for fixes whose span overlaps the
        // requested range.  Run analysis on a worker.
        let actions = tokio::task::spawn_blocking(move || {
            core_code_actions::code_actions(&doc.text, range, Some(&analysis))
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_action worker panicked: {err}").into(),
            data: None,
        })?;
        if actions.is_empty() {
            return Ok(None);
        }
        let lifted = actions
            .into_iter()
            .map(|a| {
                // Build a WorkspaceEdit from the action's
                // edits so accepting the action actually
                // applies the fix.
                let mut changes = std::collections::HashMap::new();
                let lifted_edits: Vec<TextEdit> = a
                    .edits
                    .into_iter()
                    .map(|e| TextEdit {
                        range: lift_lsp_range(e.range),
                        new_text: e.new_text,
                    })
                    .collect();
                changes.insert(uri.clone(), lifted_edits);
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: a.title,
                    kind: Some(tower_lsp::lsp_types::CodeActionKind::QUICKFIX),
                    diagnostics: None,
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command: None,
                    is_preferred: None,
                    disabled: None,
                    data: None,
                })
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "tcl-lsp.minifyDocument" => self.minify_document_command(&params.arguments).await,
            _ => Ok(None),
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let edits = core_formatting::formatting(&doc.text, &registry);
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let edits = core_formatting::range_formatting(&doc.text, range);
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<SelectionRange>>> {
        let uri = params.text_document.uri.clone();
        let positions = params.positions;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            positions
                .into_iter()
                .map(|pos| {
                    let chain = core_selection_range::selection_range(
                        &doc.text,
                        pos.line,
                        pos.character,
                        Some(&analysis),
                    );
                    materialise_selection_range(&chain)
                })
                .collect::<Vec<Option<SelectionRange>>>()
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("selection_range worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<SelectionRange> = result.into_iter().flatten().collect();
        if lifted.is_empty() {
            return Ok(None);
        }
        Ok(Some(lifted))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_rename::prepare_rename(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("prepare_rename worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(p) = result else {
            return Ok(None);
        };
        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: lift_lsp_range(p.range),
            placeholder: p.placeholder,
        }))
    }

    async fn rename(&self, params: RenameParams) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // Per-dialect cached registry for `S-rename-rich`
        // safety gating — proc renames refuse to overwrite
        // built-in command names.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let analysis_for_worker = analysis.clone();
        let new_name_worker = new_name.clone();
        let registry_worker = Arc::clone(&registry);
        let edits = tokio::task::spawn_blocking(move || {
            core_rename::rename(
                &text,
                pos.line,
                pos.character,
                &new_name_worker,
                &analysis_for_worker,
                Some(&registry_worker),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("rename worker panicked: {err}").into(),
            data: None,
        })?;
        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        if !edits.is_empty() {
            let lifted: Vec<TextEdit> = edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect();
            changes.insert(uri.clone(), lifted);
        }
        // Cross-document rename: rewrite the symbol's call /
        // definition sites in sibling documents.  Gated on the
        // same safety checks as the in-document path
        // (`is_safe_symbol_name`, no built-in shadow) so a
        // cross-doc rename can't produce an unsafe edit set.
        if core_rename::is_safe_symbol_name(&new_name)
            && !core_rename::is_builtin_command_name(&new_name, &registry)
        {
            self.add_cross_document_rename_edits(
                &uri,
                &doc.text,
                &analysis,
                pos,
                &new_name,
                &mut changes,
            )
            .await;
        }
        if changes.is_empty() {
            return Ok(None);
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        // Per-dialect cached registry — same source the
        // completion handler uses.  Threading it through lets
        // `S-signature-help-rich` surface signatures for
        // built-in commands (e.g. `puts`, `lsearch`) without
        // requiring a user proc with the same name.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_sig::signature_help(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("signature_help worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(result.map(lift_signature_help))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        // `S-hover-sync11`: consult the LRU(256) result cache
        // first.  Cache hits return the previous hover for
        // `(uri, line, character)` directly without re-running
        // the provider.  The cache is invalidated by URI in
        // `did_change` / `did_close`, so the stored entry is
        // always pinned to the source text the editor last sent.
        let cache_key = (uri.clone(), pos.line, pos.character);
        match self.hover_cache.lock().await.get(&cache_key) {
            HoverLookup::HitEmpty => return Ok(None),
            HoverLookup::Hit(h) => return Ok(Some(lift_hover(h))),
            HoverLookup::Miss => {}
        }
        // Cache miss: run the provider.  Consults the cached
        // analysis from `did_open` / `did_change`; the
        // `analysis_for` helper falls back to a fresh
        // `analyser.analyse(...)` when the publisher hasn't
        // populated the cache yet.  Worker-offload via
        // `spawn_blocking` keeps the LSP event loop responsive.
        // SYNC11's debounce + `[timing] hover` debug logs are
        // documented but not yet wired — they need an upgraded
        // logging layer beyond the bare `Client::log_message`.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_hover::hover(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("hover worker panicked: {err}").into(),
            data: None,
        })?;
        self.hover_cache.lock().await.put(cache_key, result.clone());
        Ok(result.map(lift_hover))
    }
}

fn lift_completion_kind(k: CoreCompletionKind) -> CompletionItemKind {
    match k {
        CoreCompletionKind::Variable => CompletionItemKind::VARIABLE,
        CoreCompletionKind::Function => CompletionItemKind::FUNCTION,
        CoreCompletionKind::EnumValue => CompletionItemKind::ENUM_MEMBER,
    }
}

fn lift_completion_item(item: CoreCompletionItem) -> CompletionItem {
    CompletionItem {
        label: item.label,
        kind: Some(lift_completion_kind(item.kind)),
        insert_text: Some(item.insert_text),
        detail: item.detail,
        sort_text: item.sort_text,
        ..CompletionItem::default()
    }
}

/// Materialise the `tcl-lsp-core::selection_range` flat-vector
/// representation into a `tower_lsp` `SelectionRange` tree.
///
/// The chain in `core` is innermost first, with each link
/// pointing at its parent index; the LSP wire shape recurses
/// outward via `parent: Option<Box<SelectionRange>>`.
fn materialise_selection_range(
    chain: &[core_selection_range::SelectionRange],
) -> Option<SelectionRange> {
    if chain.is_empty() {
        return None;
    }
    // Build LSP records bottom-up so the parent box for index
    // `i` is already constructed when we wrap index `i-1`.
    let mut wrapped: Vec<Option<SelectionRange>> = chain.iter().map(|_| None).collect();
    // Process in reverse (outermost first) so each link's
    // parent — at a higher index — is already populated.
    for (i, link) in chain.iter().enumerate().rev() {
        let parent_box = link
            .parent_index
            .and_then(|p| wrapped[p].clone().map(Box::new));
        wrapped[i] = Some(SelectionRange {
            range: lift_lsp_range(link.range),
            parent: parent_box,
        });
    }
    wrapped.into_iter().next().flatten()
}

fn lift_lsp_range(r: CoreLspRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

/// Deduplicate `Location`s by `(uri, range)`, preserving first-
/// seen order.  Used to merge current-document and cross-
/// document reference hits without double-listing a location.
fn dedup_locations(locations: &mut Vec<Location>) {
    let mut seen: std::collections::HashSet<(String, u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    locations.retain(|loc| {
        seen.insert((
            loc.uri.to_string(),
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
        ))
    });
}

/// Lift the analyser's diagnostic records into the LSP wire
/// shape.  Shared by both the push-based `publish_diagnostics`
/// path (via `publish_analyser_diagnostics`) and the pull-
/// based `textDocument/diagnostic` handler.
fn lift_analyser_diagnostics(
    text: &str,
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let line_index = tcl_lexer::LineIndex::new(text);
    diagnostics
        .iter()
        .cloned()
        .map(|d| {
            let start = line_index.position_at(d.span.start());
            let end = line_index.position_at(d.span.end());
            tower_lsp::lsp_types::Diagnostic {
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
                severity: Some(match d.severity {
                    tcl_compiler::analyser::Severity::Error => {
                        tower_lsp::lsp_types::DiagnosticSeverity::ERROR
                    }
                    tcl_compiler::analyser::Severity::Warning => {
                        tower_lsp::lsp_types::DiagnosticSeverity::WARNING
                    }
                    tcl_compiler::analyser::Severity::Hint
                    | tcl_compiler::analyser::Severity::Suggestion => {
                        tower_lsp::lsp_types::DiagnosticSeverity::HINT
                    }
                }),
                code: Some(tower_lsp::lsp_types::NumberOrString::String(d.code)),
                code_description: None,
                source: Some("tcl-lsp".to_string()),
                message: d.message,
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}

/// Build an empty `DocumentDiagnosticReportResult` for callers
/// that hit a no-document or no-analysis path.
fn empty_diagnostic_report() -> DocumentDiagnosticReportResult {
    DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
        RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None,
                items: Vec::new(),
            },
        },
    ))
}

/// Resolve the per-folder dialect override for the given URI. The
/// longest matching folder prefix wins so a nested folder mapping
/// shadows its parent. Returns `None` when no folder covers the
/// URI, in which case the caller falls back to the session
/// default.
fn folder_dialect_for(uri: &Url, folders: &[(Url, String)]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    let target = uri.as_str();
    for (folder, dialect) in folders {
        if !uri_under_folder(target, folder.as_str()) {
            continue;
        }
        let len = folder.as_str().len();
        let take = match best {
            Some((existing, _)) => len > existing,
            None => true,
        };
        if take {
            best = Some((len, dialect.as_str()));
        }
    }
    best.map(|(_, d)| d.to_owned())
}

/// Upper bound on the number of files the on-disk workspace scan
/// will analyse, so a pathologically large tree can't stall
/// start-up.  Open documents are always indexed regardless of
/// this cap (they flow through `publish_analyser_diagnostics`).
const WORKSPACE_SCAN_FILE_CAP: usize = 2000;

/// `true` when `path` names a directory the workspace scan should
/// not descend into: hidden directories (dot-prefixed) and the
/// common vendor / build / scratch trees that never hold
/// first-party Tcl sources worth indexing.
fn is_skipped_scan_dir(path: &Path) -> bool {
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.starts_with('.') || matches!(name, "node_modules" | "target" | "tmp"),
        // No representable file name (e.g. `..`): skip to be safe.
        None => true,
    }
}

/// `true` when `path` has a Tcl source extension the analyser can
/// usefully index (`.tcl` scripts and `.tm` Tcl modules).
fn is_tcl_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("tcl" | "tm")
    )
}

/// Iteratively walk `root`, appending the paths of Tcl source
/// files to `out` until `out` reaches `cap` entries.  Skips the
/// directories [`is_skipped_scan_dir`] rejects and silently
/// ignores unreadable directories so a single permission error
/// doesn't abort the whole scan.  Iterative (explicit stack) to
/// avoid unbounded recursion on deep trees.
fn collect_tcl_files(root: &Path, cap: usize, out: &mut Vec<PathBuf>) {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= cap {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                if !is_skipped_scan_dir(&path) {
                    stack.push(path);
                }
            } else if file_type.is_file() && is_tcl_source(&path) {
                if out.len() >= cap {
                    return;
                }
                out.push(path);
            }
        }
    }
}

/// `true` when `uri` sits under the workspace folder `folder`.
///
/// Performs the directory-boundary check the prefix-only
/// `target.starts_with(folder)` form gets wrong — without
/// the boundary check `file:///workspace/app` would also
/// match `file:///workspace/app2/...` (PR #454 Codex review P2).
/// The match succeeds when:
///
/// * `uri == folder`, OR
/// * `uri == folder` with the folder's trailing slash trimmed, OR
/// * `uri` starts with `folder + "/"`.
fn uri_under_folder(uri: &str, folder: &str) -> bool {
    let folder_trimmed = folder.trim_end_matches('/');
    if uri == folder_trimmed || uri == folder {
        return true;
    }
    // The boundary check: the byte immediately after the
    // folder prefix in `uri` must be `/`.  Compare against the
    // trimmed form so a trailing slash on `folder` doesn't
    // change the behaviour.
    let needle = format!("{folder_trimmed}/");
    uri.starts_with(&needle)
}

/// Return a fresh `result_id` for a semantic-tokens response.
/// Monotonically increasing via an atomic counter so each
/// snapshot we hand out is uniquely identifiable, letting
/// clients consult our cache via
/// `semanticTokens/full/delta { previousResultId }`.
/// Build the `ServerCapabilities` advertised in the response
/// to `initialize`.  Kept as a free function so the
/// `LanguageServer::initialize` handler stays focused on
/// state setup and result construction — the long capability
/// literal lives here rather than inside the trait method.
fn build_server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["$".to_owned()]),
            ..CompletionOptions::default()
        }),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec![" ".to_owned()]),
            retrigger_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        definition_provider: Some(OneOf::Left(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: Some(OneOf::Left(true)),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        inlay_hint_provider: Some(OneOf::Left(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
        // Note: tower-lsp 0.20 does not expose
        // `type_hierarchy_provider` on `ServerCapabilities`;
        // the type-hierarchy core provider lives in
        // `tcl-lsp-core::type_hierarchy` and will be wired
        // once we upgrade tower-lsp (tracked under
        // `S-type-hierarchy-rich`).
        semantic_tokens_provider: Some(semantic_tokens_capability()),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        linked_editing_range_provider: Some(
            tower_lsp::lsp_types::LinkedEditingRangeServerCapabilities::Simple(true),
        ),
        // `S-diagnostics-pipeline`: advertise pull-based
        // diagnostics so editors that support it can request
        // diagnostics on demand alongside the existing
        // push-based `publish_diagnostics`.
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: Some("tcl-lsp".to_string()),
            inter_file_dependencies: false,
            workspace_diagnostics: false,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        // Editor-invoked workspace commands (currently the
        // minify-document command family).
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec!["tcl-lsp.minifyDocument".to_owned()],
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        ..ServerCapabilities::default()
    }
}

/// Build the semantic-tokens capability advertising the
/// classifier's legend, range support, and delta support.
fn semantic_tokens_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
            token_types: core_semantic_tokens::legend_token_types()
                .into_iter()
                .map(tower_lsp::lsp_types::SemanticTokenType::new)
                .collect(),
            token_modifiers: core_semantic_tokens::legend_token_modifiers()
                .into_iter()
                .map(tower_lsp::lsp_types::SemanticTokenModifier::new)
                .collect(),
        },
        range: Some(true),
        // Delta support — the handler implements the minimal
        // valid shape (returns full tokens when
        // previousResultId doesn't match, otherwise empty
        // edits).
        full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
    })
}

fn next_semantic_tokens_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("st-{id}")
}

/// Convert the core's `Vec<u32>` packed semantic-tokens
/// stream (`[deltaLine, deltaCol, length, type, modifiers]`
/// per token) into a `Vec<SemanticToken>` for the wire layer.
fn lift_semantic_token_data(data: &[u32]) -> Vec<tower_lsp::lsp_types::SemanticToken> {
    let mut tokens: Vec<tower_lsp::lsp_types::SemanticToken> = Vec::with_capacity(data.len() / 5);
    for chunk in data.chunks_exact(5) {
        tokens.push(tower_lsp::lsp_types::SemanticToken {
            delta_line: chunk[0],
            delta_start: chunk[1],
            length: chunk[2],
            token_type: chunk[3],
            token_modifiers_bitset: chunk[4],
        });
    }
    tokens
}

fn lift_parameter_information(p: CoreParameterInformation) -> ParameterInformation {
    ParameterInformation {
        label: ParameterLabel::Simple(p.label),
        documentation: None,
    }
}

fn lift_signature_information(s: CoreSignatureInformation) -> SignatureInformation {
    SignatureInformation {
        label: s.label,
        documentation: s.documentation.map(|d| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d,
            })
        }),
        parameters: Some(
            s.parameters
                .into_iter()
                .map(lift_parameter_information)
                .collect(),
        ),
        active_parameter: None,
    }
}

fn lift_signature_help(h: CoreSignatureHelp) -> SignatureHelp {
    SignatureHelp {
        signatures: h
            .signatures
            .into_iter()
            .map(lift_signature_information)
            .collect(),
        active_signature: Some(h.active_signature),
        active_parameter: Some(h.active_parameter),
    }
}

fn lift_hover(h: CoreHover) -> Hover {
    let kind = match h.kind {
        CoreHoverKind::Markdown => MarkupKind::Markdown,
    };
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind,
            value: h.value,
        }),
        range: None,
    }
}

fn lift_line_range(r: core_symbols::LineRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

fn lift_symbol_kind(k: CoreSymbolKind) -> SymbolKind {
    match k {
        CoreSymbolKind::Function => SymbolKind::FUNCTION,
        CoreSymbolKind::Method => SymbolKind::METHOD,
        CoreSymbolKind::Class => SymbolKind::CLASS,
        CoreSymbolKind::Property => SymbolKind::PROPERTY,
        CoreSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
        CoreSymbolKind::Namespace => SymbolKind::NAMESPACE,
        CoreSymbolKind::Variable => SymbolKind::VARIABLE,
    }
}

fn lift_document_symbol(s: core_symbols::DocumentSymbol) -> DocumentSymbol {
    let children = if s.children.is_empty() {
        None
    } else {
        Some(s.children.into_iter().map(lift_document_symbol).collect())
    };
    #[allow(deprecated)] // `deprecated` is required by lsp-types but unused.
    DocumentSymbol {
        name: s.name,
        detail: s.detail,
        kind: lift_symbol_kind(s.kind),
        tags: None,
        deprecated: None,
        range: lift_line_range(s.range),
        selection_range: lift_line_range(s.selection_range),
        children,
    }
}

fn lift_folding_range(r: tcl_lsp_core::folding::FoldingRange) -> FoldingRange {
    FoldingRange {
        start_line: r.start_line,
        start_character: None,
        end_line: r.end_line,
        end_character: None,
        kind: Some(match r.kind {
            FoldKind::Region => FoldingRangeKind::Region,
            FoldKind::Comment => FoldingRangeKind::Comment,
        }),
        collapsed_text: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{
        PartialResultParams, ReferenceContext, TextDocumentIdentifier, WorkDoneProgressParams,
    };

    #[test]
    fn dialect_from_language_id_recognises_editor_ids() {
        assert_eq!(
            Backend::dialect_from_language_id("tcl-irule"),
            Some("f5-irules"),
        );
        assert_eq!(
            Backend::dialect_from_language_id("tcl-iapp"),
            Some("f5-iapps"),
        );
        assert_eq!(Backend::dialect_from_language_id("tcl"), Some("tcl8.6"));
    }

    #[test]
    fn dialect_from_language_id_passes_through_canonical_names() {
        assert_eq!(
            Backend::dialect_from_language_id("f5-irules"),
            Some("f5-irules"),
        );
        assert_eq!(Backend::dialect_from_language_id("tcl9.0"), Some("tcl9.0"),);
        assert_eq!(
            Backend::dialect_from_language_id("synopsys-eda-tcl"),
            Some("synopsys-eda-tcl"),
        );
    }

    #[test]
    fn dialect_from_language_id_returns_none_for_unknown_ids() {
        assert!(Backend::dialect_from_language_id("plaintext").is_none());
        assert!(Backend::dialect_from_language_id("").is_none());
        // ``tcl-bigip`` is a BIG-IP config language id with no Tcl
        // dialect counterpart — falls back to the session default.
        assert!(Backend::dialect_from_language_id("tcl-bigip").is_none());
    }

    /// The folding-range lift maps `FoldKind::Comment` to the LSP
    /// `Comment` kind and `FoldKind::Region` to `Region`. The
    /// `start_character` / `end_character` fields stay `None`
    /// because the core provider returns line-only ranges.
    #[test]
    fn lift_folding_range_preserves_kind_and_lines() {
        let r = tcl_lsp_core::folding::FoldingRange {
            start_line: 2,
            end_line: 5,
            kind: FoldKind::Region,
        };
        let lifted = lift_folding_range(r);
        assert_eq!(lifted.start_line, 2);
        assert_eq!(lifted.end_line, 5);
        assert_eq!(lifted.kind, Some(FoldingRangeKind::Region));
        assert!(lifted.start_character.is_none());
        assert!(lifted.end_character.is_none());

        let comment = tcl_lsp_core::folding::FoldingRange {
            start_line: 0,
            end_line: 3,
            kind: FoldKind::Comment,
        };
        assert_eq!(
            lift_folding_range(comment).kind,
            Some(FoldingRangeKind::Comment),
        );
    }

    /// `lift_symbol_kind` maps every `CoreSymbolKind` variant to the
    /// matching LSP `SymbolKind` constant. The mapping is exhaustive
    /// so additions to `CoreSymbolKind` force an explicit decision
    /// about the corresponding LSP `SymbolKind`.
    #[test]
    fn lift_symbol_kind_covers_all_variants() {
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Function),
            SymbolKind::FUNCTION
        );
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Method), SymbolKind::METHOD);
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Class), SymbolKind::CLASS);
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Property),
            SymbolKind::PROPERTY
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Constructor),
            SymbolKind::CONSTRUCTOR,
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Namespace),
            SymbolKind::NAMESPACE
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Variable),
            SymbolKind::VARIABLE
        );
    }

    /// `lift_document_symbol` preserves the name / detail / kind /
    /// range / selection-range fields and recursively lifts
    /// children. An empty child list maps to `None` rather than
    /// `Some(Vec::new())` so the serialised JSON omits the field
    /// entirely (per `#[serde(skip_serializing_if)]` on the LSP
    /// type).
    #[test]
    fn lift_document_symbol_preserves_fields_and_recurses() {
        let inner = core_symbols::DocumentSymbol {
            name: "child".to_owned(),
            detail: None,
            kind: CoreSymbolKind::Variable,
            range: core_symbols::LineRange {
                start_line: 2,
                start_character: 4,
                end_line: 2,
                end_character: 9,
            },
            selection_range: core_symbols::LineRange {
                start_line: 2,
                start_character: 4,
                end_line: 2,
                end_character: 9,
            },
            children: Vec::new(),
        };
        let outer = core_symbols::DocumentSymbol {
            name: "demo".to_owned(),
            detail: Some("()".to_owned()),
            kind: CoreSymbolKind::Function,
            range: core_symbols::LineRange {
                start_line: 0,
                start_character: 0,
                end_line: 4,
                end_character: 1,
            },
            selection_range: core_symbols::LineRange {
                start_line: 0,
                start_character: 5,
                end_line: 0,
                end_character: 9,
            },
            children: vec![inner],
        };

        let lifted = lift_document_symbol(outer);
        assert_eq!(lifted.name, "demo");
        assert_eq!(lifted.detail.as_deref(), Some("()"));
        assert_eq!(lifted.kind, SymbolKind::FUNCTION);
        assert_eq!(
            lifted.range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            lifted.range.end,
            Position {
                line: 4,
                character: 1
            }
        );
        assert_eq!(
            lifted.selection_range.start,
            Position {
                line: 0,
                character: 5
            },
        );
        let kids = lifted.children.expect("non-empty children lift to Some");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].name, "child");
        assert_eq!(kids[0].kind, SymbolKind::VARIABLE);
        assert!(
            kids[0].children.is_none(),
            "leaf symbol's empty children must lift to None",
        );
    }

    /// Build a fresh `Backend` for unit tests that only
    /// exercise the dialect-resolution helpers.  `Client`
    /// isn't publicly constructible, so we build via
    /// `LspService::new` and copy the wrapped `Client` into a
    /// fresh `Backend` with reset state.
    fn test_backend() -> Backend {
        let (service, _socket) = tower_lsp::LspService::new(Backend::new);
        Backend {
            client: service.inner().client.clone(),
            documents: Mutex::new(HashMap::new()),
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            dialect_registries: Mutex::new(HashMap::new()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            analyses: Mutex::new(HashMap::new()),
            hover_cache: Mutex::new(HoverCache::default()),
            semantic_tokens_cache: Mutex::new(HashMap::new()),
            workspace_index: Mutex::new(core_workspace_index::WorkspaceIndex::new()),
        }
    }

    #[tokio::test]
    async fn scan_workspace_folders_indexes_unopened_files() {
        let root = unique_scratch_dir("scan");
        std::fs::write(
            root.join("lib.tcl"),
            "proc greet {name} { puts \"hi $name\" }\n",
        )
        .unwrap();
        std::fs::write(root.join("main.tcl"), "greet world\n").unwrap();

        let backend = test_backend();
        let root_url = Url::from_file_path(&root).unwrap();
        *backend.workspace_folders.lock().await = vec![root_url];

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.lock().await;
        let defs = index.proc_definitions("greet", "greet");
        assert!(
            !defs.is_empty(),
            "expected the unopened lib.tcl proc to be indexed",
        );
        // The call site in main.tcl should be indexed too (no
        // current-URI exclusion here).
        let invs = index.invocations_of("greet", "greet", "");
        assert!(
            !invs.is_empty(),
            "expected the unopened main.tcl call site to be indexed",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn scan_workspace_folders_skips_open_documents() {
        let root = unique_scratch_dir("scan-open");
        let on_disk = root.join("buf.tcl");
        // The on-disk copy defines `stale`; the live buffer (below)
        // defines `fresh`. After the scan the index must reflect the
        // live buffer, not the on-disk copy.
        std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

        let backend = test_backend();
        let root_url = Url::from_file_path(&root).unwrap();
        let uri = Url::from_file_path(&on_disk).unwrap();
        *backend.workspace_folders.lock().await = vec![root_url];
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );
        // Seed the index from the live buffer the way did_open would.
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("fresh", "fresh").is_empty(),
            "live buffer's proc must survive the scan",
        );
        assert!(
            index.proc_definitions("stale", "stale").is_empty(),
            "on-disk copy must not clobber the open document",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn resolve_folder_dialect_picks_deepest_prefix() {
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Url::parse("file:///workspace/").unwrap(),
                "tcl9.0".to_owned(),
            ),
            (
                Url::parse("file:///workspace/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        let inside = Url::parse("file:///workspace/irules/rule.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&inside).await,
            Some("f5-irules".to_owned()),
        );
        let outside_irules = Url::parse("file:///workspace/main.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&outside_irules).await,
            Some("tcl9.0".to_owned()),
        );
        let unrelated = Url::parse("file:///elsewhere/x.tcl").unwrap();
        assert_eq!(backend.resolve_folder_dialect(&unrelated).await, None);
    }

    #[tokio::test]
    async fn resolve_folder_dialect_respects_directory_boundary() {
        // Regression for PR #454 Codex review P2: a prefix-only
        // match would incorrectly select `file:///workspace/app`'s
        // dialect for a document inside `file:///workspace/app2/`.
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Url::parse("file:///workspace/app").unwrap(),
                "f5-irules".to_owned(),
            ),
            (
                Url::parse("file:///workspace/app2/").unwrap(),
                "tcl9.0".to_owned(),
            ),
        ];
        let sibling = Url::parse("file:///workspace/app2/main.tcl").unwrap();
        assert_eq!(
            backend.resolve_folder_dialect(&sibling).await,
            Some("tcl9.0".to_owned()),
            "sibling folder must not inherit prefix-matched dialect",
        );
        let inside_app = Url::parse("file:///workspace/app/inner.tcl").unwrap();
        assert_eq!(
            backend.resolve_folder_dialect(&inside_app).await,
            Some("f5-irules".to_owned()),
        );
    }

    #[test]
    fn uri_under_folder_handles_trailing_slash_and_boundary() {
        assert!(uri_under_folder(
            "file:///ws/app/file.tcl",
            "file:///ws/app/",
        ));
        assert!(uri_under_folder(
            "file:///ws/app/file.tcl",
            "file:///ws/app",
        ));
        // The folder itself counts as a match.
        assert!(uri_under_folder("file:///ws/app", "file:///ws/app/",));
        // A sibling folder does not.
        assert!(!uri_under_folder(
            "file:///ws/app2/file.tcl",
            "file:///ws/app",
        ));
        assert!(!uri_under_folder(
            "file:///ws/app2/file.tcl",
            "file:///ws/app/",
        ));
    }

    #[test]
    fn folder_dialect_for_prefers_deepest_match() {
        let folders = vec![
            (Url::parse("file:///ws/").unwrap(), "tcl8.6".to_owned()),
            (
                Url::parse("file:///ws/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        // A file under the nested folder picks up the nested
        // (deepest-prefix) dialect, not the parent's.
        let nested = Url::parse("file:///ws/irules/app.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&nested, &folders).as_deref(),
            Some("f5-irules"),
        );
        // A file only under the root folder gets the root dialect.
        let top = Url::parse("file:///ws/util.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&top, &folders).as_deref(),
            Some("tcl8.6"),
        );
        // A file outside every folder has no override.
        let outside = Url::parse("file:///other/x.tcl").unwrap();
        assert_eq!(folder_dialect_for(&outside, &folders), None);
    }

    #[test]
    fn is_tcl_source_matches_tcl_and_tm_only() {
        assert!(is_tcl_source(Path::new("/a/b.tcl")));
        assert!(is_tcl_source(Path::new("/a/b.tm")));
        assert!(!is_tcl_source(Path::new("/a/b.txt")));
        assert!(!is_tcl_source(Path::new("/a/b")));
    }

    #[test]
    fn is_skipped_scan_dir_skips_vendor_and_hidden() {
        assert!(is_skipped_scan_dir(Path::new("/a/.git")));
        assert!(is_skipped_scan_dir(Path::new("/a/node_modules")));
        assert!(is_skipped_scan_dir(Path::new("/a/target")));
        assert!(is_skipped_scan_dir(Path::new("/a/tmp")));
        assert!(!is_skipped_scan_dir(Path::new("/a/src")));
        assert!(!is_skipped_scan_dir(Path::new("/a/irules")));
    }

    /// Build a unique scratch directory under the system temp dir
    /// for a disk-walk test, returning its path. The caller removes
    /// it when done.
    fn unique_scratch_dir(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tcl-lsp-scan-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn collect_tcl_files_walks_recursively_and_skips_vendor() {
        let root = unique_scratch_dir("walk");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("top.tcl"), "set x 1\n").unwrap();
        std::fs::write(root.join("mod.tm"), "set y 2\n").unwrap();
        std::fs::write(root.join("readme.txt"), "not tcl\n").unwrap();
        std::fs::write(root.join("sub/nested.tcl"), "set z 3\n").unwrap();
        std::fs::write(root.join("node_modules/dep.tcl"), "set q 4\n").unwrap();
        std::fs::write(root.join(".git/hook.tcl"), "set w 5\n").unwrap();

        let mut out = Vec::new();
        collect_tcl_files(&root, 100, &mut out);
        let mut names: Vec<String> = out
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();

        std::fs::remove_dir_all(&root).ok();

        assert_eq!(names, vec!["mod.tm", "nested.tcl", "top.tcl"]);
    }

    #[test]
    fn collect_tcl_files_respects_cap() {
        let root = unique_scratch_dir("cap");
        for i in 0..10 {
            std::fs::write(root.join(format!("f{i}.tcl")), "set x 1\n").unwrap();
        }
        let mut out = Vec::new();
        collect_tcl_files(&root, 3, &mut out);
        let got = out.len();
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(got, 3);
    }

    #[test]
    fn hover_cache_round_trips_an_entry() {
        let mut cache = HoverCache::default();
        let uri = Url::parse("file:///x.tcl").unwrap();
        let key = (uri.clone(), 0, 5);
        // Miss before insertion.
        assert!(matches!(cache.get(&key), HoverLookup::Miss));
        // Insert and read back.
        let h = tcl_lsp_core::hover::Hover {
            value: "demo".to_owned(),
            kind: tcl_lsp_core::hover::HoverKind::Markdown,
        };
        cache.put(key.clone(), Some(h.clone()));
        match cache.get(&key) {
            HoverLookup::Hit(got) => assert_eq!(got.value, "demo"),
            other => panic!("expected Hit, got {other:?}"),
        }
    }

    #[test]
    fn hover_cache_records_empty_hovers() {
        let mut cache = HoverCache::default();
        let uri = Url::parse("file:///x.tcl").unwrap();
        let key = (uri, 0, 5);
        cache.put(key.clone(), None);
        // An empty hover is distinct from a miss.
        assert!(matches!(cache.get(&key), HoverLookup::HitEmpty));
    }

    #[test]
    fn hover_cache_invalidates_by_uri() {
        let mut cache = HoverCache::default();
        let uri_a = Url::parse("file:///a.tcl").unwrap();
        let uri_b = Url::parse("file:///b.tcl").unwrap();
        let h = tcl_lsp_core::hover::Hover {
            value: "x".to_owned(),
            kind: tcl_lsp_core::hover::HoverKind::Markdown,
        };
        cache.put((uri_a.clone(), 0, 0), Some(h.clone()));
        cache.put((uri_b.clone(), 0, 0), Some(h));
        cache.invalidate_uri(&uri_a);
        assert!(matches!(cache.get(&(uri_a, 0, 0)), HoverLookup::Miss));
        assert!(matches!(cache.get(&(uri_b, 0, 0)), HoverLookup::Hit(_)));
    }

    #[test]
    fn hover_cache_caps_at_256_entries() {
        let mut cache = HoverCache::default();
        let uri = Url::parse("file:///x.tcl").unwrap();
        let h = tcl_lsp_core::hover::Hover {
            value: "x".to_owned(),
            kind: tcl_lsp_core::hover::HoverKind::Markdown,
        };
        for i in 0..300 {
            cache.put((uri.clone(), 0, i), Some(h.clone()));
        }
        assert_eq!(cache.entries.len(), HoverCache::CAP);
        // Earliest insertions were evicted.
        assert!(matches!(cache.get(&(uri.clone(), 0, 0)), HoverLookup::Miss,));
        // Most-recent insertions survive.
        assert!(matches!(cache.get(&(uri, 0, 299)), HoverLookup::Hit(_),));
    }

    #[tokio::test]
    async fn dialect_for_open_falls_back_to_folder_dialect() {
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![(
            Url::parse("file:///workspace/").unwrap(),
            "f5-irules".to_owned(),
        )];
        let doc = Url::parse("file:///workspace/main.tcl").unwrap();
        // A `tcl` language id maps to `tcl8.6` directly, so the
        // folder override is *not* consulted (language_id is the
        // most specific signal we have).
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl").await,
            "tcl8.6".to_owned(),
        );
        // An unknown language id (`plaintext`) lets the folder
        // override take effect.
        assert_eq!(
            backend.dialect_for_open(&doc, "plaintext").await,
            "f5-irules".to_owned(),
        );
    }

    #[tokio::test]
    async fn cross_document_definition_resolves_sibling_proc() {
        let backend = test_backend();
        let lib_uri = Url::parse("file:///lib.tcl").unwrap();
        let main_uri = Url::parse("file:///main.tcl").unwrap();
        let lib_src = "proc shared_helper {} {}\n";
        let main_src = "shared_helper\n";
        // Register both documents.
        {
            let mut docs = backend.documents.lock().await;
            docs.insert(
                lib_uri.clone(),
                DocumentState::new(lib_src.to_owned(), "tcl8.6".to_owned()),
            );
            docs.insert(
                main_uri.clone(),
                DocumentState::new(main_src.to_owned(), "tcl8.6".to_owned()),
            );
        }
        // Index the library document's proc.
        {
            let mut a = Analyser::new();
            let lib_analysis = a.analyse(lib_src, "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(lib_uri.as_str(), &lib_analysis);
        }
        // From main.tcl, jumping on `shared_helper` resolves to
        // lib.tcl's declaration.
        let locs = backend
            .compute_definition(&main_uri, Position::new(0, 0))
            .await
            .expect("definition ok");
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].uri, lib_uri);
        // `proc shared_helper` — name token starts at column 5.
        assert_eq!(locs[0].range.start.line, 0);
        assert_eq!(locs[0].range.start.character, 5);
    }

    #[tokio::test]
    async fn minify_document_command_returns_minified_source() {
        let backend = test_backend();
        let uri = Url::parse("file:///m.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("# c\nputs   hi\n".to_owned(), "tcl8.6".to_owned()),
        );
        // Plain minify (compact=false, aggressive=false).
        let args = vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
        ];
        let result = backend
            .minify_document_command(&args)
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(result["source"], serde_json::json!("puts hi"));
        assert_eq!(result["minifiedLength"], serde_json::json!(7));
        assert!(result.get("symbolMap").is_none());
    }

    #[tokio::test]
    async fn minify_document_command_compact_includes_symbol_map() {
        let backend = test_backend();
        let uri = Url::parse("file:///m.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(
                "proc greet {name} {\n    return $name\n}\n".to_owned(),
                "tcl8.6".to_owned(),
            ),
        );
        let args = vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::Bool(true), // compact
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
        ];
        let result = backend
            .minify_document_command(&args)
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(result["source"], serde_json::json!("proc a {a} {return $a}"));
        assert!(
            result["symbolMap"].as_str().is_some_and(|s| s.contains("a <- greet")),
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn cross_document_definition_skipped_when_local_match_exists() {
        let backend = test_backend();
        let uri = Url::parse("file:///main.tcl").unwrap();
        let src = "proc greet {} {}\ngreet\n";
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
        );
        // Cursor on the `greet` call (line 1) resolves locally;
        // the result points at the same document, line 0.
        let locs = backend
            .compute_definition(&uri, Position::new(1, 0))
            .await
            .expect("definition ok");
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].uri, uri);
        assert_eq!(locs[0].range.start.line, 0);
    }

    /// Register a document in the store and the workspace index.
    async fn register(backend: &Backend, uri: &Url, src: &str) {
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
        );
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        backend
            .workspace_index
            .lock()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    #[tokio::test]
    async fn cross_document_references_finds_sibling_call_sites() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        let lib_src = "proc helper {} {}\nhelper\n";
        register(&backend, &lib, lib_src).await;
        register(&backend, &consumer, "helper\nhelper\n").await;
        // From lib.tcl's `helper` declaration, cross-doc refs
        // are the two calls in consumer.tcl.
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(lib_src, "tcl8.6").clone()
        };
        let cross = backend
            .cross_document_references(&lib, lib_src, &analysis, Position::new(0, 5), false)
            .await;
        assert_eq!(cross.len(), 2, "{cross:?}");
        assert!(cross.iter().all(|l| l.uri == consumer));
    }

    #[tokio::test]
    async fn references_handler_merges_local_and_cross_document() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        // lib.tcl: declaration + one local call.
        register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
        // consumer.tcl: one cross-doc call.
        register(&backend, &consumer, "helper\n").await;
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        let result = backend.references(params).await.expect("ok").expect("some");
        // Declaration (lib:0) + local call (lib:1) + cross-doc
        // call (consumer:0) = 3 locations across 2 files.
        assert_eq!(result.len(), 3, "{result:?}");
        assert!(result.iter().any(|l| l.uri == consumer));
        assert!(result.iter().filter(|l| l.uri == lib).count() == 2);
    }

    #[tokio::test]
    async fn rename_edits_span_multiple_documents() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
        register(&backend, &consumer, "helper\nhelper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            new_name: "do_it".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let edit = backend.rename(params).await.expect("ok").expect("some");
        let changes = edit.changes.expect("changes");
        // Both documents get edits.
        assert!(changes.contains_key(&lib), "lib edits missing: {changes:?}");
        assert!(
            changes.contains_key(&consumer),
            "consumer edits missing: {changes:?}",
        );
        // consumer.tcl: two call sites rewritten to `do_it`.
        let consumer_edits = &changes[&consumer];
        assert_eq!(consumer_edits.len(), 2, "{consumer_edits:?}");
        assert!(consumer_edits.iter().all(|e| e.new_text == "do_it"));
    }

    #[tokio::test]
    async fn rename_to_builtin_blocked_cross_document() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        register(&backend, &consumer, "helper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            // `if` is a built-in command — renaming to it must be
            // refused (no edits in any document).
            new_name: "if".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let result = backend.rename(params).await.expect("ok");
        assert!(result.is_none(), "rename to a built-in should be refused");
    }

    #[tokio::test]
    async fn incoming_calls_span_multiple_documents() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        register(&backend, &consumer, "proc caller {} { helper }\n").await;
        // prepare the `helper` item from lib.tcl.
        let prep = {
            let mut a = Analyser::new();
            let analysis = a.analyse("proc helper {} {}\n", "tcl8.6").clone();
            core_call_hierarchy::prepare("proc helper {} {}\n", 0, 5, &analysis)
        };
        let core_item = prep.into_iter().next().expect("prepared item");
        assert_eq!(core_item.name, "::helper");
        let cross = backend
            .cross_document_incoming_calls(&lib, &core_item.name)
            .await;
        // The only caller is `caller` in consumer.tcl.
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].0, consumer);
        assert_eq!(cross[0].1.from.name, "::caller");
    }

    #[tokio::test]
    async fn outgoing_calls_resolve_sibling_document_callee() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let main = Url::parse("file:///main.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        // `caller` calls a builtin (`puts`) and the sibling-file
        // proc `helper`; only `helper` resolves cross-document.
        let main_src = "proc caller {} { puts hi\n helper }\n";
        register(&backend, &main, main_src).await;

        let item = core_call_hierarchy::CallHierarchyItem {
            name: "::caller".to_owned(),
            detail: None,
            range: CoreLspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            selection_range: CoreLspRange {
                start_line: 0,
                start_character: 5,
                end_line: 0,
                end_character: 11,
            },
        };
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(main_src, "tcl8.6").clone()
        };
        let cross = backend
            .cross_document_outgoing_calls(&main, main_src, &item, &analysis)
            .await;
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].to.name, "::helper");
        assert_eq!(cross[0].to.uri, lib);
        assert_eq!(cross[0].from_ranges.len(), 1, "{cross:?}");
    }
}
