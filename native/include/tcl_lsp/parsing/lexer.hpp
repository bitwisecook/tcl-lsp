#pragma once

#include "tcl_lsp/core/source_position.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>

namespace tcl_lsp {

// Raised for Tcl syntax errors detected during lexing (strict mode only).
class TclParseError : public std::runtime_error {
    using std::runtime_error::runtime_error;
};

// Tag type for owning constructor overload.
struct OwningTag {};

// Configuration flags for the lexer.  These correspond to class-level flags
// in the Python TclLexer that are toggled by the VM and dialect system.
struct LexerConfig {
    bool strict_quoting = false;
    bool expand_syntax = true;
    bool irules_brace_separator = false;
};

// Position-aware Tcl lexer.
//
// Produces Token objects carrying line/column/offset for both start and end.
// The dispatch logic mirrors Tcl tokenisation rules and adds:
//   - line/col tracking in advance()
//   - ${name} and $ns::var and $arr(idx) variable forms
//   - backslash-newline continuation
//   - COMMENT token type
class TclLexer {
  public:
    ~TclLexer() = default;
    TclLexer(const TclLexer&) = delete;
    TclLexer& operator=(const TclLexer&) = delete;
    TclLexer(TclLexer&& other) noexcept;
    TclLexer& operator=(TclLexer&& other) noexcept;

    // Construct from string_view (caller must ensure text outlives lexer).
    explicit TclLexer(std::string_view text,
                      LexerConfig config = {},
                      int32_t base_offset = 0,
                      int32_t base_line = 0,
                      int32_t base_col = 0,
                      std::unordered_map<int32_t, char> ghost_insertions = {},
                      const std::vector<int32_t>* shared_line_starts = nullptr);

    // Construct from owned string (lexer takes ownership, safe for Python bindings).
    explicit TclLexer(std::string text,
                      LexerConfig config,
                      int32_t base_offset,
                      int32_t base_line,
                      int32_t base_col,
                      std::unordered_map<int32_t, char> ghost_insertions,
                      const std::vector<int32_t>* shared_line_starts,
                      struct OwningTag);

    // Return the next token, or std::nullopt at EOF.
    [[nodiscard]] auto get_token() -> std::optional<Token>;

    // Tokenise the entire source, including SEP and EOL tokens.
    [[nodiscard]] auto tokenise_all() -> std::vector<Token>;

    // Remaining characters to process.
    [[nodiscard]] auto remaining() const -> int32_t;

    // Current position in source.
    [[nodiscard]] auto pos() const noexcept -> int32_t { return pos_; }

    // Whether currently inside a quoted string.
    [[nodiscard]] auto insidequote() const noexcept -> bool { return insidequote_; }

    // Non-fatal warnings collected in non-strict mode.
    [[nodiscard]] auto warnings() const noexcept
        -> const std::vector<std::pair<SourcePosition, std::string>>& {
        return warnings_;
    }

    // Access the source text.
    [[nodiscard]] auto text() const noexcept -> std::string_view { return text_; }

    // Access the line starts index.
    [[nodiscard]] auto line_starts() const noexcept -> const std::vector<int32_t>& {
        return line_starts_;
    }

  private:
    // Character at current position (respects ghost insertions).
    [[nodiscard]] auto cur() const -> char;

    // Advance position by n characters.
    void advance(int32_t n = 1);

    // Compute SourcePosition at current tracking position.
    [[nodiscard]] auto position() const -> SourcePosition;

    // Compute SourcePosition for an arbitrary offset using the line index.
    [[nodiscard]] auto pos_at(int32_t offset) const -> SourcePosition;

    // Token text from start_ to end_ inclusive.
    [[nodiscard]] auto token_text() const -> std::string;

    // Parse methods for each token category.
    void parse_sep();
    void parse_eol();
    void parse_command();
    void parse_var();
    void parse_brace();
    void parse_expand();
    void parse_string();
    void parse_comment();

    // Report an error: raises TclParseError in strict mode, appends to
    // warnings_ otherwise.
    void report_error(const std::string& message);
    void report_error_at(const SourcePosition& pos, const std::string& message);

    // Character class predicates (constexpr-friendly).
    static constexpr auto is_sep_char(char ch) -> bool {
        return ch == ' ' || ch == '\t' || ch == '\r' || ch == '\x0b' || ch == '\x0c';
    }

    static constexpr auto is_eol_char(char ch) -> bool { return ch == '\n' || ch == ';'; }

    static constexpr auto is_separator_char(char ch) -> bool {
        return is_sep_char(ch) || is_eol_char(ch);
    }

    static constexpr auto is_after_close_brace(char ch) -> bool { return is_separator_char(ch); }

    static constexpr auto is_after_close_quote(char ch) -> bool {
        return is_separator_char(ch) || ch == ']';
    }

    std::string owned_text_; // Owned copy (used when constructed with string)
    std::string_view text_;
    int32_t len_;
    int32_t pos_ = 0;

    LexerConfig config_;
    int32_t base_offset_;
    int32_t base_line_;
    int32_t base_col_;

    int32_t line_;
    int32_t col_;

    int32_t start_ = 0;
    int32_t end_ = 0;
    TokenType type_ = TokenType::EOL;
    bool at_command_start_ = true;
    bool insidequote_ = false;

    std::vector<std::pair<SourcePosition, std::string>> warnings_;

    std::unordered_map<int32_t, char> ghosts_;
    bool has_ghosts_ = false;

    std::optional<Token> pending_sep_;

    std::vector<int32_t> line_starts_;
};

} // namespace tcl_lsp
