#pragma once

#include "tcl_lsp/analysis/analysis_result.hpp"
#include "tcl_lsp/analysis/arg_role_resolver.hpp"
#include "tcl_lsp/analysis/command_interface.hpp"
#include "tcl_lsp/lsp/semantic_token_types.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <unordered_set>
#include <vector>

namespace tcl_lsp {

// Collect semantic tokens from Tcl source.
// Produces absolute-position tokens sorted by (line, character).
class SemanticTokenCollector {
  public:
    explicit SemanticTokenCollector(const CommandRegistryInterface* registry = nullptr);

    // Main entry point: collect all semantic tokens for the given source.
    [[nodiscard]] auto collect(std::string_view source,
                                const AnalysisResult* analysis = nullptr)
        -> std::vector<SemanticToken>;

  private:
    using TokenList = std::vector<SemanticToken>;
    using PosSet = std::unordered_set<std::uint64_t>;

    // Recursive token collection.
    void collect_tokens(TokenList& out,
                        std::string_view source,
                        const Token* body_token,
                        const PosSet& regex_positions,
                        const std::vector<std::int32_t>* shared_line_starts,
                        std::int32_t source_len);

    // Expression sub-tokenization.
    void collect_expression_tokens(TokenList& out,
                                    std::string_view expr_text,
                                    const Token& expr_token,
                                    const PosSet& regex_positions,
                                    const std::vector<std::int32_t>& line_starts,
                                    std::int32_t source_len);

    // Classify a single lexer token.
    [[nodiscard]] static auto classify_token(TokenType type,
                                              std::string_view text,
                                              bool is_command_name)
        -> std::optional<SemanticTokenType>;

    // Append a token, splitting across lines if needed.
    static void append_text_token(TokenList& out,
                                   std::int32_t line,
                                   std::int32_t character,
                                   std::string_view text,
                                   SemanticTokenType type,
                                   std::uint8_t modifiers = 0);

    // Emit namespace-qualified name as namespace + command tokens.
    static void emit_namespace_qualified(TokenList& out,
                                          const Token& tok,
                                          SemanticTokenType type,
                                          std::uint8_t modifiers);

    const CommandRegistryInterface* registry_;
};

// Delta-encode absolute-position tokens to LSP 5-int format.
// Tokens must be sorted by (line, character).
[[nodiscard]] auto delta_encode(const std::vector<SemanticToken>& tokens)
    -> std::vector<std::int32_t>;

// Compute edits to transform old_data into new_data.
struct SemanticTokenEdit {
    std::int32_t start;
    std::int32_t delete_count;
    std::vector<std::int32_t> data;
};

[[nodiscard]] auto compute_semantic_token_edits(
    std::span<const std::int32_t> old_data,
    std::span<const std::int32_t> new_data)
    -> std::vector<SemanticTokenEdit>;

} // namespace tcl_lsp
