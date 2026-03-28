#include "tcl_lsp/lsp/server.hpp"

#include <string>
#include <utility>
#include <variant>

namespace tcl_lsp {

TclLspServer::TclLspServer(lsp::Connection& connection)
    : handler_{connection}
{
    register_handlers();
}

void TclLspServer::run() {
    running_ = true;
    while (running_) {
        handler_.processIncomingMessages();
    }
}

void TclLspServer::register_handlers() {
    handler_
        .add<lsp::requests::Initialize>(
            [this](lsp::requests::Initialize::Params&& p) {
                return on_initialize(std::move(p));
            })
        .add<lsp::notifications::Initialized>(
            [this](lsp::notifications::Initialized::Params&& p) {
                on_initialized(std::move(p));
            })
        .add<lsp::requests::Shutdown>(
            [this]() { return on_shutdown(); })
        .add<lsp::notifications::Exit>(
            [this]() { on_exit(); })
        .add<lsp::notifications::TextDocument_DidOpen>(
            [this](lsp::notifications::TextDocument_DidOpen::Params&& p) {
                on_did_open(std::move(p));
            })
        .add<lsp::notifications::TextDocument_DidChange>(
            [this](lsp::notifications::TextDocument_DidChange::Params&& p) {
                on_did_change(std::move(p));
            })
        .add<lsp::notifications::TextDocument_DidClose>(
            [this](lsp::notifications::TextDocument_DidClose::Params&& p) {
                on_did_close(std::move(p));
            });
}

// Lifecycle

auto TclLspServer::on_initialize(
    [[maybe_unused]] lsp::requests::Initialize::Params&& params)
    -> lsp::requests::Initialize::Result {
    return lsp::requests::Initialize::Result{
        .capabilities = build_capabilities(),
        .serverInfo = lsp::InitializeResultServerInfo{
            .name = "tcl-lsp",
            .version = "2.0.0-native",
        },
    };
}

void TclLspServer::on_initialized(
    [[maybe_unused]] lsp::notifications::Initialized::Params&& params) {
    // Future: start background workspace scanning via Python bridge.
}

auto TclLspServer::on_shutdown() -> lsp::requests::Shutdown::Result {
    return lsp::requests::Shutdown::Result{};
}

void TclLspServer::on_exit() {
    running_ = false;
}

// Document sync

void TclLspServer::on_did_open(
    lsp::notifications::TextDocument_DidOpen::Params&& params) {
    auto uri = params.textDocument.uri.toString();
    documents_.open(
        uri,
        params.textDocument.languageId,
        params.textDocument.text,
        params.textDocument.version);
}

void TclLspServer::on_did_change(
    lsp::notifications::TextDocument_DidChange::Params&& params) {
    if (params.contentChanges.empty()) return;

    auto uri = params.textDocument.uri.toString();
    // Extract text from the last content change (full sync).
    const auto& last = params.contentChanges.back();
    auto text = std::visit(
        [](const auto& change) -> std::string { return change.text; },
        last);
    documents_.change(uri, std::move(text), params.textDocument.version);
}

void TclLspServer::on_did_close(
    lsp::notifications::TextDocument_DidClose::Params&& params) {
    auto uri = params.textDocument.uri.toString();
    documents_.close(uri);
}

// Capabilities

auto TclLspServer::build_capabilities() -> lsp::ServerCapabilities {
    // Semantic token legend — matches the Python server's token types/modifiers.
    lsp::SemanticTokensLegend legend{
        .tokenTypes = {
            "namespace", "type", "class", "enum", "interface",
            "struct", "typeParameter", "parameter", "variable",
            "property", "enumMember", "event", "function", "method",
            "macro", "keyword", "modifier", "comment", "string",
            "number", "regexp", "operator", "decorator",
        },
        .tokenModifiers = {
            "declaration", "definition", "readonly", "static",
            "deprecated", "abstract", "async", "modification",
            "documentation", "defaultLibrary",
        },
    };

    // Fields in declaration order per ServerCapabilities struct.
    return lsp::ServerCapabilities{
        .positionEncoding = lsp::PositionEncodingKind::UTF16,
        .textDocumentSync = lsp::TextDocumentSyncOptions{
            .openClose = true,
            .change = lsp::TextDocumentSyncKind::Full,
        },
        .completionProvider = lsp::CompletionOptions{},
        .hoverProvider = true,
        .signatureHelpProvider = lsp::SignatureHelpOptions{
            .triggerCharacters = std::vector<std::string>{" "},
        },
        .definitionProvider = true,
        .referencesProvider = true,
        .documentSymbolProvider = true,
        .codeActionProvider = lsp::CodeActionOptions{
            .codeActionKinds = std::vector<lsp::Enumeration<lsp::CodeActionKind, std::string>>{
                lsp::CodeActionKind::QuickFix,
                lsp::CodeActionKind::RefactorExtract,
                lsp::CodeActionKind::RefactorInline,
                lsp::CodeActionKind::RefactorRewrite,
                lsp::CodeActionKind::Source,
            },
        },
        .documentLinkProvider = lsp::DocumentLinkOptions{},
        .workspaceSymbolProvider = true,
        .documentFormattingProvider = true,
        .documentRangeFormattingProvider = true,
        .renameProvider = lsp::RenameOptions{
            .prepareProvider = true,
        },
        .foldingRangeProvider = true,
        .selectionRangeProvider = true,
        .callHierarchyProvider = true,
        .semanticTokensProvider = lsp::SemanticTokensOptions{
            .legend = std::move(legend),
            .full = lsp::SemanticTokensOptionsFull{
                .delta = true,
            },
        },
        .inlayHintProvider = true,
    };
}

} // namespace tcl_lsp
