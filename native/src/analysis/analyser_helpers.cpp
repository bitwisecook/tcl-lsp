// Variable tracking, scope helpers, naming, diagnostics, expression analysis.

#include "tcl_lsp/analysis/analyser.hpp"
#include "tcl_lsp/parsing/recovery.hpp"

#include <algorithm>
#include <cctype>
#include <sstream>
#include <vector>

namespace tcl_lsp {

namespace {

auto range_from_token(const Token& tok) -> Range {
    return {tok.start, tok.end};
}

// Pure string normalisation — splits on ::, removes empties, rebuilds with :: prefix.
auto normalise_qualified(const std::string& name) -> std::string {
    if (name.empty()) return name;
    std::vector<std::string_view> parts;
    std::string_view sv = name;
    while (!sv.empty()) {
        auto sep = sv.find("::");
        if (sep == std::string_view::npos) {
            if (!sv.empty()) parts.push_back(sv);
            break;
        }
        auto part = sv.substr(0, sep);
        if (!part.empty()) parts.push_back(part);
        sv = sv.substr(sep + 2);
    }
    if (parts.empty()) return "::";
    std::string result = "::";
    for (std::size_t i = 0; i < parts.size(); ++i) {
        if (i > 0) result += "::";
        result += parts[i];
    }
    return result;
}

// Levenshtein distance (for "did you mean?" suggestions).
auto edit_distance(std::string_view a, std::string_view b) -> int32_t {
    if (a.empty()) return static_cast<int32_t>(b.size());
    if (b.empty()) return static_cast<int32_t>(a.size());
    auto m = a.size();
    auto n = b.size();
    std::vector<int32_t> prev(n + 1);
    std::vector<int32_t> curr(n + 1);
    for (std::size_t j = 0; j <= n; ++j) {
        prev[j] = static_cast<int32_t>(j);
    }
    for (std::size_t i = 1; i <= m; ++i) {
        curr[0] = static_cast<int32_t>(i);
        for (std::size_t j = 1; j <= n; ++j) {
            auto cost = (a[i - 1] == b[j - 1]) ? 0 : 1;
            curr[j] = std::min({prev[j] + 1, curr[j - 1] + 1, prev[j - 1] + cost});
        }
        std::swap(prev, curr);
    }
    return prev[n];
}

} // namespace

// ---------------------------------------------------------------------------
// Variable tracking
// ---------------------------------------------------------------------------

void Analyser::define_var(const std::string& name, Range range, Scope* scope,
                          bool warn_if_unused) {
    auto base = normalise_var_name(name);
    if (base.empty()) return;

    auto it = scope->variables.find(base);
    if (it == scope->variables.end()) {
        scope->variables[base] = VarDef{base, range, {}, warn_if_unused};
        result_.all_variables[scope->name + "::" + base] = &scope->variables[base];
        return;
    }
    if (warn_if_unused) {
        it->second.warn_if_unused = true;
    }
}

void Analyser::record_var_read(const std::string& name, Range range, Scope* scope) {
    auto base = normalise_var_name(name);
    if (base.empty()) return;

    auto it = scope->variables.find(base);
    if (it != scope->variables.end()) {
        it->second.references.push_back(range);
        return;
    }

    // Cross-scope variables (::var, static::var) — check global scope.
    if ((base.starts_with("::") || base.starts_with("static::")) &&
        scope != &result_.global_scope()) {
        auto git = result_.global_scope().variables.find(base);
        if (git != result_.global_scope().variables.end()) {
            git->second.references.push_back(range);
        }
    }
}

// ---------------------------------------------------------------------------
// Const string / regex tracking
// ---------------------------------------------------------------------------

void Analyser::set_const_string(const std::string& var_name, const std::string& value,
                                Range value_range, Scope* scope) {
    const_strings_[scope][var_name] = {value, value_range};
}

void Analyser::clear_const_string(const std::string& var_name, Scope* scope) {
    auto it = const_strings_.find(scope);
    if (it != const_strings_.end()) {
        it->second.erase(var_name);
    }
}

auto Analyser::lookup_const_string(const std::string& var_name, Scope* scope) const
    -> std::optional<std::pair<std::string, Range>> {
    auto* s = scope;
    while (s != nullptr) {
        auto scope_it = const_strings_.find(s);
        if (scope_it != const_strings_.end()) {
            auto var_it = scope_it->second.find(var_name);
            if (var_it != scope_it->second.end()) {
                return var_it->second;
            }
        }
        s = s->parent;
    }
    return std::nullopt;
}

void Analyser::record_defining_set_as_regex(const std::string& var_name, Scope* scope,
                                            const std::string& command) {
    auto* s = scope;
    while (s != nullptr) {
        auto scope_it = const_strings_.find(s);
        if (scope_it != const_strings_.end()) {
            auto var_it = scope_it->second.find(var_name);
            if (var_it != scope_it->second.end()) {
                result_.regex_patterns.push_back(RegexPattern{
                    var_it->second.second, var_it->second.first, command});
                return;
            }
        }
        s = s->parent;
    }
}

auto Analyser::regex_var_key(Scope* scope, const std::string& name) const -> std::string {
    // Unique key for (scope, var_name) pair.
    std::ostringstream oss;
    oss << static_cast<const void*>(scope) << "::" << name;
    return oss.str();
}

// ---------------------------------------------------------------------------
// Scope helpers
// ---------------------------------------------------------------------------

auto Analyser::make_child_scope(ScopeKind kind, const std::string& name,
                                Scope* parent) -> Scope* {
    auto* child = new Scope(); // NOLINT — owned by parent's destructor
    child->kind = kind;
    child->name = name;
    child->parent = parent;
    parent->children.push_back(child);
    return child;
}

auto Analyser::namespace_from_scope(Scope* scope) -> std::string {
    auto it = ns_cache_.find(scope);
    if (it != ns_cache_.end()) return it->second;

    std::vector<std::string> parts;
    auto* s = scope;
    while (s != nullptr) {
        if (s->kind == ScopeKind::NAMESPACE) {
            parts.push_back(s->name);
        }
        s = s->parent;
    }
    if (parts.empty()) {
        ns_cache_[scope] = "::";
        return "::";
    }
    std::reverse(parts.begin(), parts.end());

    std::string ns = "::";
    for (const auto& p : parts) {
        if (p.starts_with("::")) {
            ns = normalise_qualified(p);
        } else {
            ns = normalise_qualified(ns + "::" + p);
        }
    }
    ns_cache_[scope] = ns;
    return ns;
}

auto Analyser::normalise_qualified_name(const std::string& name, Scope* scope) -> std::string {
    if (name.empty()) return name;
    if (name.starts_with("::")) return normalise_qualified(name);
    if (scope) {
        auto ns = namespace_from_scope(scope);
        return normalise_qualified(ns + "::" + name);
    }
    return normalise_qualified("::" + name);
}

auto Analyser::normalise_var_name(const std::string& raw) -> std::string {
    std::string base = raw;
    if (base.starts_with("${") && base.ends_with("}") && base.size() >= 3) {
        base = base.substr(2, base.size() - 3);
    } else if (base.starts_with("$")) {
        base = base.substr(1);
    }
    auto paren = base.find('(');
    if (paren != std::string::npos) {
        base = base.substr(0, paren);
    }
    return base;
}

// ---------------------------------------------------------------------------
// Define variables from list (foreach var-list)
// ---------------------------------------------------------------------------

void Analyser::define_vars_from_list(const std::string& var_list_text,
                                     const Token& tok, Scope* scope) {
    auto text = tok.text;
    auto span = tok.end.offset - tok.start.offset + 1;
    auto content_offset = (span > static_cast<int32_t>(text.size())) ? 1 : 0;
    auto base_line = tok.start.line;
    auto base_col = tok.start.character + content_offset;
    auto base_off = tok.start.offset + content_offset;

    std::size_t search_start = 0;
    std::istringstream iss(var_list_text);
    std::string var_name;
    while (iss >> var_name) {
        if (var_name.empty()) continue;
        auto idx = text.find(var_name, search_start);
        if (idx != std::string::npos) {
            auto start_pos = position_from_relative(
                text, static_cast<int32_t>(idx), base_line, base_col, base_off);
            auto end_pos = position_from_relative(
                text, static_cast<int32_t>(idx + var_name.size() - 1),
                base_line, base_col, base_off);
            define_var(var_name, Range{start_pos, end_pos}, scope, true);
            search_start = idx + var_name.size();
        } else {
            define_var(var_name, range_from_token(tok), scope, true);
        }
    }
}

// ---------------------------------------------------------------------------
// Unknown proc analysis (stubbed — requires IR lowering)
// ---------------------------------------------------------------------------

void Analyser::extract_unknown_proc_info(const std::string& proc_body) {
    auto start = proc_body.find_first_not_of(" \t\n\r");
    if (start == std::string::npos) {
        // Empty stub — does not suppress W123.
        result_.unknown_proc_info = UnknownProcInfo{{}, false, true, false, false, false, false};
        return;
    }
    // Without IR analysis, be conservative — treat as opaque handler.
    result_.unknown_proc_info =
        UnknownProcInfo{{}, true, false, true, true, true, true};
}

// ---------------------------------------------------------------------------
// W123 — unresolved command diagnostics (post-analysis pass)
// ---------------------------------------------------------------------------

void Analyser::emit_unresolved_command_diagnostics() {
    if (unresolved_commands_emitted_) return;
    unresolved_commands_emitted_ = true;

    if (disabled_diagnostics_.contains("W123")) return;

    // Gather known command names.
    std::unordered_set<std::string> registry_names;
    if (registry_) {
        auto names = registry_->command_names();
        registry_names.insert(names.begin(), names.end());
    }
    std::unordered_set<std::string> stub_names;
    for (const auto& s : result_.stub_commands) {
        stub_names.insert(s.name);
    }

    // Build proc tail names.
    std::unordered_set<std::string> proc_tail_names;
    for (const auto& [qname, _] : result_.all_procs) {
        auto sep = qname.rfind("::");
        if (sep != std::string::npos) {
            auto tail = qname.substr(sep + 2);
            if (!tail.empty()) proc_tail_names.insert(tail);
        }
    }

    // Check unknown proc info for suppression.
    auto& upi = result_.unknown_proc_info;
    if (upi.has_value() &&
        (upi->chains_original || upi->has_exec || upi->has_auto_load ||
         upi->case_insensitive || upi->has_pattern_dispatch)) {
        return;
    }
    if (result_.has_dynamic_providers) return;
    if (!result_.package_requires.empty()) return;

    auto dispatch_targets =
        upi.has_value() ? upi->dispatch_targets : std::unordered_set<std::string>{};

    // Alias tail names.
    std::unordered_set<std::string> alias_names;
    for (const auto& [qname, _] : result_.command_aliases) {
        auto sep = qname.rfind("::");
        if (sep != std::string::npos) {
            auto tail = qname.substr(sep + 2);
            if (!tail.empty()) alias_names.insert(tail);
        }
    }

    // Build candidate pool for suggestions.
    std::unordered_set<std::string> candidates;
    candidates.insert(registry_names.begin(), registry_names.end());
    candidates.insert(proc_tail_names.begin(), proc_tail_names.end());
    candidates.insert(stub_names.begin(), stub_names.end());
    candidates.insert(dispatch_targets.begin(), dispatch_targets.end());
    candidates.insert(alias_names.begin(), alias_names.end());

    for (const auto& inv : result_.command_invocations) {
        const auto& name = inv.name;

        if (registry_names.contains(name)) continue;
        if (!inv.resolved_qualified_name.empty()) continue;
        if (name.find("::") != std::string::npos) continue;
        if (name.starts_with("$") || name.starts_with("[")) continue;
        if (stub_names.contains(name)) continue;
        if (alias_names.contains(name)) continue;
        if (dispatch_targets.contains(name)) continue;
        if (proc_tail_names.contains(name)) continue;

        std::string msg = "Unknown command '" + name + "'";

        // "Did you mean?" suggestion (edit distance ≤ 2).
        std::string best;
        int32_t best_dist = 3;
        for (const auto& c : candidates) {
            auto d = edit_distance(name, c);
            if (d < best_dist) {
                best_dist = d;
                best = c;
            }
        }

        std::vector<CodeFix> fixes;
        if (!best.empty()) {
            msg += "; did you mean '" + best + "'?";
            fixes.push_back(
                CodeFix{inv.range, best, "Replace with '" + best + "'"});
        }

        emit_diagnostic(inv.range, Severity::HINT, "W123", msg, std::move(fixes));
    }
}

// ---------------------------------------------------------------------------
// Diagnostic deduplication
// ---------------------------------------------------------------------------

void Analyser::dedupe_diagnostics() {
    struct Key {
        std::string code;
        int32_t start_offset;
        int32_t end_offset;
        std::string message;
        Severity severity;

        auto operator==(const Key& o) const -> bool = default;

        struct Hash {
            auto operator()(const Key& k) const noexcept -> std::size_t {
                auto h = std::hash<std::string>{}(k.code);
                h ^= std::hash<int32_t>{}(k.start_offset) + 0x9e3779b9 + (h << 6) + (h >> 2);
                h ^= std::hash<int32_t>{}(k.end_offset) + 0x9e3779b9 + (h << 6) + (h >> 2);
                h ^= std::hash<std::string>{}(k.message) + 0x9e3779b9 + (h << 6) + (h >> 2);
                return h;
            }
        };
    };

    // Collect E101 lines for redundant E002 suppression.
    std::unordered_set<int32_t> e101_lines;
    for (const auto& d : result_.diagnostics) {
        if (d.code == "E101") e101_lines.insert(d.range.start.line);
    }

    std::unordered_set<Key, Key::Hash> seen;
    std::vector<Diagnostic> deduped;
    for (auto& d : result_.diagnostics) {
        Key key{d.code, d.range.start.offset, d.range.end.offset, d.message, d.severity};
        if (seen.contains(key)) continue;
        if (d.code == "E002" && e101_lines.contains(d.range.start.line)) continue;
        seen.insert(key);
        deduped.push_back(std::move(d));
    }
    result_.diagnostics = std::move(deduped);
}

auto Analyser::is_diagnostic_suppressed(int32_t line, const std::string& code) const -> bool {
    auto it = result_.suppressed_lines.find(line);
    if (it == result_.suppressed_lines.end()) return false;
    return it->second.contains("*") || it->second.contains(code);
}

void Analyser::emit_diagnostic(Range range, Severity severity, const std::string& code,
                               const std::string& message, std::vector<CodeFix> fixes) {
    if (disabled_diagnostics_.contains(code)) return;
    if (is_diagnostic_suppressed(range.start.line, code)) return;
    result_.diagnostics.push_back(
        Diagnostic{range, severity, code, message, std::move(fixes)});
}

// ---------------------------------------------------------------------------
// Expression analysis (simplified scanner — full expr lexer in Phase 5)
// ---------------------------------------------------------------------------

void Analyser::analyse_expr(std::string_view expr_text, Scope* scope) {
    if (expr_text.empty()) return;

    std::size_t i = 0;
    while (i < expr_text.size()) {
        if (expr_text[i] == '\\' && i + 1 < expr_text.size()) {
            i += 2;
            continue;
        }

        if (expr_text[i] == '$') {
            ++i;
            std::string var_name;
            if (i < expr_text.size() && expr_text[i] == '{') {
                ++i;
                auto end = expr_text.find('}', i);
                if (end != std::string_view::npos) {
                    var_name = std::string(expr_text.substr(i, end - i));
                    i = end + 1;
                }
            } else {
                auto start = i;
                while (i < expr_text.size() &&
                       (std::isalnum(static_cast<unsigned char>(expr_text[i])) ||
                        expr_text[i] == '_' || expr_text[i] == ':')) {
                    ++i;
                }
                var_name = std::string(expr_text.substr(start, i - start));
            }
            if (!var_name.empty()) {
                record_var_read(var_name, Range::zero(), scope);
            }
            continue;
        }

        if (expr_text[i] == '[') {
            ++i;
            int level = 1;
            auto start = i;
            while (i < expr_text.size() && level > 0) {
                if (expr_text[i] == '\\' && i + 1 < expr_text.size()) {
                    i += 2;
                    continue;
                }
                if (expr_text[i] == '[') ++level;
                else if (expr_text[i] == ']') --level;
                ++i;
            }
            if (level == 0) {
                auto cmd_text = expr_text.substr(start, i - start - 1);
                analyse_body(cmd_text, scope);
            }
            continue;
        }

        ++i;
    }
}

// ---------------------------------------------------------------------------
// Scan token stream for variable references and command substitutions
// ---------------------------------------------------------------------------

void Analyser::scan_var_references(const std::vector<Token>& tokens, Scope* scope) {
    for (const auto& tok : tokens) {
        if (tok.type == TokenType::VAR) {
            record_var_read(tok.text, range_from_token(tok), scope);
        } else if (tok.type == TokenType::CMD) {
            analyse_body(tok.text, scope, &tok);
        }
    }
}

} // namespace tcl_lsp
