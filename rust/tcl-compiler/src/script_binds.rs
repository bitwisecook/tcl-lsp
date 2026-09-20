// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! "Read as a script, does this word bind this name?" — [`script_binds_name`].
//!
//! Two passes ask the same question about a word the pipeline could not
//! lower: [`crate::ssa`]'s `braced_word_class`, deciding whether a `$name`
//! inside a brace-quoted word of an **undescribed** command is a read of the
//! enclosing frame, and the read-before-set emitter's
//! `barrier_body_locally_sets`, deciding the same for an opaque
//! `Statement::Barrier` body. In both, a word that binds the name itself
//! reads *its own* local whichever frame it runs in, so the enclosing frame's
//! binding is not what the `$name` refers to and W210 must stay silent.
//!
//! Both used to answer it by looking for a **top-level `set NAME`** and
//! nothing else, which is one spelling of binding out of many. A body whose
//! writes sit one block deeper, or that binds through any other command, read
//! as "never binds it":
//!
//! ```tcl
//! test one {a test} -body {
//!     foreach it $items { set last $it }   ;# binds `it` and `last`
//!     catch {risky} err                    ;# binds `err`
//!     list $last $err $it
//! } -result {…}
//! ```
//!
//! Every one of `it`, `last` and `err` drew `W210 read before it is set`
//! against the outer frame, on a body that plainly sets all three — reported
//! as issue #2117, where `tcltest`'s bare `test` spelling is undescribed
//! while the `tcltest::test` spelling lowers to a barrier and stays silent.
//!
//! The registry already knows every binding spelling — `ArgRole::VarWrite`
//! for an output operand, `ArgRole::LoopVarList` for a loop's own variables —
//! and knows which words are scripts (`ArgRole::Body`, `OpaqueScript`), so
//! this walks the word as a script, asks the registry per invocation, and
//! recurses into the script-shaped words. No command is named here.

use tcl_registry::{ArgRole, CommandRegistry};

/// How deep to follow nested script words.
///
/// Every level is a `{…}` the answer has to look inside, and real bodies nest
/// a handful deep — a `foreach` around an `if` around a `catch` is three. The
/// bound exists so a pathological body cannot make this walk quadratic in a
/// hot analyser path; refusing to descend further can only return `false`,
/// which is the conservative answer (the read stands).
const MAX_DEPTH: u8 = 6;

/// Which mentions of `name` count as the script owning it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ownership {
    /// Only a binding: an `ArgRole::VarWrite` operand or an
    /// `ArgRole::LoopVarList` entry.  A *read* of the name still belongs to
    /// whichever frame the script runs in, which for an undescribed
    /// command's word may be this one.
    Bindings,
    /// A binding, or a bare-name read (`set y`, `info exists y`).  The
    /// barrier twin wants this: its body runs in a context this frame cannot
    /// see (`interp eval PATH {…}`), so a name the body names at all is that
    /// context's, and reporting it here would blame the wrong interpreter.
    BindingsOrNameReads,
}

