#include "tcl_lsp/analysis/param_list_parser.hpp"

#include <cctype>

namespace tcl_lsp {

namespace {

auto is_ws(char c) -> bool { return c == ' ' || c == '\t' || c == '\n' || c == '\r'; }

void skip_whitespace(std::string_view text, std::size_t& i) {
    while (i < text.size() && is_ws(text[i])) {
        ++i;
    }
}

} // namespace

auto parse_param_list(std::string_view param_str) -> std::vector<ParamDef> {
    std::vector<ParamDef> params;

    // Trim leading/trailing whitespace.
    std::size_t start = 0;
    std::size_t end = param_str.size();
    while (start < end && is_ws(param_str[start])) {
        ++start;
    }
    while (end > start && is_ws(param_str[end - 1])) {
        --end;
    }
    auto text = param_str.substr(start, end - start);

    std::size_t i = 0;
    while (i < text.size()) {
        skip_whitespace(text, i);
        if (i >= text.size()) {
            break;
        }

        if (text[i] == '{') {
            // Braced parameter with possible default.
            int level = 1;
            ++i;
            auto inner_start = i;
            while (i < text.size() && level > 0) {
                if (text[i] == '{') {
                    ++level;
                } else if (text[i] == '}') {
                    --level;
                }
                ++i;
            }
            auto inner = text.substr(inner_start, (i > 0 ? i - 1 : 0) - inner_start);

            // Trim inner whitespace.
            std::size_t is = 0;
            std::size_t ie = inner.size();
            while (is < ie && is_ws(inner[is])) {
                ++is;
            }
            while (ie > is && is_ws(inner[ie - 1])) {
                --ie;
            }
            inner = inner.substr(is, ie - is);

            // Split on first whitespace: name [default].
            auto ws_pos = inner.find_first_of(" \t\n\r");
            if (ws_pos != std::string_view::npos) {
                auto name = inner.substr(0, ws_pos);
                auto rest = inner.substr(ws_pos);
                // Trim leading whitespace from default value.
                std::size_t ds = 0;
                while (ds < rest.size() && is_ws(rest[ds])) {
                    ++ds;
                }
                params.push_back({std::string(name), std::string(rest.substr(ds))});
            } else if (!inner.empty()) {
                params.push_back({std::string(inner), std::nullopt});
            }
        } else {
            // Bare word.
            auto word_start = i;
            while (i < text.size() && !is_ws(text[i])) {
                ++i;
            }
            auto word = text.substr(word_start, i - word_start);
            if (!word.empty()) {
                params.push_back({std::string(word), std::nullopt});
            }
        }
    }

    return params;
}

} // namespace tcl_lsp
