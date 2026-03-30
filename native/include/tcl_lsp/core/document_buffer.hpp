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
//
// Thread safety: all const methods are safe to call concurrently. This is
// important because BufferSnapshot (shared_ptr<const DocumentBuffer>) may
// be accessed from multiple threads simultaneously.
class DocumentBuffer {
  public:
    static auto from_source(std::string source, std::optional<int> version,
                            uint64_t epoch) -> DocumentBuffer;

    // Overload for callers that bind by function pointer (e.g. pybind11)
    // where C++ default arguments are not visible.
    static auto from_source(std::string source,
                            std::optional<int> version = std::nullopt)
        -> DocumentBuffer;

    // Custom move ops: lines_ contains string_views into source_, so a
    // default move would dangle them (SSO can relocate the buffer).
    // We clear lines_ on move and let the new owner recompute lazily —
    // but in practice, moved-from buffers are immediately placed into
    // make_shared and never moved again.
    DocumentBuffer(DocumentBuffer&& other) noexcept;
    DocumentBuffer& operator=(DocumentBuffer&& other) noexcept;
    DocumentBuffer(const DocumentBuffer&) = delete;
    DocumentBuffer& operator=(const DocumentBuffer&) = delete;

    [[nodiscard]] auto source() const noexcept -> std::string_view;
    [[nodiscard]] auto version() const noexcept -> std::optional<int>;
    [[nodiscard]] auto epoch() const noexcept -> uint64_t;

    [[nodiscard]] auto line_starts() const noexcept -> const std::vector<int32_t>& {
        return line_starts_;
    }

    // Source split by '\n'. Computed eagerly in the constructor — safe for
    // concurrent access on shared snapshots.
    [[nodiscard]] auto lines() const noexcept -> std::span<const std::string_view>;

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

    void compute_lines();

    std::string source_;
    std::optional<int> version_;
    std::vector<int32_t> line_starts_;
    uint64_t epoch_;
    std::vector<std::string_view> lines_;
};

} // namespace tcl_lsp
