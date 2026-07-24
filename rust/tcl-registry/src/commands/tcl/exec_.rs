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

//! `exec` — invoke subprocesses.
use crate::prelude::*;

// The trailing `?&?` background marker is spelled out in the SYNOPSIS line
// only from the 8.6 manpage onward (8.4/8.5 show `exec ?switches? arg
// ?arg ...?` with no `?&?`), but the behaviour itself — a trailing bare
// `&` backgrounds the pipeline and returns a list of subprocess ids — is
// documented in identical DESCRIPTION prose in every version 8.4 through
// 9.1. That makes this a single universal form, not a version-gated one.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exec ?switches? arg ?arg ...? ?&?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exec",
        // `Some(DialectSet::ALL_TCL)` is deliberate, not an oversight: F5
        // iRules is the one modelled dialect that drops `exec` (the
        // K36322151 TMM sandbox ban — no real process model), and that
        // exclusion is expressed directly by this `dialects` group:
        // `ALL_TCL` does not carry the `IRULES` bit, so this spec never
        // intersects the bare `IRULES` availability mask. Every other
        // modelled dialect that embeds a real Tcl runtime (Expect,
        // f5-iapps, f5-tmsh, the EDA vendor shells) is included in
        // `ALL_TCL` and keeps a real process model, so `exec` is available
        // there too.
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::BYTE_COMPILED
            | Traits::TAINT_SINK
            | Traits::TAINT_SOURCE
            | Traits::UNSAFE
            | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Process,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            // A failing subprocess populates the errorCode global with a
            // structured CHILDSTATUS/CHILDKILLED/CHILDSUSPENDED value and
            // errorInfo with the pipeline's collected output —
            // interpreter state beyond the raised error itself.
            SideEffect {
                target: SideEffectTarget::InterpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        // ``--`` is the option terminator that drives W304's
        // ``resolve_option_terminator`` lookup; the registry also
        // surfaces the two boolean switches for completion.
        options: const {
            &[
                OptionSpec {
                    name: "-encoding",
                    value: OptionValue::value("encodingName"),
                    detail: "Encoding used to decode subprocess output captured by exec, defaulting to the interpreter's system encoding ([encoding system]). Added in Tcl 9.0; earlier Tcl always uses the system encoding, and other encodings (including raw binary) require opening and reading the pipeline explicitly.",
                    dialects: Some(DialectSet::TCL90_PLUS),
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-ignorestderr",
                    value: OptionValue::flag(),
                    detail: "Stops exec from treating unredirected output on a subprocess's stderr as an error case. Added in Tcl 8.5; in 8.4 any unredirected stderr output always makes exec raise an error.",
                    dialects: Some(DialectSet::TCL85_PLUS),
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-keepnewline",
                    value: OptionValue::flag(),
                    detail: "Retains a trailing newline in the pipeline's result or error message; normally a single trailing newline is deleted.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; the following argument is treated as the first pipeline word even if it begins with -.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        hover: Some(HoverSnippet {
            summary: "Invoke one or more subprocesses as a shell-style pipeline.",
            synopsis: &["exec ?switches? arg ?arg ...? ?&?"],
            snippet: "Non-switch words either become literal arguments of the current subprocess or are pipeline syntax that is stripped before exec'ing and never reaches it: | and |& start a new subprocess piped from the previous one's stdout (or stdout+stderr); <, <@, and << set the first subprocess's stdin from a file, an already-open channel, or a literal value; >, >>, 2>, 2>>, >&, >>&, >@, 2>@, and >&@ redirect a subprocess's stdout/stderr to a file or open channel, and 2>@1 (Tcl 8.5+) folds stderr into exec's own result instead of a file. The first word of each subprocess command is tilde-substituted in Tcl 8.4-8.6 only (that step was dropped in Tcl 9.0) and then, if it has no slash, a PATH search is performed; no glob expansion or other shell substitution is performed on any argument. Unredirected stderr output from any subprocess is treated as an error, as is an abnormal exit (killed, suspended, or nonzero status) — see -ignorestderr. A trailing bare & backgrounds the whole pipeline instead of waiting for it, returning subprocess identifiers rather than output.",
            source: "Tcl exec(n)",
            examples: "exec ls -l /tmp\nexec sort < in.txt | uniq -c > out.txt\nset matches [exec grep -c foo $path 2>@1]\nset pids [exec sleep 30 &]",
            return_value: "The standard output of the last subprocess in the pipeline, decoded with -encoding (Tcl 9.0+) or otherwise the system encoding, with one trailing newline stripped unless -keepnewline is given. With a trailing &, a list of the pipeline's subprocess process identifiers instead.",
        }),
        // A `SHELL_ATOM`-coloured value is token-safe and
        // suppresses T100.
        taint_sink_safe_colour: Some(TaintColour::SHELL_ATOM),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
