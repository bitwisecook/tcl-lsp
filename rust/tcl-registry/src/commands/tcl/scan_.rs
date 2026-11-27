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

//! `scan` — parse a string using scanf-style conversion.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "scan string format ?varName varName ...?",
    ..FormSpec::DEFAULT
}];

/// `scan string format ?varName ...?` accepts variable-name args from
/// index 2 onward to the end of the call.  Resolve `VarWrite` dynamically for
/// every trailing arg rather than hard-coding a finite slot count, so calls
/// with 20 / 50 / 100 vars don't false-fire W210 on the unmodelled tail.
/// Only cardinality is needed; unknown input or format values retain this layout.
fn scan_arg_roles(argument_count: usize) -> Vec<(u8, ArgRole)> {
    // Index 1 is the %-string (the §6 argument-DSL rung: `%b` is 8.6+).
    std::iter::once((1u8, ArgRole::ScanFormat))
        .chain(
            (2..argument_count)
                .filter_map(|i| u8::try_from(i).ok().map(|i| (i, ArgRole::VarWrite))),
        )
        .collect()
}

/// Fold successful inline `scan` matches: the `tcl-cmd-core` kernel over
/// the input and format words.
pub(crate) fn fold_scan(args: &[&str]) -> Option<String> {
    let [input, format] = args else {
        return None;
    };
    tcl_cmd_core::scan::scan_literal(input, format)
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "scan",
        evaluation_contexts: Some(crate::evaluation_context::EvaluationContextSpec::CALLER),
        // tclsh 8.6/9.0: completes normally or with an error, including
        // through variable traces and reflected channel handlers.
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::exact(&[
                crate::completion::CompletionCode::Ok,
                crate::completion::CompletionCode::Error,
            ]),
        ),
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::ConditionalVariableOperands(
                crate::variable_output::NativeVariableOutputSpec::Scan,
            ),
        ),
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("scan"),
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        // The match / conversion path is the only one that writes: a failed
        // `regexp`, and a `scan` or `binary scan` whose input runs out, leave
        // each remaining target's previous value in place and never create a
        // target that did not exist. Measured identical on tclsh 8.4.20,
        // 8.5.19, 8.6.18, 9.0.4 and 9.1b0. Without this the store feeding one
        // looked overwritten-before-read and O109 deleted it (#2051).
        traits: Traits::BYTE_COMPILED
            | Traits::FRAME_HASH_BUILTIN
            | Traits::CONDITIONAL_VARIABLE_WRITE,
        arity: Arity::at_least(2),
        // Documented return is the int conversion count (`scan str fmt
        // var ...`). The inline `scan str fmt` form (folded by `fold_scan`)
        // instead yields the converted values — a list once something
        // converts, and a pure string when nothing does, so the hook below
        // refines this to "not the count" rather than to `List`.
        return_type: Some(TclType::Int),
        // Two positionals (`string format`) is the inline form: no variables
        // to write, so the result is the converted values, not a count.
        return_type_hook: Some(ReturnTypeHookId::Scan),
        // `scan` writes format-dependent conversions (`%d` → int, `%s` →
        // string, `%f` → double) to its targets while returning the *count*.
        // Without parsing the format the target intreps are unknown, so they
        // must not be typed `Int`.
        var_write_typing: VarWriteTyping::Destructured,
        const_fold: Some(fold_scan),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::destructure::SCAN),
        hover: Some(HoverSnippet {
            summary: "Parse string using conversion specifiers in the style of sscanf",
            synopsis: &["scan string format ?varName varName ...?"],
            snippet: "Skips whitespace in string before each conversion, except %c and %[chars]/%[^chars], which do not. A %n$ positional specifier sends that conversion's result to argument n (1-based) instead of the next varName in sequence; once one specifier in format is positional, every specifier must be. Integer conversions (%d, %o, %x/%X, %b, %u, %i) take an optional size modifier that bounds the stored range: h or no modifier limits it to 32 bits, l limits it to the wide()/64-bit range, and ll removes the limit entirely (arbitrary precision). L is a synonym for ll (unlimited) on Tcl 9.0 and later, but on Tcl 8.5 and 8.6 it instead meant the same 64-bit range as l; Tcl 8.4 has neither ll nor an unlimited range, only a plain l or L meaning \"at least 64 bits\". Tcl 9.0 and later also accept q and j as synonyms for l (wide/64-bit range) and z and t as platform-pointerSize-dependent (matching h's 32-bit range or l's wide range depending on the tcl_platform(pointerSize) value); none of these four size modifiers are recognized before 9.0. %b (binary integer) needs Tcl 8.6 or later. With one or more varName arguments the command writes each converted value and returns the count of conversions performed, or -1 if string runs out before the first conversion completes; with no varName arguments it returns the converted values as a list instead, or the empty string on that same end-of-input failure.",
            source: "Tcl scan(n)",
            examples: "scan $string {#%2x%2x%2x} r g b\nscan $string {%d:%d} hours minutes\nset count [scan $string {%d %d} x y]\nset values [scan $string {%d %d}]",
            return_value: "The number of successful conversions, or -1 if the end of string is reached before any conversion completes. With no varName arguments, returns the converted values as a list instead, or the empty string on that same end-of-input failure.",
        }),
        forms: FORMS,
        side_effects: crate::knowledge::Knowledge::declared(SIDE_EFFECTS),
        arg_role_count_resolver: Some(scan_arg_roles),
        arg_role_resolver_roles: &[ArgRole::ScanFormat, ArgRole::VarWrite],
        // `scan`'s conversion string is the same printf-style mini-language
        // `format` writes; `scan_arg_roles` puts `ArgRole::ScanFormat` at the
        // word.
        format_string_type: Some(FormatType::Sprintf),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_scan;

    #[test]
    fn scan_folds_dialect_invariant_subset() {
        // Single + multiple integer / hex / octal / char / string conversions.
        assert_eq!(fold_scan(&["42", "%d"]).as_deref(), Some("42"));
        assert_eq!(fold_scan(&["-5", "%d"]).as_deref(), Some("-5"));
        assert_eq!(fold_scan(&[" 42", "%d"]).as_deref(), Some("42"));
        assert_eq!(fold_scan(&["ff", "%x"]).as_deref(), Some("255"));
        assert_eq!(fold_scan(&["-ff", "%x"]).as_deref(), Some("-255"));
        assert_eq!(fold_scan(&["17", "%o"]).as_deref(), Some("15"));
        assert_eq!(fold_scan(&["A", "%c"]).as_deref(), Some("65"));
        assert_eq!(fold_scan(&["abc", "%s"]).as_deref(), Some("abc"));
        assert_eq!(fold_scan(&["1 2 3", "%d %d %d"]).as_deref(), Some("1 2 3"));
        assert_eq!(fold_scan(&["x42", "x%d"]).as_deref(), Some("42"));
        assert_eq!(fold_scan(&["1.5", "%d"]).as_deref(), Some("1"));
        // The 32-bit boundary folds; one past it (per-version) bails.
        assert_eq!(
            fold_scan(&["2147483647", "%d"]).as_deref(),
            Some("2147483647")
        );
        assert_eq!(
            fold_scan(&["-2147483648", "%d"]).as_deref(),
            Some("-2147483648")
        );
        assert_eq!(
            fold_scan(&["7fffffff", "%x"]).as_deref(),
            Some("2147483647")
        );
    }

    #[test]
    fn scan_bails_on_unsound_and_unsupported() {
        // The varName form has side effects.
        assert_eq!(fold_scan(&["42", "%d", "v"]), None);
        // Numeric results outside the shared 32-bit range diverge.
        assert_eq!(fold_scan(&["2147483648", "%d"]), None);
        assert_eq!(fold_scan(&["-2147483649", "%d"]), None);
        assert_eq!(fold_scan(&["ffffffff", "%x"]), None);
        // `0x`-prefixed %x (main's wrong-fold case) bails.
        assert_eq!(fold_scan(&["0xff", "%x"]), None);
        assert_eq!(fold_scan(&["-0xff", "%x"]), None);
        // Unsupported conversions / specs bail.
        assert_eq!(fold_scan(&["10", "%i"]), None);
        assert_eq!(fold_scan(&["10", "%u"]), None);
        assert_eq!(fold_scan(&["3.5", "%f"]), None);
        assert_eq!(fold_scan(&["42", "%5d"]), None);
        assert_eq!(fold_scan(&["42", "%*d"]), None);
        assert_eq!(fold_scan(&["abc", "%[a-c]"]), None);
        // Partial / failed / empty matches and literal mismatch bail.
        assert_eq!(fold_scan(&["1 x", "%d %d"]), None);
        assert_eq!(fold_scan(&["x", "%d"]), None);
        assert_eq!(fold_scan(&["42", "x%d"]), None);
        assert_eq!(fold_scan(&["", "%d"]), None);
        // A `%s` run needing list-quoting, and non-ASCII input, bail.
        assert_eq!(fold_scan(&["a;b", "%s"]), None);
        assert_eq!(fold_scan(&["caf\u{e9}", "%s"]), None);
    }
}
