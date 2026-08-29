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

//! `http::geturl` command.
//!
//! **P5 (the adversarial-module walk).** `http::geturl` carried the
//! taint facts and no option table at all — the deep dive's "the two
//! command-prefix callbacks that are invisible today".  The table below
//! is read out of `geturl`'s own `set options {…}` list and its callback
//! invocation sites in `library/http/http.tcl`, across all four bundled
//! reference trees:
//!
//! | Tcl tree | shipped `http` | option delta |
//! |---|---|---|
//! | `tmp/tcl8.4.20` | 2.5.8  | the 14-option base set |
//! | `tmp/tcl8.5.19` | 2.7.13 | adds `-keepalive -method -myaddr -protocol -strict` |
//! | `tmp/tcl8.6.16` | 2.9.8  | unchanged from 8.5 |
//! | `tmp/tcl9.0.4`  | 2.10.2 | adds `-guesstype` |
//!
//! The release deltas are declared as **core** `dialects` gates rather
//! than `http`-axis lifecycles on purpose: what the sources prove is
//! which *Tcl distribution* ships an `http` carrying the option (8.4's
//! 2.5.8 has no `-keepalive`; 8.5's 2.7.13 does), not the exact `http`
//! patch release that introduced it — and inventing `introduced: "2.7"`
//! would assert a boundary between 2.5.8 and 2.7.13 that no bundled tree
//! witnesses.  An `http` axis lifecycle stays available for whenever a
//! per-`http`-release history is imported.
//!
//! The four callbacks' appended arities come from the call sites, not the
//! documentation:
//!
//! - `-command`: `namespace eval :: $state(-command) $token` (9.0.4:531,
//!   3802) ⇒ one appended word, deferred.
//! - `-handler`: `namespace eval :: $state(-handler) [list $sock $token]`
//!   (9.0.4:3921) ⇒ two, deferred.
//! - `-progress` / `-queryprogress`: `… [list $token $state(totalsize)
//!   $state(currentsize)]` (9.0.4:4086, 4458) and `… [list $token
//!   $state(querylength) $state(queryoffset)]` (9.0.4:3521) ⇒ three each,
//!   deferred.
//!
//! Deferred is the honest answer for all four even though `geturl`
//! *without* `-command` blocks until the transaction completes: the
//! callbacks run from the event loop under `fileevent`, so they are never
//! same-invocation in the sense a `struct::list map` callback is.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// `-query` and `-querychannel` are mutually exclusive — `geturl` raises
/// "Can't combine -query and -querychannel options!" when both appear
/// (9.0.4:1136).
/// `geturl`'s option relations, all three proved against `http.tcl` and
/// `http.n` in the bundled 9.0.4 tree.
///
/// The exclusion is `geturl`'s own hard error (`http.tcl`: *"Can't combine
/// -query and -querychannel options!"*).  The two `RequiresOneOf` rows are
/// the **conditional pairs** `http.n` documents: `-queryprogress` is
/// *"made after each transfer of data to the URL in a POST request (i.e. a
/// call to `::http::geturl` with option `-query` or `-querychannel`)"*, and
/// `-queryblocksize` is *"the block size used when posting query data"* —
/// each is inert without a query body, so supplying one alone is a mistake
/// the caller wants to know about rather than a runtime error.
const OPTION_RELATIONS: &[OptionRelation] = &[
    OptionRelation::conflict(&[
        OptionTerm::Option("-query"),
        OptionTerm::Option("-querychannel"),
    ]),
    OptionRelation {
        kind: RelationKind::RequiresOneOf,
        subject: Some(OptionTerm::Option("-queryprogress")),
        terms: &[
            OptionTerm::Option("-query"),
            OptionTerm::Option("-querychannel"),
        ],
        ..OptionRelation::DEFAULT
    },
    OptionRelation {
        kind: RelationKind::RequiresOneOf,
        subject: Some(OptionTerm::Option("-queryblocksize")),
        terms: &[
            OptionTerm::Option("-query"),
            OptionTerm::Option("-querychannel"),
        ],
        ..OptionRelation::DEFAULT
    },
];

