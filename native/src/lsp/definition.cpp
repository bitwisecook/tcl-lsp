#include "tcl_lsp/lsp/definition.hpp"

#include "tcl_lsp/analysis/semantic_types.hpp"
#include "tcl_lsp/parsing/lexer.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace tcl_lsp {
namespace {

// Identifier character predicate.
auto is_ident_char(char c) -> bool {
    return (std::isalnum(static_cast<unsigned char>(c)) != 0) || c == '_' || c == ':';
}

// Result of word_at_position: the word text and whether it's a variable.
struct WordResult {
    std::string word;
    bool is_var = false;
};

// Find the word at the given position in source.
auto word_at_position(std::string_view source, std::int32_t line, std::int32_t character)
    -> WordResult {
    // Find the start of the target line.
    std::int32_t cur_line = 0;
    std::size_t pos = 0;
    while (cur_line < line && pos < source.size()) {
        if (source[pos] == '\n') cur_line += 1;
        pos += 1;
    }
    auto line_start = pos;
    auto col = static_cast<std::size_t>(character);
    auto target = line_start + col;
    if (target >= source.size()) return {};

    bool is_var = false;
    auto word_start = target;

    // Direct checks when cursor is on or immediately after '$'.
    if (source[target] == '$' && target + 1 < source.size()) {
        is_var = true;
        word_start = (source[target + 1] == '{') ? target + 2 : target + 1;
    } else if (target > 0 && source[target - 1] == '$') {
        is_var = true;
        word_start = (source[target] == '{') ? target + 1 : target;
    } else {
        // Scan left within the current word to find its start.
        while (word_start > line_start && is_ident_char(source[word_start - 1]))
            word_start -= 1;
        // Check for preceding '$' (simple $var form).
        if (word_start > line_start && source[word_start - 1] == '$') {
            is_var = true;
        }
        // Check for preceding '${' (braced ${var} form).
        else if (word_start > line_start + 1 &&
                 source[word_start - 1] == '{' && source[word_start - 2] == '$') {
            is_var = true;
        }
    }

    // Expand word to the right.
    auto word_end = word_start;
    while (word_end < source.size() && source[word_end] != '\n' && is_ident_char(source[word_end]))
        word_end += 1;

    if (word_end <= word_start) return {};
    return {std::string(source.substr(word_start, word_end - word_start)), is_var};
}

// Search scope tree for a variable definition.
auto find_var_in_scope(const Scope& scope, const std::string& name)
    -> const VarDef* {
    auto it = scope.variables.find(name);
    if (it != scope.variables.end()) return &it->second;
    // Check parent scopes.
    if (scope.parent != nullptr) return find_var_in_scope(*scope.parent, name);
    return nullptr;
}

// Find the innermost scope containing the given line.
auto find_scope_at_line(const Scope& scope, std::int32_t line)
    -> const Scope* {
    for (auto& child : scope.children) {
        if (child->body_range.has_value()) {
            auto& br = *child->body_range;
            if (line >= br.start.line && line <= br.end.line) {
                return find_scope_at_line(*child, line);
            }
        }
    }
    return &scope;
}

} // anonymous namespace

auto find_definition(std::string_view source,
                      std::int32_t line,
                      std::int32_t character,
                      const AnalysisResult& analysis)
    -> std::optional<Range> {
    auto [word, is_var] = word_at_position(source, line, character);
    if (word.empty()) return std::nullopt;

    if (is_var) {
        auto* scope = find_scope_at_line(analysis.global_scope(), line);
        if (scope != nullptr) {
            auto* var = find_var_in_scope(*scope, word);
            if (var != nullptr) return var->definition_range;
        }
    }

    // Try to find a proc definition.
    auto& procs = analysis.all_procs();
    auto it = procs.find(word);
    if (it != procs.end()) return it->second->name_range;

    // Try qualified name.
    auto qit = procs.find("::" + word);
    if (qit != procs.end()) return qit->second->name_range;

    return std::nullopt;
}

auto find_references(std::string_view source,
                      std::int32_t line,
                      std::int32_t character,
                      const AnalysisResult& analysis)
    -> std::vector<Range> {
    auto [word, is_var] = word_at_position(source, line, character);
    if (word.empty()) return {};

    // Find variable references.
    if (is_var) {
    auto* scope = find_scope_at_line(analysis.global_scope(), line);
    if (scope != nullptr) {
        auto* var = find_var_in_scope(*scope, word);
        if (var != nullptr) {
            std::vector<Range> refs;
            refs.push_back(var->definition_range);
            for (auto& r : var->references) refs.push_back(r);
            return refs;
        }
    }
    } // is_var

    // Find proc call sites via command_invocations.
    std::vector<Range> refs;
    for (auto& inv : analysis.command_invocations()) {
        if (inv.name == word || inv.resolved_qualified_name == word ||
            inv.resolved_qualified_name == "::" + word) {
            refs.push_back(inv.range);
        }
    }

    // Also include the definition site at the front.
    auto& procs = analysis.all_procs();
    auto it = procs.find(word);
    if (it != procs.end()) {
        std::vector<Range> result;
        result.reserve(refs.size() + 1);
        result.push_back(it->second->name_range);
        for (auto& r : refs) result.push_back(r);
        return result;
    }

    return refs;
}

} // namespace tcl_lsp
