#include "tcl_lsp/parsing/expr_lexer.hpp"

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdint>
#include <string_view>
#include <vector>

namespace tcl_lsp {
namespace {

// Known Tcl math functions.
constexpr std::array MATH_FUNCTIONS = {
    std::string_view{"abs"},    std::string_view{"acos"},   std::string_view{"asin"},
    std::string_view{"atan"},   std::string_view{"atan2"},  std::string_view{"bool"},
    std::string_view{"ceil"},   std::string_view{"cos"},    std::string_view{"cosh"},
    std::string_view{"double"}, std::string_view{"entier"}, std::string_view{"exp"},
    std::string_view{"floor"},  std::string_view{"fmod"},   std::string_view{"hypot"},
    std::string_view{"int"},    std::string_view{"isinf"},  std::string_view{"isnan"},
    std::string_view{"isqrt"},  std::string_view{"log"},    std::string_view{"log10"},
    std::string_view{"max"},    std::string_view{"min"},    std::string_view{"pow"},
    std::string_view{"rand"},   std::string_view{"round"},  std::string_view{"sin"},
    std::string_view{"sinh"},   std::string_view{"sqrt"},   std::string_view{"srand"},
    std::string_view{"tan"},    std::string_view{"tanh"},   std::string_view{"wide"},
};

auto is_math_function(std::string_view name) -> bool {
    return std::find(MATH_FUNCTIONS.begin(), MATH_FUNCTIONS.end(), name) != MATH_FUNCTIONS.end();
}

// Multi-character operators (longest-first for greedy matching).
struct MultiOp {
    std::string_view text;
    bool word_boundary; // requires word boundary checks (eq, ne, etc.)
};

constexpr std::array MULTI_OPS = {
    MultiOp{"**", false},
    MultiOp{"<<", false},
    MultiOp{">>", false},
    MultiOp{"<=", false},
    MultiOp{">=", false},
    MultiOp{"==", false},
    MultiOp{"!=", false},
    MultiOp{"&&", false},
    MultiOp{"||", false},
    MultiOp{"ne", true},
    MultiOp{"eq", true},
    MultiOp{"in", true},
    MultiOp{"ni", true},
    MultiOp{"lt", true},
    MultiOp{"le", true},
    MultiOp{"gt", true},
    MultiOp{"ge", true},
};

auto is_single_op(char ch) -> bool {
    return ch == '+' || ch == '-' || ch == '*' || ch == '/' || ch == '%' || ch == '<' ||
           ch == '>' || ch == '&' || ch == '|' || ch == '^' || ch == '~' || ch == '!';
}

auto is_bool_literal(std::string_view text) -> bool {
    return text == "true" || text == "false" || text == "yes" || text == "no" || text == "on" ||
           text == "off";
}

auto is_ws(char ch) -> bool {
    return ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r';
}

auto is_id_char(char ch) -> bool {
    return std::isalnum(static_cast<unsigned char>(ch)) != 0 || ch == '_';
}

class ExprLexer {
  public:
    explicit ExprLexer(std::string_view source) : src_(source) {}

    auto tokenise() -> std::vector<ExprToken> {
        std::vector<ExprToken> tokens;
        while (pos_ < len()) {
            char ch = cur();

            if (is_ws(ch)) {
                read_whitespace(tokens);
                continue;
            }
            if (std::isdigit(static_cast<unsigned char>(ch)) != 0 ||
                (ch == '.' && pos_ + 1 < len() &&
                 std::isdigit(static_cast<unsigned char>(src_[pos_ + 1])) != 0)) {
                read_number(tokens);
                continue;
            }
            if (ch == '$') {
                read_variable(tokens);
                continue;
            }
            if (ch == '[') {
                read_command(tokens);
                continue;
            }
            if (ch == '"') {
                read_string(tokens);
                continue;
            }
            if (ch == '(') {
                tokens.push_back({ExprTokenType::PAREN_OPEN, src_.substr(pos_, 1), ipos(), ipos()});
                pos_ += 1;
                continue;
            }
            if (ch == ')') {
                tokens.push_back(
                    {ExprTokenType::PAREN_CLOSE, src_.substr(pos_, 1), ipos(), ipos()});
                pos_ += 1;
                continue;
            }
            if (ch == ',') {
                tokens.push_back({ExprTokenType::COMMA, src_.substr(pos_, 1), ipos(), ipos()});
                pos_ += 1;
                continue;
            }
            if (ch == '?') {
                tokens.push_back({ExprTokenType::TERNARY_Q, src_.substr(pos_, 1), ipos(), ipos()});
                pos_ += 1;
                continue;
            }
            if (ch == ':') {
                tokens.push_back({ExprTokenType::TERNARY_C, src_.substr(pos_, 1), ipos(), ipos()});
                pos_ += 1;
                continue;
            }
            if (try_multi_op(tokens))
                continue;
            if (is_single_op(ch)) {
                tokens.push_back({ExprTokenType::OPERATOR, src_.substr(pos_, 1), ipos(), ipos()});
                pos_ += 1;
                continue;
            }
            if (std::isalpha(static_cast<unsigned char>(ch)) != 0 || ch == '_') {
                read_identifier(tokens);
                continue;
            }
            if (ch == '{') {
                read_braced(tokens);
                continue;
            }
            // Unknown character — skip.
            pos_ += 1;
        }
        return tokens;
    }

