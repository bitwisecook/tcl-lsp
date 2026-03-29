#include "tcl_lsp/lsp/known_commands.hpp"

#include "tcl_lsp/registry/command_desc.hpp"

#include <string_view>
#include <unordered_set>

// Include generated command descriptors.
#include "../registry/generated/all_commands.cpp"

namespace tcl_lsp {
namespace {

// Build the keyword set from the generated registry at first use.
// This replaces the previous hardcoded ~150 command list with all 1840+
// generated commands, plus control-flow words and OO definition keywords
// that appear as arguments rather than standalone commands.
const std::unordered_set<std::string_view>& keyword_set() {
    static const auto s = [] {
        std::unordered_set<std::string_view> set;
        set.reserve(2048);
        for (const auto* desc : generated::all_commands) {
            set.insert(desc->name);
            // Also register subcommand-qualified names (e.g. "string length")
            // as individual subcommand names for definition-context highlighting.
        }
        // Control-flow words that appear as arguments, not standalone commands.
        for (auto kw : {"else", "elseif", "then", "on", "trap", "finally"}) {
            set.insert(kw);
        }
        // TclOO definition-context keywords.
        for (auto kw : {"constructor", "destructor", "method", "my", "next",
                        "self", "forward", "mixin", "filter", "superclass",
                        "renamemethod", "deletemethod", "export", "unexport"}) {
            set.insert(kw);
        }
        // tcltest commands (imported unqualified).
        for (auto kw : {"test", "cleanupTests", "makeFile", "removeFile",
                        "makeDirectory", "removeDirectory", "viewFile",
                        "customMatch", "testConstraint"}) {
            set.insert(kw);
        }
        return set;
    }();
    return s;
}

const std::unordered_set<std::string_view>& operator_set() {
    static const std::unordered_set<std::string_view> s = {
        "+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!=",
    };
    return s;
}

} // anonymous namespace

auto is_keyword(std::string_view name) -> bool {
    return keyword_set().count(name) > 0;
}

auto is_operator(std::string_view name) -> bool {
    return operator_set().count(name) > 0;
}

} // namespace tcl_lsp
