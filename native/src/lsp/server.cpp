#include "tcl_lsp/lsp/server.hpp"

#include "tcl_lsp/lsp/semantic_token_collector.hpp"
#include "tcl_lsp/lsp/semantic_token_types.hpp"

#include <lsp/json/json.h>
#include <lsp/serialization.h>

#include <string>
#include <utility>
#include <variant>

namespace tcl_lsp {

TclLspServer::TclLspServer(lsp::Connection& connection)
    : handler_{connection}
    , python_{std::make_unique<PythonBridge>()}
{
    register_handlers();

    // Wire Python diagnostics back to the client via lsp-framework.
    python_->set_notification_callback(
        [this](std::string_view method, const std::string& params_json) {
            auto json_params = lsp::json::parse(params_json);
            handler_.sendNotification(method, std::move(json_params));
        });
}

void TclLspServer::run() {
    running_ = true;
    while (running_) {
        handler_.processIncomingMessages();
    }
}

// Helper: call a Python feature and return the parsed JSON result.
auto TclLspServer::call_python_feature(const std::string& method,
                                        const std::string& params_json)
    -> lsp::json::Value {
    auto result = python_->call_feature(method, params_json);
    if (!result.has_value()) return {};
    return lsp::json::parse(*result);
}

void TclLspServer::register_handlers() {
    handler_
        // Lifecycle
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

        // Document sync
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

    // Helper lambda: extract URI from a textDocument/semanticTokens request.
    auto extract_uri = [](const lsp::json::Value& params) -> std::string {
        auto* td = params.object().find("textDocument");
        if (td == nullptr) return {};
        auto* u = td->object().find("uri");
        if (u == nullptr) return {};
        return u->string();
    };

    // Helper lambda: build the {"data": [...]} JSON response.
    auto build_tokens_response = [](const std::vector<std::int32_t>& data)
        -> lsp::json::Value {
        lsp::json::Array arr;
        arr.reserve(data.size());
        for (auto v : data) arr.push_back(lsp::json::Value{static_cast<std::int64_t>(v)});
        lsp::json::Object result;
        result["data"] = lsp::json::Value{std::move(arr)};
        return lsp::json::Value{std::move(result)};
    };

    // Native semantic tokens handler.
    handler_.add("textDocument/semanticTokens/full",
        [this, extract_uri, build_tokens_response](lsp::json::Value&& params) -> lsp::json::Value {
            auto uri = extract_uri(params);
            auto snapshot = documents_.get_source(uri);
            if (!snapshot.has_value()) return lsp::json::Value{lsp::json::Object{}};

            auto tokens = semantic_token_collector_.collect(snapshot->first);
            auto data = delta_encode(tokens);
            return build_tokens_response(data);
        });

    // Delta semantic tokens — fall back to full for now.
    handler_.add("textDocument/semanticTokens/full/delta",
        [this, extract_uri, build_tokens_response](lsp::json::Value&& params) -> lsp::json::Value {
            auto uri = extract_uri(params);
            auto snapshot = documents_.get_source(uri);
            if (!snapshot.has_value()) return lsp::json::Value{lsp::json::Object{}};

            auto tokens = semantic_token_collector_.collect(snapshot->first);
            auto data = delta_encode(tokens);
            return build_tokens_response(data);
        });

    // Remaining feature requests delegated to Python via generic JSON handlers.
    static constexpr std::string_view python_features[] = {
        "textDocument/completion",
        "textDocument/hover",
        "textDocument/definition",
        "textDocument/references",
        "textDocument/documentSymbol",
        "textDocument/foldingRange",
        "textDocument/rename",
        "textDocument/prepareRename",
        "textDocument/signatureHelp",
        "textDocument/formatting",
        "textDocument/rangeFormatting",
        "textDocument/codeAction",
        "workspace/symbol",
        "textDocument/inlayHint",
        "textDocument/prepareCallHierarchy",
        "callHierarchy/incomingCalls",
        "callHierarchy/outgoingCalls",
        "textDocument/documentLink",
        "textDocument/selectionRange",
    };

    for (auto method : python_features) {
        handler_.add(method,
            [this, m = std::string(method)](lsp::json::Value&& params)
                -> lsp::json::Value {
                auto params_json = lsp::json::stringify(params);
                return call_python_feature(m, params_json);
            });
    }

    // workspace/executeCommand — route custom commands to Python bridge.
    handler_.add("workspace/executeCommand",
        [this](lsp::json::Value&& params) -> lsp::json::Value {
            auto& obj = params.object();
            std::string command;
            if (auto* v = obj.find("command")) {
                command = v->string();
            }
            std::string args_json = "[]";
            if (auto* v = obj.find("arguments")) {
                args_json = lsp::json::stringify(*v);
            }
            auto result = python_->execute_command(command, args_json);
            if (!result.has_value()) return {};
            return lsp::json::parse(*result);
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
    python_->on_initialized("{}", "{}");
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

    // Forward to Python for analysis + diagnostics.
    python_->on_did_open(uri, params.textDocument.languageId,
                         params.textDocument.text,
                         params.textDocument.version);
}

void TclLspServer::on_did_change(
    lsp::notifications::TextDocument_DidChange::Params&& params) {
    if (params.contentChanges.empty()) return;

    auto uri = params.textDocument.uri.toString();
    const auto& last = params.contentChanges.back();
    auto text = std::visit(
        [](const auto& change) -> std::string { return change.text; },
        last);
    documents_.change(uri, text, params.textDocument.version);

    // Forward to Python for re-analysis + diagnostics.
    python_->on_did_change(uri, text, params.textDocument.version);
}

void TclLspServer::on_did_close(
    lsp::notifications::TextDocument_DidClose::Params&& params) {
    auto uri = params.textDocument.uri.toString();
    documents_.close(uri);
    python_->on_did_close(uri);
}

// Capabilities

auto TclLspServer::build_capabilities() -> lsp::ServerCapabilities {
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
        .executeCommandProvider = lsp::ExecuteCommandOptions{
            .commands = std::vector<std::string>{
                "tcl-lsp.optimiseDocument",
                "tcl-lsp.minifyDocument",
                "tcl-lsp.unminifyError",
                "tcl-lsp.fixAllSafeIssues",
                "tcl-lsp.describeIruleEvent",
                "tcl-lsp.describeIruleCommand",
                "tcl-lsp.listIruleEvents",
                "tcl-lsp.listSubcommands",
                "tcl-lsp.searchHelp",
                "tcl-lsp.suggestPackagesForSymbol",
                "tcl-lsp.listKnownPackages",
                "tcl-lsp.compilerExplorer",
                "tcl-lsp.tkPreview",
                "tcl-lsp.diagramData",
                "tcl-lsp.extractRule",
                "tcl-lsp.listRules",
                "tcl-lsp.writeRuleBack",
            },
        },
        .callHierarchyProvider = true,
        .semanticTokensProvider = lsp::SemanticTokensOptions{
            .legend = std::move(legend),
            .full = lsp::SemanticTokensOptionsFull{
                .delta = true,
            },
        },
        .inlayHintProvider = false,
    };
}

} // namespace tcl_lsp