  private:
    [[nodiscard]] auto len() const -> std::size_t { return src_.size(); }

    [[nodiscard]] auto cur() const -> char { return src_[pos_]; }

    [[nodiscard]] auto ipos() const -> std::int32_t { return static_cast<std::int32_t>(pos_); }

    void read_whitespace(std::vector<ExprToken>& out) {
        auto start = pos_;
        while (pos_ < len() && is_ws(src_[pos_]))
            pos_ += 1;
        out.push_back({ExprTokenType::WHITESPACE,
                       src_.substr(start, pos_ - start),
                       static_cast<std::int32_t>(start),
                       ipos() - 1});
    }

    void read_number(std::vector<ExprToken>& out) {
        auto start = pos_;
        // 0x, 0o, 0b prefixes
        if (src_[pos_] == '0' && pos_ + 1 < len() &&
            (src_[pos_ + 1] == 'x' || src_[pos_ + 1] == 'X' || src_[pos_ + 1] == 'o' ||
             src_[pos_ + 1] == 'O' || src_[pos_ + 1] == 'b' || src_[pos_ + 1] == 'B')) {
            pos_ += 2;
            while (pos_ < len() && is_id_char(src_[pos_]))
                pos_ += 1;
            out.push_back({ExprTokenType::NUMBER,
                           src_.substr(start, pos_ - start),
                           static_cast<std::int32_t>(start),
                           ipos() - 1});
            return;
        }
        // Integer part
        while (pos_ < len() && std::isdigit(static_cast<unsigned char>(src_[pos_])) != 0)
            pos_ += 1;
        // Fractional part
        if (pos_ < len() && src_[pos_] == '.') {
            pos_ += 1;
            while (pos_ < len() && std::isdigit(static_cast<unsigned char>(src_[pos_])) != 0)
                pos_ += 1;
        }
        // Scientific notation — only if followed by digit (or sign+digit).
        if (pos_ < len() && (src_[pos_] == 'e' || src_[pos_] == 'E')) {
            auto save = pos_;
            pos_ += 1;
            if (pos_ < len() && (src_[pos_] == '+' || src_[pos_] == '-'))
                pos_ += 1;
            if (pos_ < len() && std::isdigit(static_cast<unsigned char>(src_[pos_])) != 0) {
                while (pos_ < len() && std::isdigit(static_cast<unsigned char>(src_[pos_])) != 0)
                    pos_ += 1;
            } else {
                pos_ = save;
            }
        }
        out.push_back({ExprTokenType::NUMBER,
                       src_.substr(start, pos_ - start),
                       static_cast<std::int32_t>(start),
                       ipos() - 1});
    }

    void read_variable(std::vector<ExprToken>& out) {
        auto start = pos_;
        pos_ += 1; // skip $
        if (pos_ < len() && src_[pos_] == '{') {
            pos_ += 1;
            while (pos_ < len() && src_[pos_] != '}')
                pos_ += 1;
            if (pos_ < len())
                pos_ += 1;
        } else {
            while (pos_ < len() && (std::isalnum(static_cast<unsigned char>(src_[pos_])) != 0 ||
                                    src_[pos_] == '_' || src_[pos_] == ':'))
                pos_ += 1;
            if (pos_ < len() && src_[pos_] == '(') {
                pos_ += 1;
                int level = 1;
                while (pos_ < len() && level > 0) {
                    if (src_[pos_] == '(')
                        level += 1;
                    else if (src_[pos_] == ')')
                        level -= 1;
                    pos_ += 1;
                }
            }
        }
        out.push_back({ExprTokenType::VARIABLE,
                       src_.substr(start, pos_ - start),
                       static_cast<std::int32_t>(start),
                       ipos() - 1});
    }

