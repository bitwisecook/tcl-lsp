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

// Find the word at the given position in source.
auto word_at_position(std::string_view source, std::int32_t line, std::int32_t character)
    -> std::string {
    // Find the line.
    std::int32_t cur_line = 0;
    std::size_t pos = 0;
    while (cur_line < line && pos < source.size()) {
        if (source[pos] == '\n') cur_line += 1;
        pos += 1;
    }
    // Now pos is at the start of the target line.
    auto line_start = pos;
    auto col = static_cast<std::size_t>(character);
    auto target = line_start + col;
    if (target >= source.size()) return {};

    // Check if we're on a variable reference ($var).
    bool is_var = false;
    auto word_start = target;
    if (target > 0 && source[target - 1] == '$') {
        is_var = true;
        word_start = target;
    } else if (source[target] == '$' && target + 1 < source.size()) {
        is_var = true;
        word_start = target + 1;
    }

    // Expand word boundaries.
    if (!is_var) {
        while (word_start > line_start && (std::isalnum(static_cast<unsigned char>(source[word_start - 1])) != 0 ||
               source[word_start - 1] == '_' || source[word_start - 1] == ':'))
            word_start -= 1;
    }
    auto word_end = is_var ? word_start : target;
    while (word_end < source.size() && source[word_end] != '\n' &&
           (std::isalnum(static_cast<unsigned char>(source[word_end])) != 0 ||
            source[word_end] == '_' || source[word_end] == ':'))
        word_end += 1;

    if (word_end <= word_start) return {};
    return std::string(source.substr(word_start, word_end - word_start));
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
    auto word = word_at_position(source, line, character);
    if (word.empty()) return std::nullopt;

    // Check if it's a variable reference.
    // Look at the character before the word to see if it's $.
    std::int32_t cur_line = 0;
    std::size_t pos = 0;
    while (cur_line < line && pos < source.size()) {
        if (source[pos] == '\n') cur_line += 1;
        pos += 1;
    }
    auto col = static_cast<std::size_t>(character);
    auto target = pos + col;
    bool is_var = (target > 0 && target <= source.size() && source[target - 1] == '$') ||
                   (target < source.size() && source[target] == '$');

    if (is_var) {
        // Find the scope at this line and search for the variable.
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
    auto word = word_at_position(source, line, character);
    if (word.empty()) return {};

    // Find variable references.
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
