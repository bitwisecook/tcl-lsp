#pragma once

#include "tcl_lsp/core/range.hpp"
#include "tcl_lsp/core/source_position.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <vector>

namespace tcl_lsp {

// Which delimiter was left unclosed in a partial command.
enum class UnclosedDelimiter : std::uint8_t { BRACE, BRACKET, QUOTE };

// A single Tcl command parsed from the token stream.
struct SegmentedCommand {
    Range range{};
    std::vector<Token> argv;        // representative token per word
    std::vector<std::string> texts; // reconstructed source text per word
    std::vector<bool> single_token_word;
    std::vector<Token> all_tokens;
    std::optional<std::string> preceding_comment;
    bool is_partial = false;
    std::optional<UnclosedDelimiter> partial_delimiter;
    std::optional<std::vector<bool>> expand_word; // nullopt if no {*}

    // Convenience accessors.
    [[nodiscard]] auto name() const -> std::string;
    [[nodiscard]] auto args() const -> std::vector<std::string>;
    [[nodiscard]] auto arg_tokens() const -> std::vector<Token>;
    [[nodiscard]] auto arg_single_token() const -> std::vector<bool>;
};

// A region of top-level source mapped to its commands, for incremental re-analysis.
struct TopLevelChunk {
    int32_t index = 0;
    int32_t start_offset = 0;
    int32_t end_offset = 0; // exclusive
    std::size_t source_hash = 0;
    std::vector<SegmentedCommand> commands;
};

// Segment a token stream into per-command structures at EOL boundaries.
//
// When known_commands is provided, the segmenter attempts error recovery on
// commands that appear to contain an unclosed delimiter.  Set recovery to
// false to disable this heuristic.
[[nodiscard]] auto
segment_commands(std::string_view source,
                 const Token* body_token = nullptr,
                 const std::unordered_set<std::string>* known_commands = nullptr,
                 const std::unordered_map<int32_t, char>* virtual_insertions = nullptr,
                 std::vector<std::pair<SourcePosition, std::string>>* collect_warnings = nullptr,
                 bool recovery = true) -> std::vector<SegmentedCommand>;

// Split source into top-level chunks, one per command.
[[nodiscard]] auto segment_top_level_chunks(std::string_view source) -> std::vector<TopLevelChunk>;

// Return the index of the first chunk that differs between two versions.
[[nodiscard]] auto find_first_dirty_chunk(const std::vector<TopLevelChunk>& old_chunks,
                                          const std::vector<TopLevelChunk>& new_chunks) -> int32_t;

// Check for a token that looks like an unclosed delimiter in the last command.
// Returns (suspicious token, delimiter type), or nullopt.
[[nodiscard]] auto has_suspicious_token(const SegmentedCommand& cmd, int32_t source_len)
    -> std::optional<std::pair<Token, UnclosedDelimiter>>;

// Find a recovery offset inside a suspicious token's text.
[[nodiscard]] auto find_recovery_offset(std::string_view token_text,
                                        int32_t token_start_offset,
                                        const std::unordered_set<std::string>& known_commands)
    -> std::optional<int32_t>;

} // namespace tcl_lsp
