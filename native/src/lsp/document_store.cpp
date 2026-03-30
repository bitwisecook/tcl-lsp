#include "tcl_lsp/lsp/document_store.hpp"

#include <utility>

namespace tcl_lsp {

// DocumentState

DocumentState::DocumentState(std::string uri, std::string language_id,
                             std::string source, int32_t version,
                             uint64_t epoch)
    : uri{std::move(uri)}
    , language_id{std::move(language_id)}
    , snapshot{std::make_shared<DocumentBuffer>(
          DocumentBuffer::from_source(std::move(source), version, epoch))}
{
}

void DocumentState::update_full(std::string new_source, int32_t new_version,
                                uint64_t epoch) {
    snapshot = std::make_shared<DocumentBuffer>(
        DocumentBuffer::from_source(std::move(new_source), new_version, epoch));
}

// DocumentStore

void DocumentStore::open(std::string uri, std::string language_id,
                         std::string source, int32_t version) {
    std::unique_lock lock{mutex_};
    ++epoch_;
    auto key = uri;  // copy before move
    docs_.insert_or_assign(
        std::move(key),
        DocumentState{std::move(uri), std::move(language_id),
                      std::move(source), version, epoch_});
}

void DocumentStore::change(const std::string& uri, std::string new_source,
                           int32_t new_version) {
    std::unique_lock lock{mutex_};
    auto it = docs_.find(uri);
    if (it != docs_.end()) {
        ++epoch_;
        it->second.update_full(std::move(new_source), new_version, epoch_);
    }
}

void DocumentStore::close(const std::string& uri) {
    std::unique_lock lock{mutex_};
    if (docs_.erase(uri) > 0) {
        ++epoch_;
    }
}

auto DocumentStore::get_snapshot(const std::string& uri) const
    -> BufferSnapshot {
    std::shared_lock lock{mutex_};
    auto it = docs_.find(uri);
    if (it == docs_.end()) return nullptr;
    return it->second.snapshot;
}

auto DocumentStore::get_source(const std::string& uri) const
    -> std::optional<std::pair<std::string, int32_t>> {
    std::shared_lock lock{mutex_};
    auto it = docs_.find(uri);
    if (it == docs_.end()) return std::nullopt;
    const auto& snap = it->second.snapshot;
    return std::pair{std::string(snap->source()),
                     snap->version().value_or(0)};
}

auto DocumentStore::get_language_id(const std::string& uri) const
    -> std::string {
    std::shared_lock lock{mutex_};
    auto it = docs_.find(uri);
    if (it == docs_.end()) return {};
    return it->second.language_id;
}

auto DocumentStore::is_open(const std::string& uri) const -> bool {
    std::shared_lock lock{mutex_};
    return docs_.contains(uri);
}

auto DocumentStore::size() const -> std::size_t {
    std::shared_lock lock{mutex_};
    return docs_.size();
}

auto DocumentStore::epoch() const noexcept -> uint64_t {
    std::shared_lock lock{mutex_};
    return epoch_;
}

} // namespace tcl_lsp
