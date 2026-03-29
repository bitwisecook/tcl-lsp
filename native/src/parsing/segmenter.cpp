#include "tcl_lsp/parsing/segmenter.hpp"

#include "tcl_lsp/parsing/lexer.hpp"

#include <algorithm>
#include <functional>
#include <string>
#include <utility>

namespace tcl_lsp {

namespace {

// Minimum number of lines a suspicious STR/ESC token must span to trigger
// recovery.  Tuned to avoid false positives on multi-line string literals.
constexpr int32_t recovery_line_threshold = 3;

// Return the source-level text fragment for a single token.
// Variables get $-prefixed; command substitutions get [...]-wrapped.
auto word_piece(const Token& tok) -> std::string {
    if (tok.type == TokenType::VAR) {
        // Use ${name} form unless the name contains '}'.
        if (tok.text.find('}') != std::string::npos) {
            return "$" + tok.text;
        }
        return "${" + tok.text + "}";
    }
    if (tok.type == TokenType::CMD) {
        return "[" + tok.text + "]";
    }
    return tok.text;
}

// Build a Range spanning from the first to the last token.
auto range_from_tokens(const std::vector<Token>& tokens) -> Range {
    return {tokens.front().start, tokens.back().end};
}

// Extract the first word from a stripped line (up to whitespace/special char).
auto extract_first_word(std::string_view stripped) -> std::string_view {
    std::size_t end = 0;
    while (end < stripped.size()) {
        const char ch = stripped[end];
        if (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == ';' || ch == '{' ||
            ch == '[') {
            break;
        }
        ++end;
    }
    return stripped.substr(0, end);
}

// Create a lexer with appropriate base offsets for body_token.
auto make_lexer(std::string_view source,
                const Token* body_token,
                std::unordered_map<int32_t, char> ghosts) -> TclLexer {
    if (body_token == nullptr) {
        return TclLexer(source, {}, 0, 0, 0, std::move(ghosts));
    }
    if (body_token->type == TokenType::STR || body_token->type == TokenType::CMD) {
        return TclLexer(source,
                        {},
                        body_token->start.offset + 1,
                        body_token->start.line,
                        body_token->start.character + 1,
                        std::move(ghosts));
    }
    return TclLexer(source,
                    {},
                    body_token->start.offset,
                    body_token->start.line,
                    body_token->start.character,
                    std::move(ghosts));
}

// The inner segmentation loop — no error recovery.
auto segment_raw(std::string_view source,
                 const Token* body_token,
                 std::unordered_map<int32_t, char> ghosts,
                 std::vector<std::pair<SourcePosition, std::string>>* collect_warnings)
    -> std::vector<SegmentedCommand> {

    auto lexer = make_lexer(source, body_token, std::move(ghosts));

    std::vector<SegmentedCommand> commands;
    std::vector<Token> argv;
    std::vector<std::string> texts;
    std::vector<bool> single;
    std::vector<bool> expand;
    std::vector<Token> all_tokens;
    TokenType prev_type = TokenType::EOL;
    std::optional<std::string> last_comment;
    bool next_expand = false;
    bool has_expand = false;

    while (true) {
        auto maybe_tok = lexer.get_token();
        if (!maybe_tok) {
            break;
        }
        auto& tok = *maybe_tok;

        if (tok.type == TokenType::COMMENT) {
            auto text = tok.text;
            // Strip leading # and whitespace.
            std::string_view sv = text;
            while (!sv.empty() && sv.front() == '#') {
                sv.remove_prefix(1);
            }
            // Trim leading spaces.
            while (!sv.empty() && (sv.front() == ' ' || sv.front() == '\t')) {
                sv.remove_prefix(1);
            }
            // Trim trailing whitespace.
            while (!sv.empty() && (sv.back() == ' ' || sv.back() == '\t' || sv.back() == '\r')) {
                sv.remove_suffix(1);
            }
            std::string line(sv);

            if (last_comment.has_value()) {
                last_comment.value() += "\n";
                last_comment.value() += line;
            } else {
                last_comment = std::move(line);
            }
            continue;
        }

        if (tok.type == TokenType::SEP) {
            prev_type = tok.type;
            continue;
        }

        if (tok.type == TokenType::EXPAND) {
            all_tokens.push_back(tok);
            next_expand = true;
            has_expand = true;
            prev_type = TokenType::SEP;
            continue;
        }

        if (tok.type == TokenType::EOL) {
            if (!argv.empty()) {
                SegmentedCommand cmd;
                cmd.range = range_from_tokens(all_tokens);
                cmd.argv = std::move(argv);
                cmd.texts = std::move(texts);
                cmd.single_token_word = std::move(single);
                cmd.all_tokens = std::move(all_tokens);
                cmd.preceding_comment = std::move(last_comment);
                if (has_expand) {
                    cmd.expand_word = std::move(expand);
                }
                commands.push_back(std::move(cmd));
                last_comment.reset(); // NOLINT(bugprone-use-after-move) — moved into cmd
            } else {
                // Blank line — reset accumulated comments.
                auto nl_count = std::count(tok.text.begin(), tok.text.end(), '\n');
                if (nl_count > 1) {
                    last_comment.reset();
                }
            }
            argv = {};
            texts = {};
            single = {};
            expand = {};
            all_tokens = {};
            has_expand = false;
            next_expand = false;
            prev_type = tok.type;
            continue;
        }

        all_tokens.push_back(tok);
        auto piece = word_piece(tok);

        if (prev_type == TokenType::SEP || prev_type == TokenType::EOL) {
            argv.push_back(tok);
            texts.push_back(std::move(piece));
            single.push_back(true);
            expand.push_back(next_expand);
            next_expand = false;
        } else {
            if (!texts.empty()) {
                texts.back() += piece;
                single.back() = false;
                // Keep argv token range aligned to the full Tcl word span.
                auto& prev = argv.back();
                prev = Token{prev.type, prev.text, prev.start, tok.end, prev.in_quote};
            } else {
                argv.push_back(tok);
                texts.push_back(std::move(piece));
                single.push_back(true);
                expand.push_back(next_expand);
                next_expand = false;
            }
        }

        prev_type = tok.type;
    }

    // Trailing command without final EOL.
    if (!argv.empty()) {
        SegmentedCommand cmd;
        cmd.range = range_from_tokens(all_tokens);
        cmd.argv = std::move(argv);
        cmd.texts = std::move(texts);
        cmd.single_token_word = std::move(single);
        cmd.all_tokens = std::move(all_tokens);
        cmd.preceding_comment = std::move(last_comment);
        if (has_expand) {
            cmd.expand_word = std::move(expand);
        }
        commands.push_back(std::move(cmd));
    }

    // Harvest non-fatal warnings from the lexer.
    if (collect_warnings != nullptr) {
        const auto& w = lexer.warnings();
        collect_warnings->insert(collect_warnings->end(), w.begin(), w.end());
    }

    return commands;
}

} // anonymous namespace

// SegmentedCommand accessors.

auto SegmentedCommand::name() const -> std::string {
    if (texts.empty()) {
        return "";
    }
    return texts[0];
}

auto SegmentedCommand::args() const -> std::span<const std::string> {
    if (texts.size() <= 1)
        return {};
    return {texts.data() + 1, texts.size() - 1};
}

auto SegmentedCommand::arg_tokens() const -> std::span<const Token> {
    if (argv.size() <= 1)
        return {};
    return {argv.data() + 1, argv.size() - 1};
}

auto SegmentedCommand::arg_single_token() const -> std::vector<bool> {
    if (single_token_word.size() <= 1)
        return {};
    return {single_token_word.begin() + 1, single_token_word.end()};
}

// Suspicious token detection.

auto has_suspicious_token(const SegmentedCommand& cmd, int32_t source_len)
    -> std::optional<std::pair<Token, UnclosedDelimiter>> {

    for (const auto& tok : cmd.all_tokens) {
        UnclosedDelimiter delimiter = UnclosedDelimiter::BRACE;
        if (tok.type == TokenType::STR) {
            delimiter = UnclosedDelimiter::BRACE;
        } else if (tok.type == TokenType::CMD) {
            delimiter = UnclosedDelimiter::BRACKET;
        } else if (tok.type == TokenType::ESC) {
            delimiter = UnclosedDelimiter::QUOTE;
        } else {
            continue;
        }

        // CMD tokens reaching EOF are always unterminated — no line-span threshold.
        if (delimiter == UnclosedDelimiter::BRACKET) {
            if (tok.end.offset >= source_len - 1) {
                return std::pair{tok, delimiter};
            }
            continue;
        }

        const int32_t line_span = tok.end.line - tok.start.line;
        if (line_span < recovery_line_threshold) {
            continue;
        }
        if (tok.end.offset >= source_len - 1) {
            return std::pair{tok, delimiter};
        }
    }

    return std::nullopt;
}

// Recovery offset finding.

auto find_recovery_offset(std::string_view token_text,
                          int32_t token_start_offset,
                          const std::unordered_set<std::string>& known_commands)
    -> std::optional<int32_t> {

    int32_t inner_offset = 0;
    int32_t line_num = 0;
    std::size_t pos = 0;

    while (pos <= token_text.size()) {
        // Find end of current line.
        auto nl = token_text.find('\n', pos);
        auto line_end = (nl != std::string_view::npos) ? nl : token_text.size();
        auto line = token_text.substr(pos, line_end - pos);

        if (line_num == 0) {
            // Skip the first line — it's part of the broken command.
            inner_offset += static_cast<int32_t>(line.size()) + 1;
            pos = line_end + 1;
            ++line_num;
            continue;
        }

        // Strip leading whitespace.
        auto stripped = line;
        std::size_t leading_ws = 0;
        while (leading_ws < stripped.size() &&
               (stripped[leading_ws] == ' ' || stripped[leading_ws] == '\t')) {
            ++leading_ws;
        }
        stripped.remove_prefix(leading_ws);

        if (!stripped.empty()) {
            auto first_word = extract_first_word(stripped);
            if (known_commands.contains(std::string(first_word))) {
                // source_offset = token_start_offset(the '{') + 1(for content)
                //               + inner_offset + leading_whitespace
                return token_start_offset + 1 + inner_offset + static_cast<int32_t>(leading_ws);
            }
        }

        inner_offset += static_cast<int32_t>(line.size()) + 1;
        pos = line_end + 1;
        ++line_num;
    }

    return std::nullopt;
}

// Public entry point.

auto segment_commands(std::string_view source,
                      const Token* body_token,
                      const std::unordered_set<std::string>* known_commands,
                      const std::unordered_map<int32_t, char>* ghost_insertions,
                      std::vector<std::pair<SourcePosition, std::string>>* collect_warnings,
                      bool recovery) -> std::vector<SegmentedCommand> {

    std::unordered_map<int32_t, char> ghosts;
    if (ghost_insertions != nullptr) {
        ghosts = *ghost_insertions;
    }

    auto commands = segment_raw(source, body_token, std::move(ghosts), collect_warnings);

    if (commands.empty()) {
        return commands;
    }

    if (!recovery) {
        return commands;
    }

    // Only attempt recovery for top-level segmentation.
    if (body_token != nullptr) {
        return commands;
    }

    // Check the last command for a suspicious token.
    auto& last_cmd = commands.back();
    auto result = has_suspicious_token(last_cmd, static_cast<int32_t>(source.size()));
    if (!result) {
        return commands;
    }

    auto& [suspicious_tok, delimiter] = *result;

    // Need known commands for recovery.
    if (known_commands == nullptr || known_commands->empty()) {
        return commands;
    }

    auto recovery_off =
        find_recovery_offset(suspicious_tok.text, suspicious_tok.start.offset, *known_commands);
    if (!recovery_off) {
        return commands;
    }

    // Mark the broken command as partial.
    last_cmd.is_partial = true;
    last_cmd.partial_delimiter = delimiter;

    // Truncate the partial command range.
    const SourcePosition partial_end{
        suspicious_tok.start.line, suspicious_tok.start.character, suspicious_tok.start.offset};
    last_cmd.range = Range{last_cmd.range.start, partial_end};

    // Re-segment from the recovery point.
    auto offset = *recovery_off;
    if (offset >= 0 && static_cast<std::size_t>(offset) < source.size()) {
        auto remaining = source.substr(static_cast<std::size_t>(offset));
        if (!remaining.empty()) {
            // Trim trailing whitespace to check if there's real content.
            auto trimmed = remaining;
            while (!trimmed.empty() && (trimmed.back() == ' ' || trimmed.back() == '\t' ||
                                        trimmed.back() == '\n' || trimmed.back() == '\r')) {
                trimmed.remove_suffix(1);
            }
            if (!trimmed.empty()) {
                const Token body{TokenType::ESC,
                                 std::string(remaining),
                                 {0, 0, offset},
                                 {0, 0, offset + static_cast<int32_t>(remaining.size())}};
                auto recovered = segment_raw(remaining, &body, {}, collect_warnings);
                commands.insert(commands.end(),
                                std::make_move_iterator(recovered.begin()),
                                std::make_move_iterator(recovered.end()));
            }
        }
    }

    return commands;
}

// Top-level chunking.

auto segment_top_level_chunks(std::string_view source) -> std::vector<TopLevelChunk> {
    auto commands = segment_commands(source);

    std::vector<TopLevelChunk> chunks;
    chunks.reserve(commands.size());

    for (std::size_t i = 0; i < commands.size(); ++i) {
        auto& cmd = commands[i];
        const int32_t start = cmd.range.start.offset;
        const int32_t cmd_end = cmd.range.end.offset;

        // Extend end to include trailing whitespace/newlines up to next command.
        int32_t tile_end = 0;
        if (i + 1 < commands.size()) {
            tile_end = commands[i + 1].range.start.offset;
        } else {
            tile_end = static_cast<int32_t>(source.size());
        }

        // Hash only the command text (not trailing whitespace).
        auto cmd_text = source.substr(static_cast<std::size_t>(start),
                                      static_cast<std::size_t>(cmd_end - start + 1));
        auto hash = std::hash<std::string_view>{}(cmd_text);

        TopLevelChunk chunk;
        chunk.index = static_cast<int32_t>(i);
        chunk.start_offset = start;
        chunk.end_offset = tile_end;
        chunk.source_hash = hash;
        chunk.commands = {std::move(cmd)};
        chunks.push_back(std::move(chunk));
    }

    return chunks;
}

auto find_first_dirty_chunk(const std::vector<TopLevelChunk>& old_chunks,
                            const std::vector<TopLevelChunk>& new_chunks) -> int32_t {
    auto limit = std::min(old_chunks.size(), new_chunks.size());
    for (std::size_t i = 0; i < limit; ++i) {
        if (old_chunks[i].source_hash != new_chunks[i].source_hash) {
            return static_cast<int32_t>(i);
        }
    }
    return static_cast<int32_t>(limit);
}

} // namespace tcl_lsp
