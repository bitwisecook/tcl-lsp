#include "tcl_lsp/parsing/lexer.hpp"

#include <algorithm>
#include <stdexcept>

namespace tcl_lsp {

TclLexer::TclLexer(TclLexer&& other) noexcept
    : owned_text_(std::move(other.owned_text_)),
      text_(owned_text_.empty() ? other.text_ : std::string_view(owned_text_)), len_(other.len_),
      pos_(other.pos_), config_(other.config_), base_offset_(other.base_offset_),
      base_line_(other.base_line_), base_col_(other.base_col_), line_(other.line_),
      col_(other.col_), start_(other.start_), end_(other.end_), type_(other.type_),
      at_command_start_(other.at_command_start_), insidequote_(other.insidequote_),
      warnings_(std::move(other.warnings_)), ghosts_(std::move(other.ghosts_)),
      has_ghosts_(other.has_ghosts_), pending_sep_(std::move(other.pending_sep_)),
      line_starts_(std::move(other.line_starts_)) {}

TclLexer& TclLexer::operator=(TclLexer&& other) noexcept {
    if (this != &other) {
        owned_text_ = std::move(other.owned_text_);
        text_ = owned_text_.empty() ? other.text_ : std::string_view(owned_text_);
        len_ = other.len_;
        pos_ = other.pos_;
        config_ = other.config_;
        base_offset_ = other.base_offset_;
        base_line_ = other.base_line_;
        base_col_ = other.base_col_;
        line_ = other.line_;
        col_ = other.col_;
        start_ = other.start_;
        end_ = other.end_;
        type_ = other.type_;
        at_command_start_ = other.at_command_start_;
        insidequote_ = other.insidequote_;
        warnings_ = std::move(other.warnings_);
        ghosts_ = std::move(other.ghosts_);
        has_ghosts_ = other.has_ghosts_;
        pending_sep_ = std::move(other.pending_sep_);
        line_starts_ = std::move(other.line_starts_);
    }
    return *this;
}

TclLexer::TclLexer(std::string text,
                   LexerConfig config,
                   int32_t base_offset,
                   int32_t base_line,
                   int32_t base_col,
                   std::unordered_map<int32_t, char> ghost_insertions,
                   const std::vector<int32_t>* shared_line_starts,
                   OwningTag)
    : owned_text_(std::move(text)), text_(owned_text_), len_(static_cast<int32_t>(text_.size())),
      config_(config), base_offset_(base_offset), base_line_(base_line), base_col_(base_col),
      line_(base_line), col_(base_col), ghosts_(std::move(ghost_insertions)),
      has_ghosts_(!ghosts_.empty()) {
    if (shared_line_starts != nullptr) {
        line_starts_ = *shared_line_starts;
    } else {
        line_starts_.push_back(0);
        for (int32_t i = 0; i < len_; ++i) {
            if (text_[static_cast<std::size_t>(i)] == '\n') {
                line_starts_.push_back(i + 1);
            }
        }
    }
}

TclLexer::TclLexer(std::string_view text,
                   LexerConfig config,
                   int32_t base_offset,
                   int32_t base_line,
                   int32_t base_col,
                   std::unordered_map<int32_t, char> ghost_insertions,
                   const std::vector<int32_t>* shared_line_starts)
    : text_(text), len_(static_cast<int32_t>(text.size())), config_(config),
      base_offset_(base_offset), base_line_(base_line), base_col_(base_col), line_(base_line),
      col_(base_col), ghosts_(std::move(ghost_insertions)), has_ghosts_(!ghosts_.empty()) {
    if (shared_line_starts != nullptr) {
        line_starts_ = *shared_line_starts;
    } else {
        line_starts_.push_back(0);
        for (int32_t i = 0; i < len_; ++i) {
            if (text_[static_cast<std::size_t>(i)] == '\n') {
                line_starts_.push_back(i + 1);
            }
        }
    }
}

auto TclLexer::remaining() const -> int32_t {
    auto r = len_ - pos_;
    if (r > 0)
        return r;
    if (has_ghosts_ && ghosts_.contains(pos_))
        return 1;
    return 0;
}

auto TclLexer::cur() const -> char {
    if (has_ghosts_) {
        auto it = ghosts_.find(pos_);
        if (it != ghosts_.end())
            return it->second;
    }
    return text_[static_cast<std::size_t>(pos_)];
}

void TclLexer::advance(int32_t n) {
    if (has_ghosts_) {
        for (int32_t i = 0; i < n; ++i) {
            auto it = ghosts_.find(pos_);
            if (it != ghosts_.end()) {
                ghosts_.erase(it);
                if (ghosts_.empty())
                    has_ghosts_ = false;
                continue;
            }
            if (pos_ < len_) {
                if (text_[static_cast<std::size_t>(pos_)] == '\n') {
                    line_ += 1;
                    col_ = 0;
                } else {
                    col_ += 1;
                }
                pos_ += 1;
            }
        }
    } else {
        // Fast path — no ghost insertions (>99% of instances).
        for (int32_t i = 0; i < n; ++i) {
            if (pos_ < len_) {
                if (text_[static_cast<std::size_t>(pos_)] == '\n') {
                    line_ += 1;
                    col_ = 0;
                } else {
                    col_ += 1;
                }
                pos_ += 1;
            }
        }
    }
}

auto TclLexer::position() const -> SourcePosition {
    return {line_, col_, pos_ + base_offset_};
}

auto TclLexer::pos_at(int32_t offset) const -> SourcePosition {
    auto it = std::upper_bound(line_starts_.begin(), line_starts_.end(), offset);
    auto line_idx = static_cast<int32_t>(std::distance(line_starts_.begin(), it)) - 1;
    auto col = offset - line_starts_[static_cast<std::size_t>(line_idx)];
    return {
        base_line_ + line_idx,
        (line_idx == 0) ? (base_col_ + col) : col,
        offset + base_offset_,
    };
}

auto TclLexer::token_text() const -> std::string {
    if (end_ < start_)
        return "";
    auto length = end_ - start_ + 1;
    return std::string(
        text_.substr(static_cast<std::size_t>(start_), static_cast<std::size_t>(length)));
}

void TclLexer::report_error(const std::string& message) {
    if (config_.strict_quoting) {
        throw TclParseError(message);
    }
    warnings_.emplace_back(position(), message);
}

void TclLexer::report_error_at(const SourcePosition& pos, const std::string& message) {
    if (config_.strict_quoting) {
        throw TclParseError(message);
    }
    warnings_.emplace_back(pos, message);
}

// Parse methods

void TclLexer::parse_sep() {
    start_ = pos_;
    if (!has_ghosts_) {
        auto pos = pos_;
        auto col = col_;
        while (pos < len_ && is_sep_char(text_[static_cast<std::size_t>(pos)])) {
            col += 1;
            pos += 1;
        }
        pos_ = pos;
        col_ = col;
    } else {
        while (remaining() && is_sep_char(cur())) {
            advance();
        }
    }
    end_ = pos_ - 1;
    type_ = TokenType::SEP;
}

void TclLexer::parse_eol() {
    start_ = pos_;
    if (!has_ghosts_) {
        auto pos = pos_;
        auto line = line_;
        auto col = col_;
        while (pos < len_ && is_separator_char(text_[static_cast<std::size_t>(pos)])) {
            if (text_[static_cast<std::size_t>(pos)] == '\n') {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            pos += 1;
        }
        pos_ = pos;
        line_ = line;
        col_ = col;
    } else {
        while (remaining() != 0 && is_separator_char(cur())) {
            advance();
        }
    }
    end_ = pos_ - 1;
    type_ = TokenType::EOL;
    at_command_start_ = true;
}

void TclLexer::parse_command() {
    int32_t level = 1;
    int32_t blevel = 0;
    bool in_quotes = false;
    advance(); // skip opening '['
    start_ = pos_;

    if (!has_ghosts_) {
        auto pos = pos_;
        auto line = line_;
        auto col = col_;

        while (pos < len_) {
            auto ch = text_[static_cast<std::size_t>(pos)];
            if (ch == '"' && blevel == 0) {
                in_quotes = !in_quotes;
                col += 1;
                pos += 1;
            } else if (ch == '[' && blevel == 0 && !in_quotes) {
                level += 1;
                col += 1;
                pos += 1;
            } else if (ch == ']' && blevel == 0 && !in_quotes) {
                level -= 1;
                if (level == 0)
                    break;
                col += 1;
                pos += 1;
            } else if (ch == '\\') {
                col += 1;
                pos += 1;
                if (pos < len_) {
                    if (text_[static_cast<std::size_t>(pos)] == '\n') {
                        line += 1;
                        col = 0;
                    } else {
                        col += 1;
                    }
                    pos += 1;
                }
            } else if (ch == '$' && !in_quotes && blevel == 0) {
                // ${...} inside command — scan for matching }.
                if (pos + 1 < len_ && text_[static_cast<std::size_t>(pos + 1)] == '{') {
                    col += 1;
                    pos += 1; // skip $
                    col += 1;
                    pos += 1; // skip {
                    while (pos < len_ && text_[static_cast<std::size_t>(pos)] != '}') {
                        if (text_[static_cast<std::size_t>(pos)] == '\n') {
                            line += 1;
                            col = 0;
                        } else {
                            col += 1;
                        }
                        pos += 1;
                    }
                    if (pos >= len_) {
                        pos_ = pos;
                        line_ = line;
                        col_ = col;
                        report_error("missing close-brace for variable name");
                    } else {
                        if (text_[static_cast<std::size_t>(pos)] == '\n') {
                            line += 1;
                            col = 0;
                        } else {
                            col += 1;
                        }
                        pos += 1;
                    }
                } else {
                    col += 1;
                    pos += 1;
                }
            } else if (ch == '{' && !in_quotes) {
                blevel += 1;
                col += 1;
                pos += 1;
            } else if (ch == '}' && !in_quotes) {
                if (blevel != 0)
                    blevel -= 1;
                col += 1;
                pos += 1;
            } else {
                if (ch == '\n') {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
                pos += 1;
            }
        }

        pos_ = pos;
        line_ = line;
        col_ = col;
    } else {
        // Slow path — ghost insertion support.
        while (true) {
            if (!remaining())
                break;
            auto ch = cur();
            if (ch == '"' && blevel == 0) {
                in_quotes = !in_quotes;
            } else if (ch == '[' && blevel == 0 && !in_quotes) {
                level += 1;
            } else if (ch == ']' && blevel == 0 && !in_quotes) {
                level -= 1;
                if (level == 0)
                    break;
            } else if (ch == '\\') {
                advance();
            } else if (ch == '$' && !in_quotes && blevel == 0) {
                if (pos_ + 1 < len_ && text_[static_cast<std::size_t>(pos_ + 1)] == '{') {
                    advance(); // skip $
                    advance(); // skip {
                    while (remaining() && cur() != '}') {
                        advance();
                    }
                    if (!remaining()) {
                        report_error("missing close-brace for variable name");
                    }
                }
            } else if (ch == '{' && !in_quotes) {
                blevel += 1;
            } else if (ch == '}' && !in_quotes) {
                if (blevel != 0)
                    blevel -= 1;
            }
            advance();
        }
    }

    end_ = pos_ - 1;
    type_ = TokenType::CMD;
    if (remaining() && cur() == ']') {
        advance();
    } else if (level > 0) {
        report_error("missing close-bracket");
    }
}

void TclLexer::parse_var() {
    auto dollar_pos = position();
    advance(); // skip '$'

    // Handle ${name} form.
    if (remaining() && cur() == '{') {
        advance(); // skip '{'
        start_ = pos_;
        while (remaining() && cur() != '}') {
            advance();
        }
        end_ = pos_ - 1;
        type_ = TokenType::VAR;
        if (remaining()) {
            advance(); // skip '}'
        } else {
            report_error_at(dollar_pos, "missing close-brace for variable name");
        }
        return;
    }

    start_ = pos_;
    // Accept alnum, underscore, :: (namespace separator).
    while (remaining()) {
        auto ch = cur();
        if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') ||
            ch == '_') {
            advance();
        } else if (ch == ':' && pos_ + 1 < len_ &&
                   text_[static_cast<std::size_t>(pos_ + 1)] == ':') {
            advance(2); // skip '::'
        } else {
            break;
        }
    }

    // Handle array indexing $arr(idx).
    if (remaining() && cur() == '(') {
        int32_t level = 1;
        advance();
        while (remaining() && level > 0) {
            auto ch = cur();
            if (ch == '(') {
                level += 1;
            } else if (ch == ')') {
                level -= 1;
            } else if (ch == '$' && pos_ + 1 < len_ &&
                       text_[static_cast<std::size_t>(pos_ + 1)] == '{') {
                // ${...} inside array index — scan for matching }.
                advance(); // skip $
                advance(); // skip {
                while (remaining() && cur() != '}') {
                    advance();
                }
                if (!remaining()) {
                    report_error("missing close-brace for variable name");
                }
                // Found } — will be advanced at end of loop.
            }
            if (level > 0) {
                advance();
            }
        }
        if (remaining()) {
            advance(); // skip closing ')'
        } else {
            report_error("missing )");
        }
        end_ = pos_ - 1;
        type_ = TokenType::VAR;
        return;
    }

    if (start_ == pos_) {
        // bare '$'
        start_ = pos_ - 1;
        end_ = pos_ - 1;
        type_ = TokenType::STR;
    } else {
        end_ = pos_ - 1;
        type_ = TokenType::VAR;
    }
}

void TclLexer::parse_brace() {
    int32_t level = 1;
    advance(); // skip opening '{'
    start_ = pos_;

    if (!has_ghosts_) {
        auto pos = pos_;
        auto line = line_;
        auto col = col_;

        while (pos < len_) {
            auto ch = text_[static_cast<std::size_t>(pos)];
            if (ch == '\\') {
                col += 1;
                pos += 1;
                if (pos < len_) {
                    if (text_[static_cast<std::size_t>(pos)] == '\n') {
                        line += 1;
                        col = 0;
                    } else {
                        col += 1;
                    }
                    pos += 1;
                }
                continue;
            }
            if (ch == '}') {
                level -= 1;
                if (level == 0) {
                    end_ = pos - 1;
                    col += 1;
                    pos += 1;
                    pos_ = pos;
                    line_ = line;
                    col_ = col;
                    if (pos < len_ && !is_after_close_brace(text_[static_cast<std::size_t>(pos)])) {
                        if (text_[static_cast<std::size_t>(pos)] == '\\' && pos + 1 < len_ &&
                            (text_[static_cast<std::size_t>(pos + 1)] == '\n' ||
                             text_[static_cast<std::size_t>(pos + 1)] == '\r')) {
                            // backslash-newline is a valid line continuation
                        } else if (config_.irules_brace_separator &&
                                   text_[static_cast<std::size_t>(pos)] == '{') {
                            auto sep_pos = position();
                            pending_sep_ = Token{TokenType::SEP, "", sep_pos, sep_pos, false};
                        } else {
                            report_error("extra characters after close-brace");
                        }
                    }
                    type_ = TokenType::STR;
                    return;
                }
            } else if (ch == '{') {
                level += 1;
            }
            if (ch == '\n') {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            pos += 1;
        }

        // EOF without matching close-brace.
        pos_ = pos;
        line_ = line;
        col_ = col;
        end_ = pos - 1;
        report_error("missing close-brace");
        type_ = TokenType::STR;
        return;
    }

    // Slow path — ghost insertion support.
    while (true) {
        if (!remaining()) {
            end_ = pos_ - 1;
            report_error("missing close-brace");
            type_ = TokenType::STR;
            return;
        }
        auto ch = cur();
        if (ch == '\\' && remaining() >= 2) {
            advance();
        } else if (ch == '}') {
            level -= 1;
            if (level == 0) {
                end_ = pos_ - 1;
                advance(); // skip closing '}'
                if (remaining() && !is_after_close_brace(cur())) {
                    if (cur() == '\\' && remaining() >= 2 &&
                        (text_[static_cast<std::size_t>(pos_ + 1)] == '\n' ||
                         text_[static_cast<std::size_t>(pos_ + 1)] == '\r')) {
                        // backslash-newline is a valid line continuation
                    } else if (config_.irules_brace_separator && cur() == '{') {
                        auto sep_pos = position();
                        pending_sep_ = Token{TokenType::SEP, "", sep_pos, sep_pos, false};
                    } else {
                        report_error("extra characters after close-brace");
                    }
                }
                type_ = TokenType::STR;
                return;
            }
        } else if (ch == '{') {
            level += 1;
        }
        advance();
    }
}

void TclLexer::parse_expand() {
    start_ = pos_;
    advance(3);    // skip '{*}'
    end_ = start_; // zero-width
    type_ = TokenType::EXPAND;
}

void TclLexer::parse_string() {
    const bool newword = (type_ == TokenType::SEP || type_ == TokenType::EOL ||
                          type_ == TokenType::STR || type_ == TokenType::EXPAND);
    if (newword && remaining() && cur() == '{') {
        // Check for {*} expansion prefix (Tcl 8.5+).
        if (config_.expand_syntax && pos_ + 2 < len_ &&
            text_[static_cast<std::size_t>(pos_ + 1)] == '*' &&
            text_[static_cast<std::size_t>(pos_ + 2)] == '}' && pos_ + 3 < len_ &&
            !is_separator_char(text_[static_cast<std::size_t>(pos_ + 3)])) {
            parse_expand();
            return;
        }
        parse_brace();
        return;
    }
    if (newword && remaining() && cur() == '"') {
        insidequote_ = true;
        advance();
    }
    start_ = pos_;

    if (!has_ghosts_) {
        auto pos = pos_;
        auto line = line_;
        auto col = col_;
        auto insidequote = insidequote_;

        while (pos < len_) {
            auto ch = text_[static_cast<std::size_t>(pos)];
            if (ch == '\\') {
                if (pos + 1 < len_) {
                    col += 1;
                    pos += 1;
                    if (text_[static_cast<std::size_t>(pos)] == '\n') {
                        line += 1;
                        col = 0;
                    } else {
                        col += 1;
                    }
                    pos += 1;
                    continue;
                }
                // Backslash at EOF — fall through.
            } else if (ch == '$' || ch == '[') {
                end_ = pos - 1;
                type_ = TokenType::ESC;
                pos_ = pos;
                line_ = line;
                col_ = col;
                return;
            } else if (!insidequote && is_separator_char(ch)) {
                end_ = pos - 1;
                type_ = TokenType::ESC;
                pos_ = pos;
                line_ = line;
                col_ = col;
                return;
            } else if (ch == '"' && insidequote) {
                end_ = pos - 1;
                type_ = TokenType::ESC;
                col += 1;
                pos += 1;
                insidequote_ = false;
                pos_ = pos;
                line_ = line;
                col_ = col;
                if (pos < len_ && !is_after_close_quote(text_[static_cast<std::size_t>(pos)])) {
                    if (text_[static_cast<std::size_t>(pos)] == '\\' && pos + 1 < len_ &&
                        (text_[static_cast<std::size_t>(pos + 1)] == '\n' ||
                         text_[static_cast<std::size_t>(pos + 1)] == '\r')) {
                        // backslash-newline is a valid line continuation
                    } else {
                        report_error("extra characters after close-quote");
                    }
                }
                return;
            }
            if (ch == '\n') {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            pos += 1;
        }

        // EOF
        pos_ = pos;
        line_ = line;
        col_ = col;
        if (insidequote) {
            report_error("missing \"");
        }
        end_ = pos - 1;
        type_ = TokenType::ESC;
        return;
    }

    // Slow path — ghost insertion support.
    while (true) {
        if (!remaining()) {
            if (insidequote_) {
                report_error("missing \"");
            }
            end_ = pos_ - 1;
            type_ = TokenType::ESC;
            return;
        }
        auto ch = cur();
        if (ch == '\\') {
            if (remaining() >= 2) {
                advance();
            }
        } else if (ch == '$' || ch == '[') {
            end_ = pos_ - 1;
            type_ = TokenType::ESC;
            return;
        } else if (is_separator_char(ch)) {
            if (!insidequote_) {
                end_ = pos_ - 1;
                type_ = TokenType::ESC;
                return;
            }
        } else if (ch == '"') {
            if (insidequote_) {
                end_ = pos_ - 1;
                type_ = TokenType::ESC;
                advance();
                insidequote_ = false;
                if (remaining() && !is_after_close_quote(cur())) {
                    if (cur() == '\\' && remaining() >= 2 &&
                        (text_[static_cast<std::size_t>(pos_ + 1)] == '\n' ||
                         text_[static_cast<std::size_t>(pos_ + 1)] == '\r')) {
                        // backslash-newline is a valid line continuation
                    } else {
                        report_error("extra characters after close-quote");
                    }
                }
                return;
            }
        }
        advance();
    }
}

void TclLexer::parse_comment() {
    start_ = pos_; // include the '#'

    if (!has_ghosts_) {
        auto pos = pos_;
        auto line = line_;
        auto col = col_;

        while (pos < len_) {
            auto ch = text_[static_cast<std::size_t>(pos)];
            if (ch == '\\') {
                auto next_pos = pos + 1;
                if (next_pos < len_) {
                    auto next_ch = text_[static_cast<std::size_t>(next_pos)];
                    if (next_ch == '\n') {
                        // Backslash-newline continuation.
                        col += 1;
                        pos += 1;
                        line += 1;
                        col = 0;
                        pos += 1;
                        continue;
                    }
                    col += 1;
                    pos += 1;
                    if (text_[static_cast<std::size_t>(pos)] == '\n') {
                        line += 1;
                        col = 0;
                    } else {
                        col += 1;
                    }
                    pos += 1;
                    continue;
                }
            }
            if (ch == '\n')
                break;
            col += 1;
            pos += 1;
        }

        pos_ = pos;
        line_ = line;
        col_ = col;
    } else {
        while (remaining()) {
            auto ch = cur();
            if (ch == '\\') {
                auto next_pos = pos_ + 1;
                if (next_pos < len_) {
                    auto next_ch = text_[static_cast<std::size_t>(next_pos)];
                    if (next_ch == '\n') {
                        advance(); // skip backslash
                        advance(); // skip newline
                        continue;
                    }
                    advance(); // skip backslash
                    advance(); // skip escaped char
                    continue;
                }
            }
            if (ch == '\n')
                break;
            advance();
        }
    }

    end_ = pos_ - 1;
    type_ = TokenType::COMMENT;
}

auto TclLexer::get_token() -> std::optional<Token> {
    // Drain any synthetic SEP queued by parse_brace (iRules }{ boundary).
    if (pending_sep_.has_value()) {
        auto sep = std::move(*pending_sep_);
        pending_sep_.reset();
        type_ = TokenType::SEP;
        return sep;
    }

    while (true) {
        if (pos_ >= len_ && !(has_ghosts_ && ghosts_.contains(pos_))) {
            if (type_ != TokenType::EOL && type_ != TokenType::EOF_) {
                type_ = TokenType::EOL;
            } else {
                type_ = TokenType::EOF_;
                return std::nullopt;
            }
            auto p = SourcePosition{line_, col_, pos_ + base_offset_};
            return Token{TokenType::EOL, "", p, p, false};
        }

        auto start_pos = SourcePosition{line_, col_, pos_ + base_offset_};

        // Get current character for dispatch.
        char ch = '\0';
        if (has_ghosts_) {
            auto it = ghosts_.find(pos_);
            ch = (it != ghosts_.end()) ? it->second : text_[static_cast<std::size_t>(pos_)];
        } else {
            ch = text_[static_cast<std::size_t>(pos_)];
        }

        // Dispatch.
        if (ch == '[') {
            parse_command();
        } else if (ch == '$') {
            parse_var();
        } else if (!insidequote_) {
            if (is_sep_char(ch)) {
                parse_sep();
            } else if (is_eol_char(ch)) {
                parse_eol();
            } else if (ch == '#' && at_command_start_) {
                parse_comment();
            } else {
                parse_string();
            }
        } else {
            parse_string();
        }

        // Build the token.
        auto tok_text = token_text();
        auto end_offset = (end_ >= start_) ? end_ : start_;
        auto end_pos = pos_at(end_offset);

        // Track command-start position for comment detection.
        if (type_ != TokenType::SEP && type_ != TokenType::EOL && type_ != TokenType::COMMENT) {
            at_command_start_ = false;
        }

        return Token{type_, std::move(tok_text), start_pos, end_pos, insidequote_};
    }
}

auto TclLexer::tokenise_all() -> std::vector<Token> {
    std::vector<Token> tokens;
    while (auto tok = get_token()) {
        tokens.push_back(std::move(*tok));
    }
    return tokens;
}

} // namespace tcl_lsp