/// Every option `geturl` accepts, with the release each appeared in and
/// the callback shape each executable option really has.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-binary",
        value: OptionValue::boolean(),
        detail: "Force the response body to be treated as binary rather than text.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-blocksize",
        value: OptionValue::value("bytes"),
        detail: "Block size used when reading the response (default 8192).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-channel",
        value: OptionValue::channel("chan"),
        detail: "Write the response body to this channel instead of buffering it in the token.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(1)),
        detail: "Completion callback, invoked from the event loop with the token appended; supplying it makes geturl return immediately.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-guesstype",
        value: OptionValue::boolean(),
        detail: "Guess the response content type when the server does not supply a usable one.",
        // Absent from http 2.9.8 (tcl8.6.16); present in 2.10.2 (tcl9.0.4).
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-handler",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Body-reading callback, invoked with (socket token) appended; its return value is the byte count consumed.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-headers",
        // Verified as a `list` by geturl's own type table, which also
        // rejects an odd element count — it is a key/value list, i.e. a
        // dict, and it is the credential-bearing option (W310).
        value: OptionValue::value("dict"),
        detail: "Extra request headers as a key/value list; the credential-bearing option (Authorization, Cookie, …).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-keepalive",
        value: OptionValue::boolean(),
        detail: "Reuse a persistent connection for this request.",
        surface: Some(SpecSurface::TCL85_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-method",
        value: OptionValue::value("verb"),
        detail: "HTTP method to use instead of the GET/POST default.",
        surface: Some(SpecSurface::TCL85_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-myaddr",
        value: OptionValue::value("address"),
        detail: "Local address to bind the outgoing socket to.",
        surface: Some(SpecSurface::TCL85_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-progress",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(3)),
        detail: "Download-progress callback, invoked with (token totalsize currentsize) appended.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-protocol",
        value: OptionValue::value("version"),
        detail: "HTTP protocol version to advertise (default 1.1).",
        surface: Some(SpecSurface::TCL85_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-query",
        value: OptionValue::value("query"),
        detail: "Request body to POST; mutually exclusive with -querychannel.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-queryblocksize",
        value: OptionValue::value("bytes"),
        detail: "Block size used when writing the request body (default 8192).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-querychannel",
        value: OptionValue::channel("chan"),
        detail: "Read the request body from this channel; mutually exclusive with -query.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-queryprogress",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(3)),
        detail: "Upload-progress callback, invoked with (token querylength queryoffset) appended.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-strict",
        value: OptionValue::boolean(),
        detail: "Reject URLs that are not strictly RFC 3986 conformant.",
        surface: Some(SpecSurface::TCL85_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value("milliseconds"),
        detail: "Abort the transaction after this many milliseconds (0 = no timeout).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("mimetype"),
        detail: "Content-Type of the request body (default application/x-www-form-urlencoded).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-validate",
        value: OptionValue::boolean(),
        detail: "Send a HEAD-style validation request rather than fetching the body.",
        ..OptionSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::geturl",
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Retrieve a URL — the primary command for the http package.",
            synopsis: &["http::geturl url ?options?"],
            snippet: "Retrieves the resource at *url* and returns a token that can be passed to the other ``http::`` commands.  Options include ``-query``, ``-headers``, ``-handler``, ``-command``, ``-timeout``, ``-type``, ``-method``, ``-keepalive`` and more.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        // `url` (arg 0) is a network-address arg — SSRF sink
        // (T104); `-headers` can carry credentials.
        taint_network_sink_args: Some(&[0]),
        credential_options: const { &["-headers"] },
        options: OPTIONS,
        option_relations: OPTION_RELATIONS,
        // `geturl` takes the URL positionally and then loops
        // `foreach {flag value} $args` over everything after it, so its
        // options are not a leading run.
        option_placement: OptionPlacement::Anywhere,
        required_package: Some("http"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SpecSurface};
    use super::*;

    /// The four callbacks and their measured appended arities — the half
    /// of `geturl` that was invisible before P5.
    #[test]
    fn the_callbacks_carry_their_measured_appended_arity() {
        let spec = spec();
        let arity = |name: &str| {
            spec.options
                .iter()
                .find(|option| option.name == name)
                .map(OptionSpec::value_appended_arity)
        };
        assert_eq!(arity("-command"), Some(AppendedArity::Exactly(1)));
        assert_eq!(arity("-handler"), Some(AppendedArity::Exactly(2)));
        assert_eq!(arity("-progress"), Some(AppendedArity::Exactly(3)));
        assert_eq!(arity("-queryprogress"), Some(AppendedArity::Exactly(3)));
        for name in ["-command", "-handler", "-progress", "-queryprogress"] {
            assert_eq!(
                spec.options
                    .iter()
                    .find(|option| option.name == name)
                    .and_then(OptionSpec::value_script_timing),
                Some(ScriptTiming::Deferred),
                "{name} runs from the event loop",
            );
        }
    }

    /// The release deltas the four bundled Tcl trees prove.
    #[test]
    fn the_release_deltas_match_the_bundled_trees() {
        let spec = spec();
        let gate = |name: &str| {
            spec.options
                .iter()
                .find(|option| option.name == name)
                .map(|option| option.surface)
        };
        // The 8.4 base set: no gate of their own.
        for name in ["-binary", "-channel", "-command", "-handler", "-headers"] {
            assert_eq!(gate(name), Some(None), "{name} exists in http 2.5.8");
        }
        // 8.5's http 2.7.13 additions.
        for name in ["-keepalive", "-method", "-myaddr", "-protocol", "-strict"] {
            assert_eq!(
                gate(name),
                Some(Some(SpecSurface::TCL85_PLUS)),
                "{name} arrived with http 2.7.13 (tcl8.5)",
            );
        }
        // 9.0's http 2.10.2 addition.
        assert_eq!(
            gate("-guesstype"),
            Some(Some(SpecSurface::TCL90_PLUS)),
            "-guesstype arrived with http 2.10.2 (tcl9.0)",
        );
    }

    /// The credential and network-sink facts survive the option table,
    /// and `-query`/`-querychannel` keep their documented conflict.
    #[test]
    fn the_security_facts_and_the_conflict_are_declared() {
        let spec = spec();
        assert_eq!(spec.taint_network_sink_args, Some(&[0][..]));
        assert_eq!(spec.credential_options, &["-headers"]);
        assert_eq!(spec.option_relations.len(), 3);
        assert_eq!(
            spec.option_relations[0].describe(),
            "option_conflict {-query -querychannel}",
        );
    }

    /// Exactly the 20 options `geturl`'s own `set options {…}` list names
    /// under 9.0, and no invented ones.
    #[test]
    fn the_option_table_is_the_sources_option_list() {
        let mut names: Vec<&str> = spec().options.iter().map(|option| option.name).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "-binary",
                "-blocksize",
                "-channel",
                "-command",
                "-guesstype",
                "-handler",
                "-headers",
                "-keepalive",
                "-method",
                "-myaddr",
                "-progress",
                "-protocol",
                "-query",
                "-queryblocksize",
                "-querychannel",
                "-queryprogress",
                "-strict",
                "-timeout",
                "-type",
                "-validate",
            ],
        );
    }
}
