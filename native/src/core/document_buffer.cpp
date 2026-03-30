#include "tcl_lsp/core/document_buffer.hpp"

#include <algorithm>
#include <cstdint>

namespace tcl_lsp {

namespace {

auto compute_line_starts(std::string_view source) -> std::vector<int32_t> {
    std::vector<int32_t> starts;
    starts.reserve(source.size() / 40 + 1); // rough estimate: ~40 chars/line
    starts.push_back(0);
    for (int32_t i = 0; i < static_cast<int32_t>(source.size()); ++i) {
        if (source[static_cast<std::size_t>(i)] == '\n') {
            starts.push_back(i + 1);
        }
    }
    return starts;
}

} // namespace

DocumentBuffer::DocumentBuffer(std::string source,
                               std::optional<int> version,
                               std::vector<int32_t> line_starts,
                               uint64_t epoch)
    : source_(std::move(source))
    , version_(version)
    , line_starts_(std::move(line_starts))
    , epoch_(epoch) {}

auto DocumentBuffer::from_source(std::string source,
                                 std::optional<int> version,
                                 uint64_t epoch) -> DocumentBuffer {
    auto line_starts = compute_line_starts(source);
    return {std::move(source), version, std::move(line_starts), epoch};
}

auto DocumentBuffer::source() const noexcept -> std::string_view {
    return source_;
}

auto DocumentBuffer::version() const noexcept -> std::optional<int> {
    return version_;
}

auto DocumentBuffer::epoch() const noexcept -> uint64_t {
    return epoch_;
}

auto DocumentBuffer::lines() const -> std::span<const std::string_view> {
    if (!lines_cache_) {
        std::vector<std::string_view> result;
        result.reserve(line_starts_.size());
        for (std::size_t i = 0; i < line_starts_.size(); ++i) {
            auto start = static_cast<std::size_t>(line_starts_[i]);
            std::size_t end = 0;
            if (i + 1 < line_starts_.size()) {
                // Exclude the trailing '\n'.
                end = static_cast<std::size_t>(line_starts_[i + 1]) - 1;
            } else {
                end = source_.size();
            }
            result.push_back(std::string_view(source_).substr(start, end - start));
        }
        lines_cache_ = std::move(result);
    }
    return *lines_cache_;
}

auto DocumentBuffer::offset_to_line_col(int32_t offset) const -> std::pair<int32_t, int32_t> {
    auto safe = std::clamp(offset, int32_t{0}, static_cast<int32_t>(source_.size()));
    // upper_bound gives the first line_start > safe; subtract 1 for the line.
    auto it = std::upper_bound(line_starts_.begin(), line_starts_.end(), safe);
    auto line = static_cast<int32_t>(std::distance(line_starts_.begin(), it)) - 1;
    line = std::max(int32_t{0}, line);
    auto col = safe - line_starts_[static_cast<std::size_t>(line)];
    return {line, col};
}

auto DocumentBuffer::offset_to_position(int32_t offset) const -> SourcePosition {
    auto safe = std::clamp(offset, int32_t{0}, static_cast<int32_t>(source_.size()));
    auto [line, col] = offset_to_line_col(safe);
    return {line, col, safe};
}

auto DocumentBuffer::position_to_offset(int32_t line, int32_t character) const -> int32_t {
    if (line_starts_.empty()) {
        return 0;
    }
    auto safe_line = std::clamp(line, int32_t{0}, static_cast<int32_t>(line_starts_.size()) - 1);
    auto line_start = line_starts_[static_cast<std::size_t>(safe_line)];

    int32_t line_end = 0;
    if (static_cast<std::size_t>(safe_line) + 1 < line_starts_.size()) {
        line_end = line_starts_[static_cast<std::size_t>(safe_line) + 1] - 1; // exclude '\n'
    } else {
        line_end = static_cast<int32_t>(source_.size());
    }

    auto line_length = std::max(int32_t{0}, line_end - line_start);
    auto safe_char = std::clamp(character, int32_t{0}, line_length);
    return line_start + safe_char;
}

auto DocumentBuffer::range_from_offsets(int32_t start, int32_t end_inclusive) const -> Range {
    if (source_.empty()) {
        auto pos = SourcePosition{0, 0, 0};
        return {pos, pos};
    }

    auto max_end = static_cast<int32_t>(source_.size()) - 1;
    auto safe_start = std::clamp(start, int32_t{0}, max_end);
    auto safe_end = std::clamp(end_inclusive, int32_t{0}, max_end);
    if (safe_end < safe_start) {
        safe_end = safe_start;
    }

    return {offset_to_position(safe_start), offset_to_position(safe_end)};
}

auto DocumentBuffer::chunk_line_range(int32_t start_offset, int32_t end_offset) const
    -> std::tuple<int32_t, int32_t, int32_t, int32_t> {
    auto [sl, sc] = offset_to_line_col(start_offset);
    auto [el, ec] = offset_to_line_col(end_offset);
    return {sl, sc, el, ec};
}

} // namespace tcl_lsp
