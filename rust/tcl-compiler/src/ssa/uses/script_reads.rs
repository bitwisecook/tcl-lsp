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

//! The free reads of the scripts a statement keeps inside itself, such as the arms of an opaque `switch`.

use super::super::SsaInvocationContext;
use super::super::defs::collapsed_extra_defs;
use super::{ClassifiedUses, expr_vars_for, uses_of_classified_in_context};
use crate::ir::Statement;
use crate::var_refs::VarReferenceScanner;
use std::collections::{BTreeSet, HashSet};

/// Reads of a non-lowered (`-glob`/`-regexp`, or `-exact` with a fall-through
/// arm) `switch` kept opaque in a CFG block: the subject word, every arm
/// pattern, and the *free* reads of each arm/default body.
///
/// The arm bodies are the *only* scripts in the pipeline that reach SSA
/// un-lowered, so this walk is the one place a `UseClass` would otherwise be
/// invented rather than derived. It is threaded through instead: a
/// brace-quoted data word inside an arm keeps the same
/// [`UseClass::Quoted`] it would carry had the arm been lowered, so
/// read-before-set skips it while liveness still honours it.
pub(super) fn switch_reads(
    subject: &str,
    arms: &[crate::ir::SwitchArm],
    default_body: Option<&crate::ir::Script>,
    patterns_braced: bool,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let mut reads = ClassifiedUses {
        substituted: selection.word_reads(scanner, subject),
        quoted: BTreeSet::new(),
        by_name: BTreeSet::new(),
    };
    for arm in arms {
        // A pattern from the canonical single braced `{pat body …}` block is a
        // literal list element — tclsh 9.0.4: `proc f {z} {switch -glob $z
        // {$a* {puts hit} default {puts miss}}}; f {$a}` prints `hit` with `a`
        // undefined. Supplied as separate words (`switch $s $pat {body}`) the
        // pattern does substitute. Same classification the lowered `-exact`
        // path already applies to its patterns.
        let pattern_sink = if patterns_braced {
            &mut reads.quoted
        } else {
            &mut reads.substituted
        };
        pattern_sink.extend(selection.word_reads(scanner, &arm.pattern));
        if let Some(body) = &arm.body {
            reads.merge(free_reads_in_script(body, scanner, selection));
        }
    }
    if let Some(db) = default_body {
        reads.merge(free_reads_in_script(db, scanner, selection));
    }
    reads
}

/// Reads a collapsed body consumes from the *outer* scope: its reads minus its
/// own defs (so arm-local temporaries — `set tmp 1; puts $tmp` — aren't seen as
/// outer reads). The def set is completed with the `for`-init/next and
/// if/while/for condition command-sub defs that `defs_from_ir_script` omits.
pub(in super::super) fn free_reads_in_script(
    script: &crate::ir::Script,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let registry = selection.registry;
    let mut defs: HashSet<String> = crate::ir_helpers::defs_from_ir_script(script)
        .into_iter()
        .collect();
    if selection.standalone {
        defs.extend(collapsed_extra_defs(script, registry, 0));
    }
    let mut reads = reads_in_script(script, scanner, selection);
    reads.remove_defs(&defs);
    reads
}

/// Recursively collect classified variable reads from an un-lowered IR script.
fn reads_in_script(
    script: &crate::ir::Script,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let mut reads = ClassifiedUses::default();
    for stmt in &script.statements {
        reads.merge(reads_in_stmt(stmt, scanner, selection));
    }
    reads
}

/// Classified variable reads of a single statement, recursing into nested
/// bodies. Leaf reads come from [`uses_of_classified`] (which resolves a
/// nested `Statement::Switch` via [`switch_reads`]); structured statements are
/// walked here because they are not lowered inside an opaque switch arm.
///
/// Every context this walk adds by hand — an `if`/`while`/`for` condition, a
/// loop's value word, a nested body — is one the enclosing frame really does
/// evaluate, so it is [`UseClass::Substituted`]; the single exception is a
/// **braced** loop value word, which is literal list text.
pub(in super::super) fn reads_in_stmt(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> ClassifiedUses {
    let mut reads = ClassifiedUses::default();
    reads.merge_classified(uses_of_classified_in_context(stmt, scanner, selection));
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                reads
                    .substituted
                    .extend(expr_vars_for(scanner, &clause.condition));
                reads.merge(reads_in_script(&clause.body, scanner, selection));
            }
            if let Some(eb) = else_body {
                reads.merge(reads_in_script(eb, scanner, selection));
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            reads.substituted.extend(expr_vars_for(scanner, condition));
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            reads.merge(reads_in_script(init, scanner, selection));
            reads.substituted.extend(expr_vars_for(scanner, condition));
            reads.merge(reads_in_script(next, scanner, selection));
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for it in iterators {
                // A braced value word is literal list text — `foreach n {a $b
                // c}` iterates the three characters `$b`, it does not read
                // `b`. The name is still *recorded*, as `Quoted`: dropping it
                // here would take its liveness use with it and draw a false
                // W220 on a store whose only mention is that word.
                // Classifying is what separates the two.
                let sink = if it.list_braced {
                    &mut reads.quoted
                } else {
                    &mut reads.substituted
                };
                sink.extend(selection.word_reads(scanner, &it.list_arg));
            }
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::Catch { body, .. } => {
            reads.merge(reads_in_script(body, scanner, selection));
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            reads.merge(reads_in_script(body, scanner, selection));
            for handler in handlers {
                reads.merge(reads_in_script(&handler.body, scanner, selection));
            }
            if let Some(fb) = finally_body {
                reads.merge(reads_in_script(fb, scanner, selection));
            }
        }
        _ => {}
    }
    reads
}
