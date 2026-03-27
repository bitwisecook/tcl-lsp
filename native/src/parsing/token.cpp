#include "tcl_lsp/parsing/token.hpp"

namespace tcl_lsp {

auto to_string(TokenType type) -> std::string {
    switch (type) {
        case TokenType::ESC:     return "ESC";
        case TokenType::STR:     return "STR";
        case TokenType::CMD:     return "CMD";
        case TokenType::VAR:     return "VAR";
        case TokenType::SEP:     return "SEP";
        case TokenType::EOL:     return "EOL";
        case TokenType::EOF_:    return "EOF";
        case TokenType::COMMENT: return "COMMENT";
        case TokenType::EXPAND:  return "EXPAND";
    }
    return "UNKNOWN";
}

auto to_string(const Token& t) -> std::string {
    return "Token(" + to_string(t.type) + ", \"" + t.text + "\")";
}

}  // namespace tcl_lsp
