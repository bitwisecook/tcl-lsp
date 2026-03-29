#include "tcl_lsp/lsp/semantic_token_collector.hpp"

#include "tcl_lsp/lsp/known_commands.hpp"
#include "tcl_lsp/lsp/semantic_token_types.hpp"
#include "tcl_lsp/parsing/expr_lexer.hpp"
#include "tcl_lsp/parsing/lexer.hpp"
#include "tcl_lsp/parsing/recovery.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <optional>
#include <regex>
#include <span>
#include <string>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

namespace tcl_lsp {
namespace {

// Build a position key for the regex position set.
auto pos_key(std::int32_t line, std::int32_t character) -> std::uint64_t {
    return (static_cast<std::uint64_t>(static_cast<std::uint32_t>(line)) << 32) |
            static_cast<std::uint64_t>(static_cast<std::uint32_t>(character));
}

// Check if a character is a digit.
auto is_number_text(std::string_view text) -> bool {
    if (text.empty()) return false;
    bool has_dot = false;
    std::size_t start = 0;
    if (text[0] == '-' || text[0] == '+') start = 1;
    if (start >= text.size()) return false;
    // Hex/oct/bin prefix
    if (text.size() > start + 1 && text[start] == '0' &&
        (text[start + 1] == 'x' || text[start + 1] == 'X' ||
         text[start + 1] == 'o' || text[start + 1] == 'O' ||
         text[start + 1] == 'b' || text[start + 1] == 'B')) {
        for (std::size_t i = start + 2; i < text.size(); i++) {
            if (!std::isalnum(static_cast<unsigned char>(text[i]))) return false;
        }
        return text.size() > start + 2;
    }
    for (std::size_t i = start; i < text.size(); i++) {
        char ch = text[i];
        if (ch == '.') {
            if (has_dot) return false;
            has_dot = true;
        } else if (ch == 'e' || ch == 'E') {
            // Accept scientific notation.
            if (i + 1 < text.size() && (text[i + 1] == '+' || text[i + 1] == '-'))
                i += 1;
        } else if (!std::isdigit(static_cast<unsigned char>(ch))) {
            return false;
        }
    }
    return true;
}

// Compute text length for token rendering (accounts for delimiters).
auto rendered_text_for_token(const Token& tok) -> std::string {
    if (tok.type == TokenType::VAR) {
        auto span = tok.end.offset - tok.start.offset + 1;
        if (span > static_cast<std::int32_t>(tok.text.size()) + 1)
            return std::string("${") + std::string(tok.text) + "}";
        return "$" + std::string(tok.text);
    }
    if (tok.type == TokenType::STR)
        return "{" + std::string(tok.text) + "}";
    return std::string(tok.text);
}

// Check if text looks like an iRule event name (ALL_CAPS with underscores).
auto is_event_name(std::string_view text) -> bool {
    if (text.empty()) return false;
    for (char ch : text) {
        if (ch != '_' && !std::isupper(static_cast<unsigned char>(ch)) &&
            !std::isdigit(static_cast<unsigned char>(ch)))
            return false;
    }
    return std::isupper(static_cast<unsigned char>(text[0])) != 0;
}

} // anonymous namespace

SemanticTokenCollector::SemanticTokenCollector(const CommandRegistryInterface* registry)
    : registry_(registry) {}

auto SemanticTokenCollector::collect(std::string_view source,
                                      const AnalysisResult* analysis)
    -> std::vector<SemanticToken> {
    PosSet regex_positions;
    if (analysis != nullptr) {
        for (auto& rp : analysis->regex_patterns()) {
            regex_positions.insert(pos_key(rp.range.start.line, rp.range.start.character));
        }
        // Also add from regex_position_set if exposed.
    }

    TokenList tokens;
    tokens.reserve(256);
    collect_tokens(tokens, source, nullptr, regex_positions, nullptr, 0);

    // Sort by (line, character).
    std::sort(tokens.begin(), tokens.end(), [](const SemanticToken& a, const SemanticToken& b) {
        if (a.line != b.line) return a.line < b.line;
        return a.character < b.character;
    });

    return tokens;
}

void SemanticTokenCollector::collect_tokens(
    TokenList& out,
    std::string_view source,
    const Token* body_token,
    const PosSet& regex_positions,
    const std::vector<std::int32_t>* shared_line_starts,
    std::int32_t source_len) {

    // Compute ghost insertions for error recovery.
    auto ghosts = compute_ghost_insertions(source, body_token);

    // Create lexer.
    std::int32_t base_offset = 0;
    std::int32_t base_line = 0;
    std::int32_t base_col = 0;
    if (body_token != nullptr) {
        base_offset = body_token->start.offset + 1;
        base_line = body_token->start.line;
        base_col = body_token->start.character + 1;
    }

    TclLexer lexer(source, {}, base_offset, base_line, base_col,
                    std::move(ghosts), shared_line_starts);

    const auto* line_starts_ptr = &lexer.line_starts();
    if (shared_line_starts == nullptr && body_token == nullptr) {
        source_len = static_cast<std::int32_t>(source.size());
    }

    // Track commands: group tokens by command boundaries.
    std::vector<Token> argv;        // first token per argument
    std::vector<std::string> argv_texts;  // concatenated text per argument
    std::vector<Token> all_tokens;  // all tokens in current command
    auto prev_type = TokenType::EOL;

    // Flush: process a complete command.
    auto flush_command = [&]() {
        if (argv.empty()) return;
        auto& cmd_name = argv_texts[0];

        // Get arg texts (after command name).
        std::vector<std::string> arg_texts(argv_texts.begin() + 1, argv_texts.end());

        // Look up argument roles.
        static constexpr ArgRole ROLES[] = {
            ArgRole::BODY, ArgRole::EXPR, ArgRole::VAR_NAME, ArgRole::VAR_READ, ArgRole::PATTERN,
        };
        auto role_sets = arg_indices_for_roles(registry_, cmd_name, arg_texts,
                                                std::span<const ArgRole>(ROLES, 5));
        auto& body_indices = role_sets[0];
        auto& expr_indices = role_sets[1];
        auto& varname_indices = role_sets[2];
        auto& varread_indices = role_sets[3];
        auto& pattern_indices = role_sets[4];

        // Merge VAR_NAME and VAR_READ.
        for (auto idx : varread_indices) varname_indices.insert(idx);

        // Special arg indices.
        auto param_idx = proc_param_list_arg_index(cmd_name, argv_texts);
        auto procname_idx = procedure_name_arg_index(cmd_name, argv_texts);
        auto subcmd_idx = subcommand_arg_index(cmd_name, argv_texts);
        auto option_idxs = option_arg_indices(cmd_name, arg_texts);

        // Walk tokens.
        std::int32_t arg_idx = -1; // -1 = command name
        auto prev = TokenType::EOL;

        for (auto& tok : all_tokens) {
            if (tok.type == TokenType::SEP) {
                prev = tok.type;
                continue;
            }

            // Determine argument index.
            if (prev == TokenType::SEP || prev == TokenType::EOL)
                arg_idx += 1;
            prev = tok.type;

            bool is_cmd_name = (arg_idx == 0);
            auto role_idx = arg_idx - 1; // 0-based after command name
            bool is_body = !is_cmd_name && body_indices.count(role_idx) > 0;
            bool is_expr = !is_cmd_name && expr_indices.count(role_idx) > 0;
            bool is_varname = !is_cmd_name && varname_indices.count(role_idx) > 0;
            bool is_pattern = !is_cmd_name && pattern_indices.count(role_idx) > 0;
            bool is_option = !is_cmd_name && option_idxs.count(role_idx) > 0;

            // CMD tokens: always recurse.
            if (tok.type == TokenType::CMD) {
                collect_tokens(out, tok.text, &tok, regex_positions,
                                line_starts_ptr, source_len);
                continue;
            }

            // STR in BODY role: recurse.
            if (tok.type == TokenType::STR && is_body && !tok.text.empty()) {
                collect_tokens(out, tok.text, &tok, regex_positions,
                                line_starts_ptr, source_len);
                continue;
            }

            // STR in EXPR role: expression sub-lexer.
            if (tok.type == TokenType::STR && is_expr && !tok.text.empty()) {
                collect_expression_tokens(out, tok.text, tok, regex_positions,
                                           *line_starts_ptr, source_len);
                continue;
            }

            // Proc name definition.
            if (procname_idx.has_value() && arg_idx == *procname_idx &&
                static_cast<std::size_t>(arg_idx) < argv.size() &&
                (tok.type == TokenType::ESC || tok.type == TokenType::STR)) {
                auto rendered = rendered_text_for_token(tok);
                append_text_token(out, tok.start.line, tok.start.character,
                                   rendered, SemanticTokenType::FUNCTION,
                                   modifier_bit(SemanticTokenModifier::DEFINITION));
                continue;
            }

            // Known subcommand.
            if (subcmd_idx.has_value() && arg_idx == *subcmd_idx &&
                static_cast<std::size_t>(arg_idx) < argv.size() &&
                (tok.type == TokenType::ESC || tok.type == TokenType::STR) &&
                is_known_subcommand(registry_, cmd_name, argv_texts[static_cast<std::size_t>(arg_idx)])) {
                auto rendered = rendered_text_for_token(tok);
                append_text_token(out, tok.start.line, tok.start.character,
                                   rendered, SemanticTokenType::KEYWORD,
                                   modifier_bit(SemanticTokenModifier::DEFAULT_LIBRARY));
                continue;
            }

            // Parameter list.
            if (param_idx.has_value() && arg_idx == *param_idx &&
                static_cast<std::size_t>(arg_idx) < argv.size() &&
                tok.type == TokenType::STR) {
                // Emit the whole param list as parameter tokens.
                auto rendered = rendered_text_for_token(tok);
                append_text_token(out, tok.start.line, tok.start.character,
                                   rendered, SemanticTokenType::PARAMETER,
                                   modifier_bit(SemanticTokenModifier::DECLARATION));
                continue;
            }

            // Variable name arguments.
            if (is_varname && tok.type == TokenType::ESC) {
                append_text_token(out, tok.start.line, tok.start.character,
                                   tok.text, SemanticTokenType::VARIABLE,
                                   modifier_bit(SemanticTokenModifier::DECLARATION));
                continue;
            }

            // Regex pattern arguments.
            if (is_pattern && (tok.type == TokenType::ESC || tok.type == TokenType::STR)) {
                auto rendered = rendered_text_for_token(tok);
                append_text_token(out, tok.start.line, tok.start.character,
                                   rendered, SemanticTokenType::REGEXP);
                continue;
            }

            // Analysis-driven regex override.
            if (!regex_positions.empty() &&
                regex_positions.count(pos_key(tok.start.line, tok.start.character)) > 0) {
                auto rendered = rendered_text_for_token(tok);
                append_text_token(out, tok.start.line, tok.start.character,
                                   rendered, SemanticTokenType::REGEXP);
                continue;
            }

            // iRule event name in "when EVENT { body }".
            if (cmd_name == "when" && arg_idx == 1 &&
                tok.type == TokenType::ESC && is_event_name(tok.text)) {
                append_text_token(out, tok.start.line, tok.start.character,
                                   tok.text, SemanticTokenType::EVENT);
                continue;
            }

            // Option/flag highlighting.
            if (is_option && tok.type == TokenType::ESC &&
                !tok.text.empty() && tok.text[0] == '-') {
                append_text_token(out, tok.start.line, tok.start.character,
                                   tok.text, SemanticTokenType::DECORATOR);
                continue;
            }

            // Default classification.
            auto type_opt = classify_token(tok.type, tok.text, is_cmd_name);
            if (!type_opt.has_value()) continue;
            auto sem_type = *type_opt;

            std::uint8_t modifiers = 0;
            if (is_cmd_name && sem_type == SemanticTokenType::KEYWORD)
                modifiers = modifier_bit(SemanticTokenModifier::DEFAULT_LIBRARY);

            // Compute rendered text for proper length.
            std::string rendered;
            if (tok.type == TokenType::VAR) {
                rendered = rendered_text_for_token(tok);
            } else {
                rendered = std::string(tok.text);
            }

            // Namespace-qualified command names: split into namespace + command.
            if (is_cmd_name && tok.text.find("::") != std::string_view::npos &&
                tok.type != TokenType::COMMENT) {
                emit_namespace_qualified(out, tok, sem_type, modifiers);
                continue;
            }

            append_text_token(out, tok.start.line, tok.start.character,
                               rendered, sem_type, modifiers);
        }
    };

    // Main loop: walk tokens, grouping by command.
    while (true) {
        auto tok_opt = lexer.get_token();
        if (!tok_opt.has_value()) break;
        auto& tok = *tok_opt;

        if (tok.type == TokenType::SEP) {
            all_tokens.push_back(tok);
            prev_type = tok.type;
            continue;
        }
        if (tok.type == TokenType::EOL) {
            flush_command();
            argv.clear();
            argv_texts.clear();
            all_tokens.clear();
            prev_type = tok.type;
            continue;
        }

        all_tokens.push_back(tok);

        // Build argv for command identification.
        if (prev_type == TokenType::SEP || prev_type == TokenType::EOL) {
            argv.push_back(tok);
            argv_texts.push_back(std::string(tok.text));
        } else {
            if (!argv_texts.empty()) {
                argv_texts.back() += tok.text;
            } else {
                argv.push_back(tok);
                argv_texts.push_back(std::string(tok.text));
            }
        }

        prev_type = tok.type;
    }

    // Handle trailing command without final EOL.
    flush_command();
}

void SemanticTokenCollector::collect_expression_tokens(
    TokenList& out,
    std::string_view expr_text,
    const Token& expr_token,
    const PosSet& regex_positions,
    const std::vector<std::int32_t>& line_starts,
    std::int32_t source_len) {

    auto expr_tokens = tokenise_expr(expr_text);

    // The expression body starts after the opening brace.
    auto base_line = expr_token.start.line;
    auto base_col = expr_token.start.character + 1;

    for (auto& et : expr_tokens) {
        if (et.type == ExprTokenType::WHITESPACE || et.type == ExprTokenType::EOF_)
            continue;

        // For CMD tokens inside expressions, recurse into the command.
        if (et.type == ExprTokenType::COMMAND) {
            // Skip the surrounding [ ].
            if (et.text.size() > 2) {
                auto inner = et.text.substr(1, et.text.size() - 2);
                // Create a synthetic body token for position tracking.
                Token body_tok;
                body_tok.type = TokenType::CMD;
                body_tok.text = std::string(inner);
                body_tok.start.line = base_line;
                body_tok.start.character = base_col + et.start;
                body_tok.start.offset = expr_token.start.offset + 1 + et.start;
                body_tok.end = body_tok.start;
                body_tok.end.offset += static_cast<std::int32_t>(et.text.size()) - 1;
                collect_tokens(out, inner, &body_tok, regex_positions,
                                &line_starts, source_len);
            }
            continue;
        }

        std::optional<SemanticTokenType> sem_type;
        switch (et.type) {
            case ExprTokenType::VARIABLE:
                sem_type = SemanticTokenType::VARIABLE;
                break;
            case ExprTokenType::NUMBER:
                sem_type = SemanticTokenType::NUMBER;
                break;
            case ExprTokenType::OPERATOR:
            case ExprTokenType::PAREN_OPEN:
            case ExprTokenType::PAREN_CLOSE:
            case ExprTokenType::TERNARY_Q:
            case ExprTokenType::TERNARY_C:
            case ExprTokenType::COMMA:
                sem_type = SemanticTokenType::OPERATOR;
                break;
            case ExprTokenType::FUNCTION:
                sem_type = SemanticTokenType::FUNCTION;
                break;
            case ExprTokenType::BOOL:
                sem_type = SemanticTokenType::KEYWORD;
                break;
            case ExprTokenType::STRING:
                sem_type = SemanticTokenType::STRING;
                break;
            default:
                break;
        }

        if (!sem_type.has_value()) continue;

        // Compute absolute position.
        auto line = base_line;
        auto col = base_col + et.start;
        // Account for newlines within the expression.
        for (std::int32_t i = 0; i < et.start && static_cast<std::size_t>(i) < expr_text.size(); i++) {
            if (expr_text[static_cast<std::size_t>(i)] == '\n') {
                line += 1;
                col = et.start - i - 1;
                // Reset col calculation for multi-line expressions.
                // We need to recalculate from the last newline.
            }
        }
        // More accurate: scan from start of expression to token start.
        line = base_line;
        col = base_col;
        for (std::int32_t i = 0; i < et.start && static_cast<std::size_t>(i) < expr_text.size(); i++) {
            if (expr_text[static_cast<std::size_t>(i)] == '\n') {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        out.push_back({line, col,
                        static_cast<std::int32_t>(et.text.size()),
                        *sem_type, 0});
    }
}

auto SemanticTokenCollector::classify_token(TokenType type,
                                             std::string_view text,
                                             bool is_command_name)
    -> std::optional<SemanticTokenType> {
    switch (type) {
        case TokenType::VAR:
            return SemanticTokenType::VARIABLE;
        case TokenType::CMD:
            return std::nullopt; // Handled by recursion.
        case TokenType::STR:
            return SemanticTokenType::STRING;
        case TokenType::COMMENT:
            return SemanticTokenType::COMMENT;
        case TokenType::ESC:
            if (is_command_name) {
                if (is_keyword(text)) return SemanticTokenType::KEYWORD;
                if (is_operator(text)) return SemanticTokenType::OPERATOR;
                return SemanticTokenType::FUNCTION;
            }
            if (is_number_text(text)) return SemanticTokenType::NUMBER;
            return SemanticTokenType::STRING;
        case TokenType::SEP:
        case TokenType::EOL:
        case TokenType::EOF_:
        case TokenType::EXPAND:
            return std::nullopt;
    }
    return std::nullopt;
}

void SemanticTokenCollector::append_text_token(TokenList& out,
                                                std::int32_t line,
                                                std::int32_t character,
                                                std::string_view text,
                                                SemanticTokenType type,
                                                std::uint8_t modifiers) {
    if (text.empty()) return;

    // Split across newlines.
    auto cur_line = line;
    auto cur_char = character;
    std::size_t pos = 0;

    while (pos < text.size()) {
        auto nl = text.find('\n', pos);
        auto end = (nl == std::string_view::npos) ? text.size() : nl;
        auto len = static_cast<std::int32_t>(end - pos);
        if (len > 0) {
            out.push_back({cur_line, cur_char, len, type, modifiers});
        }
        if (nl == std::string_view::npos) break;
        cur_line += 1;
        cur_char = 0;
        pos = nl + 1;
    }
}

void SemanticTokenCollector::emit_namespace_qualified(TokenList& out,
                                                       const Token& tok,
                                                       SemanticTokenType type,
                                                       std::uint8_t modifiers) {
    auto text = tok.text;
    auto idx = text.rfind("::");
    if (idx == std::string_view::npos) {
        append_text_token(out, tok.start.line, tok.start.character,
                           text, type, modifiers);
        return;
    }

    auto ns_part = text.substr(0, idx + 2);
    auto cmd_part = text.substr(idx + 2);

    append_text_token(out, tok.start.line, tok.start.character,
                       ns_part, SemanticTokenType::NAMESPACE, modifiers);
    if (!cmd_part.empty()) {
        auto cmd_col = tok.start.character + static_cast<std::int32_t>(idx) + 2;
        append_text_token(out, tok.start.line, cmd_col,
                           cmd_part, type, modifiers);
    }
}

// Delta encoding.

auto delta_encode(const std::vector<SemanticToken>& tokens)
    -> std::vector<std::int32_t> {
    std::vector<std::int32_t> data;
    data.resize(tokens.size() * 5);

    std::int32_t prev_line = 0;
    std::int32_t prev_char = 0;
    std::size_t idx = 0;

    for (auto& tok : tokens) {
        auto delta_line = tok.line - prev_line;
        auto delta_char = (delta_line == 0) ? (tok.character - prev_char) : tok.character;
        data[idx] = delta_line;
        data[idx + 1] = delta_char;
        data[idx + 2] = tok.length;
        data[idx + 3] = static_cast<std::int32_t>(tok.type);
        data[idx + 4] = static_cast<std::int32_t>(tok.modifiers);
        idx += 5;
        prev_line = tok.line;
        prev_char = tok.character;
    }

    return data;
}

auto compute_semantic_token_edits(
    std::span<const std::int32_t> old_data,
    std::span<const std::int32_t> new_data)
    -> std::vector<SemanticTokenEdit> {

    auto old_len = static_cast<std::int32_t>(old_data.size());
    auto new_len = static_cast<std::int32_t>(new_data.size());
    auto min_len = std::min(old_len, new_len);

    // Find common prefix (5-int aligned).
    std::int32_t prefix = 0;
    while (prefix < min_len && old_data[static_cast<std::size_t>(prefix)] ==
                                    new_data[static_cast<std::size_t>(prefix)])
        prefix += 1;
    prefix = (prefix / 5) * 5;

    // Find common suffix (5-int aligned).
    std::int32_t suffix = 0;
    while (suffix < (min_len - prefix) &&
           old_data[static_cast<std::size_t>(old_len - 1 - suffix)] ==
               new_data[static_cast<std::size_t>(new_len - 1 - suffix)])
        suffix += 1;
    suffix = (suffix / 5) * 5;

    auto delete_count = old_len - prefix - suffix;
    auto insert_start = static_cast<std::size_t>(prefix);
    auto insert_end = static_cast<std::size_t>(new_len - suffix);

    if (delete_count == 0 && insert_start == insert_end) return {};

    std::vector<std::int32_t> insert_data(new_data.begin() + static_cast<std::ptrdiff_t>(insert_start),
                                           new_data.begin() + static_cast<std::ptrdiff_t>(insert_end));

    return {{prefix, delete_count, std::move(insert_data)}};
}

} // namespace tcl_lsp