/// True when `word`, read as a script, owns `name` under `ownership` — by any
/// command the registry says takes a variable operand, at any nesting depth
/// up to [`MAX_DEPTH`].
///
/// `name` is already normalised (`crate::naming::normalise_var_name`).
pub(crate) fn script_binds_name(
    word: &str,
    name: &str,
    ownership: Ownership,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> bool {
    // A script that never spells the name cannot bind it, and segmenting is
    // the expensive half. The callers reach here having found the name in the
    // word, so this rejects only the nested levels.
    if !word.contains(name) {
        return false;
    }
    binds(word, name, ownership, registry, config, MAX_DEPTH)
}

fn binds(
    script: &str,
    name: &str,
    ownership: Ownership,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    depth: u8,
) -> bool {
    if depth == 0 {
        return false;
    }
    for segment in crate::segmenter::segment_commands_with_offset_and_config(script, 0, config) {
        let Some((command, args)) = segment.texts.split_first() else {
            continue;
        };
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let at = |role: ArgRole| registry.arg_indices_for_role(command, &args, role);

        // An output operand: `catch … err`, `scan … out`, `binary scan … v`,
        // `lassign`'s tail, `incr`, `append`, `lappend`, `upvar`'s locals.
        // `VarRead` joins it for a barrier body — a bare-name read such as
        // the one-argument `set y` names that context's variable too.
        let name_roles: &[ArgRole] = match ownership {
            Ownership::Bindings => &[ArgRole::VarWrite],
            Ownership::BindingsOrNameReads => &[ArgRole::VarWrite, ArgRole::VarRead],
        };
        if name_roles
            .iter()
            .copied()
            .flat_map(&at)
            .filter_map(|index| args.get(index))
            .any(|word| crate::naming::normalise_var_name(word) == name)
        {
            return true;
        }
        // A loop's own variables: `foreach {k v} $pairs …` binds `k` and `v`.
        if at(ArgRole::LoopVarList)
            .into_iter()
            .filter_map(|index| args.get(index))
            .any(|list| {
                list.split_whitespace()
                    .any(|word| crate::naming::normalise_var_name(word) == name)
            })
        {
            return true;
        }
        // A script-shaped word binds in the same frame its own command runs
        // in, which for a body role is this one.
        if [ArgRole::Body, ArgRole::OpaqueScript]
            .into_iter()
            .flat_map(&at)
            .filter_map(|index| args.get(index))
            .any(|body| binds(body, name, ownership, registry, config, depth - 1))
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{Ownership, script_binds_name};

    fn binds(script: &str, name: &str) -> bool {
        script_binds_name(
            script,
            name,
            Ownership::Bindings,
            tcl_registry::default_registry(),
            tcl_lexer::LexerConfig::default(),
        )
    }

    fn owns(script: &str, name: &str) -> bool {
        script_binds_name(
            script,
            name,
            Ownership::BindingsOrNameReads,
            tcl_registry::default_registry(),
            tcl_lexer::LexerConfig::default(),
        )
    }

    /// The shape the old top-level-`set` reading already answered, kept so a
    /// rewrite cannot lose it.
    #[test]
    fn a_top_level_set_still_binds() {
        assert!(binds("set a 1\nputs $a", "a"));
        assert!(!binds("puts $a", "a"));
    }

    /// The #2117 body: three names, three spellings, none of them a
    /// top-level `set`.
    #[test]
    fn a_nested_set_a_loop_variable_and_a_catch_operand_all_bind() {
        let body = "foreach it $items { set last $it }\ncatch {risky} err\nlist $last $err $it";
        for name in ["it", "last", "err"] {
            assert!(binds(body, name), "`{name}` is bound by this body");
        }
        assert!(!binds(body, "items"), "`items` is only read");
    }

    /// Depth, in both directions: a write far enough down is not found, which
    /// is the conservative answer rather than a wrong one.
    #[test]
    fn nesting_is_followed_to_a_bound() {
        assert!(binds("if {1} { while {1} { set deep 1 } }", "deep"));
        let too_deep = "if {1} { if {1} { if {1} { if {1} { if {1} { if {1} { set x 1 } } } } } }";
        assert!(!binds(too_deep, "x"));
    }

    /// An array element write binds the array, as `normalise_var_name` reads
    /// it everywhere else.
    #[test]
    fn an_array_element_write_binds_the_array() {
        assert!(binds("foreach k $ks { set map($k) 1 }\nparray map", "map"));
    }

    /// A one-argument `set` reads its operand rather than binding it, so it
    /// is a binding for nobody — but it *names* the variable, which is what
    /// an opaque barrier body (`interp eval i {set y} 7`) needs: the name
    /// belongs to the child interpreter, not to this frame.
    #[test]
    fn a_bare_name_read_is_ownership_only_for_a_barrier_body() {
        assert!(!binds("set y", "y"));
        assert!(owns("set y", "y"));
    }
}
