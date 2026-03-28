#include "tcl_lsp/analysis/stub_parser.hpp"
#include "tcl_lsp/core/source_position.hpp"

#include <algorithm>
#include <cctype>
#include <charconv>
#include <expected>
#include <string>
#include <unordered_set>
#include <vector>

namespace tcl_lsp {

namespace {

const std::unordered_set<std::string> VALID_ROLES = {
    "body", "expr", "var", "var_read", "name", "pattern", "channel", "value"};

const std::unordered_set<std::string> VALID_FLAGS = {
    "-barrier", "-loop", "-pure", "-mutator", "-unsafe", "-scope_alias"};

auto to_lower(std::string_view sv) -> std::string {
    std::string result(sv);
    std::transform(result.begin(), result.end(), result.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return result;
}

auto strip_comment_prefix(std::string_view line) -> std::string_view {
    // Strip leading #, spaces, tabs.
    auto pos = line.find_first_not_of("# \t");
    if (pos == std::string_view::npos) {
        return {};
    }
    auto text = line.substr(pos);

    // Strip optional "tcl-lsp:" prefix (case-insensitive).
    auto lower = to_lower(text.substr(0, 8));
    if (lower.starts_with("tcl-lsp:")) {
        text = text.substr(8);
        pos = text.find_first_not_of(" \t");
        if (pos == std::string_view::npos) {
            return {};
        }
        text = text.substr(pos);
    }
    return text;
}

// Split a string_view on whitespace, returning a vector of tokens.
auto split_whitespace(std::string_view sv) -> std::vector<std::string_view> {
    std::vector<std::string_view> tokens;
    std::size_t i = 0;
    while (i < sv.size()) {
        while (i < sv.size() && std::isspace(static_cast<unsigned char>(sv[i]))) {
            ++i;
        }
        if (i >= sv.size()) {
            break;
        }
        auto start = i;
        while (i < sv.size() && !std::isspace(static_cast<unsigned char>(sv[i]))) {
            ++i;
        }
        tokens.push_back(sv.substr(start, i - start));
    }
    return tokens;
}

auto parse_stub_args(std::string_view args_str) -> std::optional<std::vector<StubArgDef>> {
    if (args_str.empty()) {
        return std::vector<StubArgDef>{};
    }

    std::vector<StubArgDef> result;
    for (auto token : split_whitespace(args_str)) {
        bool optional = false;
        std::string name_str(token);

        // Handle optional args: ?argName? or ?argName:role?
        if (name_str.starts_with('?') && name_str.ends_with('?') && name_str.size() >= 2) {
            optional = true;
            name_str = name_str.substr(1, name_str.size() - 2);
        }

        // Handle role annotation: argName:role
        std::string role = "value";
        auto colon_pos = name_str.find(':');
        if (colon_pos != std::string::npos) {
            auto role_str = to_lower(name_str.substr(colon_pos + 1));
            if (!VALID_ROLES.contains(role_str)) {
                return std::nullopt;
            }
            role = role_str;
            name_str = name_str.substr(0, colon_pos);
        }

        if (name_str.empty()) {
            return std::nullopt;
        }

        result.push_back({std::move(name_str), std::move(role), optional});
    }
    return result;
}

auto parse_stub_flags(std::string_view flags_str) -> std::unordered_set<std::string> {
    std::unordered_set<std::string> flags;
    for (auto token : split_whitespace(flags_str)) {
        std::string tok(token);
        if (VALID_FLAGS.contains(tok)) {
            flags.insert(std::move(tok));
        }
    }
    return flags;
}

auto ci_contains(std::string_view haystack, std::string_view needle) -> bool {
    if (needle.size() > haystack.size()) {
        return false;
    }
    auto lower = to_lower(haystack);
    return lower.find(needle) != std::string::npos;
}

} // namespace

auto is_stubs_begin(std::string_view comment) -> bool {
    return ci_contains(comment, "tcl-lsp:") && ci_contains(comment, "stubs-begin");
}

auto is_stubs_end(std::string_view comment) -> bool {
    return ci_contains(comment, "tcl-lsp:") && ci_contains(comment, "stubs-end");
}

auto parse_stub_line(std::string_view line, Range line_range)
    -> std::expected<StubCommandDef, StubParseError> {
    auto text = strip_comment_prefix(line);
    auto lower = to_lower(text);

    // Must start with "stub " (case-insensitive).
    if (!lower.starts_with("stub ")) {
        return std::unexpected(StubParseError::NOT_A_STUB);
    }
    // Skip expr stubs.
    if (lower.starts_with("stub expr-")) {
        return std::unexpected(StubParseError::NOT_A_STUB);
    }

    text = text.substr(5); // skip "stub "
    // Skip whitespace after "stub".
    auto pos = text.find_first_not_of(" \t");
    if (pos == std::string_view::npos) {
        return std::unexpected(StubParseError::MISSING_NAME);
    }
    text = text.substr(pos);

    // Extract command name (first non-whitespace token).
    auto name_end = text.find_first_of(" \t{");
    if (name_end == std::string_view::npos) {
        return std::unexpected(StubParseError::MISSING_BRACES);
    }
    auto cmd_name = std::string(text.substr(0, name_end));

    // Find args in braces.
    auto brace_start = text.find('{', name_end);
    if (brace_start == std::string_view::npos) {
        return std::unexpected(StubParseError::MISSING_BRACES);
    }
    auto brace_end = text.find('}', brace_start);
    if (brace_end == std::string_view::npos) {
        return std::unexpected(StubParseError::MISSING_BRACES);
    }

    auto args_str = text.substr(brace_start + 1, brace_end - brace_start - 1);
    auto args = parse_stub_args(args_str);
    if (!args.has_value()) {
        return std::unexpected(StubParseError::INVALID_ARG_SYNTAX);
    }

    auto flags_str = text.substr(brace_end + 1);
    auto flags = parse_stub_flags(flags_str);

    return StubCommandDef{
        std::move(cmd_name),
        std::move(*args),
        line_range,
        flags.contains("-barrier"),
        flags.contains("-loop"),
        flags.contains("-pure"),
        flags.contains("-mutator"),
        flags.contains("-unsafe"),
        flags.contains("-scope_alias"),
    };
}

auto parse_expr_stub_line(std::string_view line, Range line_range)
    -> std::expected<StubExprDef, StubParseError> {
    auto text = strip_comment_prefix(line);
    auto lower = to_lower(text);

    // Must match "stub expr-func <name> ?arity?" or "stub expr-op <name> ?arity?".
    if (!lower.starts_with("stub expr-")) {
        return std::unexpected(StubParseError::NOT_A_STUB);
    }

    auto tokens = split_whitespace(text);
    // tokens[0] = "stub", tokens[1] = "expr-func" or "expr-op", tokens[2] = name, tokens[3] = arity
    if (tokens.size() < 3) {
        return std::unexpected(StubParseError::MISSING_NAME);
    }

    auto kind_lower = to_lower(tokens[1]);
    std::string kind;
    if (kind_lower == "expr-func") {
        kind = "function";
    } else if (kind_lower == "expr-op") {
        kind = "operator";
    } else {
        return std::unexpected(StubParseError::INVALID_EXPR_KIND);
    }

    auto name = std::string(tokens[2]);
    int32_t arity = (kind == "function") ? 1 : 2; // defaults

    if (tokens.size() >= 4) {
        int32_t parsed = 0;
        auto [ptr, ec] = std::from_chars(tokens[3].data(), tokens[3].data() + tokens[3].size(),
                                         parsed);
        if (ec == std::errc{}) {
            arity = parsed;
        }
    }

    return StubExprDef{std::move(name), std::move(kind), arity, true, line_range};
}

auto scan_source_for_stubs(std::string_view source) -> StubScanResult {
    StubScanResult result;
    bool in_block = false;
    int32_t lineno = 0;

    std::size_t pos = 0;
    while (pos <= source.size()) {
        // Find end of line.
        auto eol = source.find('\n', pos);
        auto line = (eol != std::string_view::npos) ? source.substr(pos, eol - pos)
                                                     : source.substr(pos);

        // Trim trailing \r.
        if (!line.empty() && line.back() == '\r') {
            line = line.substr(0, line.size() - 1);
        }

        // Only process comment lines.
        auto stripped_start = line.find_first_not_of(" \t");
        if (stripped_start != std::string_view::npos && line[stripped_start] == '#') {
            auto stripped = line.substr(stripped_start);

            if (is_stubs_begin(stripped)) {
                in_block = true;
            } else if (is_stubs_end(stripped)) {
                in_block = false;
            } else if (in_block && ci_contains(stripped, "stub")) {
                SourcePosition spos{lineno, 0, 0};
                Range line_range{spos, spos};

                auto expr_stub = parse_expr_stub_line(stripped, line_range);
                if (expr_stub.has_value()) {
                    result.expressions.push_back(std::move(*expr_stub));
                } else {
                    auto cmd_stub = parse_stub_line(stripped, line_range);
                    if (cmd_stub.has_value()) {
                        result.commands.push_back(std::move(*cmd_stub));
                    }
                }
            }
        }

        if (eol == std::string_view::npos) {
            break;
        }
        pos = eol + 1;
        ++lineno;
    }

    return result;
}

} // namespace tcl_lsp
