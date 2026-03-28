#include "tcl_lsp/lsp/document_store.hpp"

#include <utility>

namespace tcl_lsp {

// DocumentState

DocumentState::DocumentState(std::string uri, std::string language_id,
                             std::string source, int32_t version)
    : uri{std::move(uri)}
    , language_id{std::move(language_id)}
    , source{std::move(source)}
    , version{version}
    , buffer{DocumentBuffer::from_source(this->source)}
{
}

void DocumentState::update_full(std::string new_source, int32_t new_version) {
    source = std::move(new_source);
    version = new_version;
    buffer = DocumentBuffer::from_source(source);
}

// DocumentStore

void DocumentStore::open(std::string uri, std::string language_id,
                         std::string source, int32_t version) {
    std::unique_lock lock{mutex_};
    auto key = uri;  // copy before move
    docs_.insert_or_assign(
        std::move(key),
        DocumentState{std::move(uri), std::move(language_id),
                      std::move(source), version});
}

void DocumentStore::change(const std::string& uri, std::string new_source,
                           int32_t new_version) {
    std::unique_lock lock{mutex_};
    auto it = docs_.find(uri);
    if (it != docs_.end()) {
        it->second.update_full(std::move(new_source), new_version);
    }
}

void DocumentStore::close(const std::string& uri) {
    std::unique_lock lock{mutex_};
    docs_.erase(uri);
}

auto DocumentStore::get_source(const std::string& uri) const
    -> std::optional<std::pair<std::string, int32_t>> {
    std::shared_lock lock{mutex_};
    auto it = docs_.find(uri);
    if (it == docs_.end()) return std::nullopt;
    return std::pair{it->second.source, it->second.version};
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

} // namespace tcl_lsp
