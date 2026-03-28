#pragma once

#include "tcl_lsp/core/document_buffer.hpp"

#include <cstdint>
#include <mutex>
#include <optional>
#include <shared_mutex>
#include <string>
#include <unordered_map>

namespace tcl_lsp {

// Per-document state tracked by the C++ server.
struct DocumentState {
    std::string uri;
    std::string language_id;
    std::string source;
    int32_t version = 0;
    DocumentBuffer buffer;

    DocumentState(std::string uri, std::string language_id,
                  std::string source, int32_t version);

    // Apply a full-content change (TextDocumentSyncKind::Full).
    void update_full(std::string new_source, int32_t new_version);
};

// Thread-safe document store for all open documents.
class DocumentStore {
public:
    // Open a new document (didOpen).
    void open(std::string uri, std::string language_id,
              std::string source, int32_t version);

    // Update an existing document (didChange — full sync).
    void change(const std::string& uri, std::string new_source,
                int32_t new_version);

    // Close a document (didClose).
    void close(const std::string& uri);

    // Get a snapshot of a document's source and version.
    // Returns nullopt if the document is not open.
    [[nodiscard]] auto get_source(const std::string& uri) const
        -> std::optional<std::pair<std::string, int32_t>>;

    // Get the language_id for a document.
    [[nodiscard]] auto get_language_id(const std::string& uri) const
        -> std::string;

    // Check whether a document is open.
    [[nodiscard]] auto is_open(const std::string& uri) const -> bool;

    // Number of open documents.
    [[nodiscard]] auto size() const -> std::size_t;

private:
    mutable std::shared_mutex mutex_;
    std::unordered_map<std::string, DocumentState> docs_;
};

} // namespace tcl_lsp
