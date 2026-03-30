#pragma once

#include "tcl_lsp/core/range.hpp"
#include "tcl_lsp/core/source_position.hpp"

#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <tuple>
#include <utility>
#include <vector>

namespace tcl_lsp {

// Per-document position infrastructure.
//
// Owns the source text and provides O(log n) offset-to-position conversion
// via a precomputed line-starts index. Replaces scattered source.split("\n")
// and ad-hoc SourceMap construction in Python.
class DocumentBuffer {
  public:
    static auto from_source(std::string source, std::optional<int> version = std::nullopt,
                            uint64_t epoch = 0) -> DocumentBuffer;

    DocumentBuffer(DocumentBuffer&&) = default;
    DocumentBuffer& operator=(DocumentBuffer&&) = default;
    DocumentBuffer(const DocumentBuffer&) = delete;
    DocumentBuffer& operator=(const DocumentBuffer&) = delete;

    [[nodiscard]] auto source() const noexcept -> std::string_view;
    [[nodiscard]] auto version() const noexcept -> std::optional<int>;
    [[nodiscard]] auto epoch() const noexcept -> uint64_t;

    [[nodiscard]] auto line_starts() const noexcept -> const std::vector<int32_t>& {
        return line_starts_;
    }

    // Source split by '\n', lazily cached.
    [[nodiscard]] auto lines() const -> std::span<const std::string_view>;

    // O(log n) offset -> position via binary search on line_starts.
    [[nodiscard]] auto offset_to_position(int32_t offset) const -> SourcePosition;

    // O(1) (line, character) -> offset with clamping.
    [[nodiscard]] auto position_to_offset(int32_t line, int32_t character) const -> int32_t;

    // O(log n) offset -> (line, col) without allocating a SourcePosition.
    [[nodiscard]] auto offset_to_line_col(int32_t offset) const -> std::pair<int32_t, int32_t>;

    // Build a Range from inclusive source offsets.
    [[nodiscard]] auto range_from_offsets(int32_t start, int32_t end_inclusive) const -> Range;

    // O(log n) chunk line range: returns (start_line, start_col, end_line, end_col).
    [[nodiscard]] auto chunk_line_range(int32_t start_offset, int32_t end_offset) const
        -> std::tuple<int32_t, int32_t, int32_t, int32_t>;

  private:
    DocumentBuffer(std::string source,
                   std::optional<int> version,
                   std::vector<int32_t> line_starts,
                   uint64_t epoch);

    std::string source_;
    std::optional<int> version_;
    std::vector<int32_t> line_starts_;
    uint64_t epoch_;
    mutable std::optional<std::vector<std::string_view>> lines_cache_;
};

} // namespace tcl_lsp
