#pragma once

#include "tcl_lsp/lsp/document_store.hpp"

#include <lsp/connection.h>
#include <lsp/messagehandler.h>
#include <lsp/messages.h>

#include <atomic>
#include <string>

namespace tcl_lsp {

// The native Tcl LSP server.
//
// Owns the lsp-framework Connection and MessageHandler, the document store,
// and (in later phases) the Python bridge for delegating feature requests to
// existing Python code.
class TclLspServer {
public:
    explicit TclLspServer(lsp::Connection& connection);

    // Run the message loop until shutdown/exit.
    void run();

    // Access the document store (thread-safe).
    [[nodiscard]] auto documents() const -> const DocumentStore& {
        return documents_;
    }

private:
    lsp::MessageHandler handler_;
    DocumentStore documents_;
    std::atomic<bool> running_{false};

    // Lifecycle
    auto on_initialize(lsp::requests::Initialize::Params&& params)
        -> lsp::requests::Initialize::Result;
    void on_initialized(lsp::notifications::Initialized::Params&& params);
    auto on_shutdown() -> lsp::requests::Shutdown::Result;
    void on_exit();

    // Document sync
    void on_did_open(lsp::notifications::TextDocument_DidOpen::Params&& params);
    void on_did_change(lsp::notifications::TextDocument_DidChange::Params&& params);
    void on_did_close(lsp::notifications::TextDocument_DidClose::Params&& params);

    // Register all handlers with the message handler.
    void register_handlers();

    // Build the ServerCapabilities for the initialize response.
    [[nodiscard]] static auto build_capabilities() -> lsp::ServerCapabilities;
};

} // namespace tcl_lsp
