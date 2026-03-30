#pragma once

#include "tcl_lsp/core/document_buffer.hpp"

#include <cstdint>
#include <memory>
#include <mutex>
#include <optional>
#include <shared_mutex>
#include <string>
#include <unordered_map>

namespace tcl_lsp {

// Immutable snapshot of a document buffer. Cheap to copy (atomic refcount
// bump). All query methods work on the const DocumentBuffer via operator->.
using BufferSnapshot = std::shared_ptr<const DocumentBuffer>;

// Per-document state tracked by the C++ server.
struct DocumentState {
    std::string uri;
    std::string language_id;
    BufferSnapshot snapshot;

    DocumentState(std::string uri, std::string language_id,
                  std::string source, int32_t version, uint64_t epoch);

    // Apply a full-content change (TextDocumentSyncKind::Full).
    void update_full(std::string new_source, int32_t new_version,
                     uint64_t epoch);
};

// Thread-safe document store for all open documents.
//
// Readers grab a BufferSnapshot (shared_ptr copy under shared_lock) and
// work on immutable data outside any lock. Writers hold a unique_lock only
// long enough to swap in a new snapshot and bump the epoch.
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

    // Get an immutable snapshot of a document's buffer.
    // Returns nullptr if the document is not open. Zero-copy.
    [[nodiscard]] auto get_snapshot(const std::string& uri) const
        -> BufferSnapshot;

    // Get a copy of a document's source and version (backward compat).
    // Prefer get_snapshot() for zero-copy access.
    [[nodiscard]] auto get_source(const std::string& uri) const
        -> std::optional<std::pair<std::string, int32_t>>;

    // Get the language_id for a document.
    [[nodiscard]] auto get_language_id(const std::string& uri) const
        -> std::string;

    // Check whether a document is open.
    [[nodiscard]] auto is_open(const std::string& uri) const -> bool;

    // Number of open documents.
    [[nodiscard]] auto size() const -> std::size_t;

    // Monotonically increasing mutation counter. Incremented on every
    // open/change/close. Snapshots carry the epoch they were created at.
    [[nodiscard]] auto epoch() const noexcept -> uint64_t;

private:
    mutable std::shared_mutex mutex_;
    std::unordered_map<std::string, DocumentState> docs_;
    uint64_t epoch_{0};
};

} // namespace tcl_lsp