    void read_command(std::vector<ExprToken>& out) {
        auto start = pos_;
        pos_ += 1; // skip [
        int level = 1;
        while (pos_ < len() && level > 0) {
            if (src_[pos_] == '[')
                level += 1;
            else if (src_[pos_] == ']')
                level -= 1;
            else if (src_[pos_] == '\\')
                pos_ += 1; // skip escaped char
            pos_ += 1;
        }
        out.push_back({ExprTokenType::COMMAND,
                       src_.substr(start, pos_ - start),
                       static_cast<std::int32_t>(start),
                       ipos() - 1});
    }

    void read_string(std::vector<ExprToken>& out) {
        auto start = pos_;
        pos_ += 1; // skip opening quote
        while (pos_ < len() && src_[pos_] != '"') {
            if (src_[pos_] == '\\')
                pos_ += 1;
            pos_ += 1;
        }
        if (pos_ < len())
            pos_ += 1; // skip closing quote
        out.push_back({ExprTokenType::STRING,
                       src_.substr(start, pos_ - start),
                       static_cast<std::int32_t>(start),
                       ipos() - 1});
    }

    void read_identifier(std::vector<ExprToken>& out) {
        auto start = pos_;
        while (pos_ < len() && is_id_char(src_[pos_]))
            pos_ += 1;
        auto text = src_.substr(start, pos_ - start);
        auto s = static_cast<std::int32_t>(start);
        auto e = ipos() - 1;

        if (is_bool_literal(text)) {
            out.push_back({ExprTokenType::BOOL, text, s, e});
        } else if (is_math_function(text)) {
            out.push_back({ExprTokenType::FUNCTION, text, s, e});
        } else {
            // Unknown identifier — treat as function name.
            out.push_back({ExprTokenType::FUNCTION, text, s, e});
        }
    }

    void read_braced(std::vector<ExprToken>& out) {
        auto start = pos_;
        pos_ += 1; // skip {
        int level = 1;
        auto saved = pos_;
        while (pos_ < len() && level > 0) {
            if (src_[pos_] == '{')
                level += 1;
            else if (src_[pos_] == '}')
                level -= 1;
            pos_ += 1;
        }
        if (level != 0) {
            pos_ = saved;
            out.push_back({ExprTokenType::STRING,
                           src_.substr(start, 1),
                           static_cast<std::int32_t>(start),
                           static_cast<std::int32_t>(start)});
            return;
        }
        out.push_back({ExprTokenType::STRING,
                       src_.substr(start, pos_ - start),
                       static_cast<std::int32_t>(start),
                       ipos() - 1});
    }

    auto try_multi_op(std::vector<ExprToken>& out) -> bool {
        for (const auto& [text, word_boundary] : MULTI_OPS) {
            if (pos_ + text.size() > len())
                continue;
            if (src_.substr(pos_, text.size()) != text)
                continue;
            if (word_boundary) {
                // Check preceding character — alpha/underscore means mid-identifier.
                if (pos_ > 0) {
                    char prev = src_[pos_ - 1];
                    if (std::isalpha(static_cast<unsigned char>(prev)) != 0 || prev == '_')
                        continue;
                }
                auto after = pos_ + text.size();
                if (after < len() && (std::isalpha(static_cast<unsigned char>(src_[after])) != 0 ||
                                      src_[after] == '_'))
                    continue;
            }
            auto s = ipos();
            out.push_back({ExprTokenType::OPERATOR,
                           src_.substr(pos_, text.size()),
                           s,
                           static_cast<std::int32_t>(pos_ + text.size() - 1)});
            pos_ += text.size();
            return true;
        }
        return false;
    }

    std::string_view src_;
    std::size_t pos_ = 0;
};

} // anonymous namespace

auto tokenise_expr(std::string_view source) -> std::vector<ExprToken> {
    return ExprLexer(source).tokenise();
}

} // namespace tcl_lsp
