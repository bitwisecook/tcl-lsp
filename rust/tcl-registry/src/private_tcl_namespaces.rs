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

//! Private, undocumented `::tcl::` implementation namespaces (issue #988).
//!
//! Real Tcl backs a handful of built-in ensemble commands with a private
//! sub-namespace under `::tcl::` — `dict create` is implemented by
//! `::tcl::dict::create`, `string totitle` by `::tcl::string::totitle`, and
//! so on. These namespaces are undocumented internals: calling into them
//! directly works (confirmed against tclsh 8.6/9.0 — see the `analyser`
//! diagnostic that consumes this table), but it is never a documented or
//! supported way to write Tcl, and membership churns release to release
//! (confirmed against the 8.4.20/8.6.16/9.0.4 C sources and live tclsh — for
//! example `::tcl::zlib` does not exist on every 8.6 build).
//!
//! Because of that churn, and because none of this is a documented
//! contract, this table does not model per-version availability the way an
//! ordinary [`crate::spec::CommandSpec`] does. It exists purely to drive a
//! single generic "this is a private implementation namespace, use the
//! public command instead" diagnostic.
//!
//! What it *does* model, per namespace, is how far the "private" claim
//! actually reaches — see [`TailRule`]. `::tcl::chan` is the namespace
//! where that matters: tcllib publishes fourteen **public, documented**
//! packages into it (`tcl::chan::memchan`, `tcl::chan::fifo`, …, the whole
//! of `tcllib-2.0/modules/virtchannel_base`), so "anything under
//! `::tcl::chan` is private" is simply false there.

use tcl_dialect::model::{SurfaceQuery, surface_admits};

/// How far a private namespace's claim over its members reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailRule {
    /// Every member of the namespace is core implementation detail:
    /// `::tcl::clock::GetSystemTimeZone` is as private as
    /// `::tcl::clock::format`, and nothing public is published there.
    Whole,
    /// The namespace is *shared* — Tcl's own ensemble implementation lives
    /// there, but public third-party packages publish commands alongside
    /// it. Only a tail that is a genuine subcommand of the public ensemble
    /// (for the active dialect) is the private backing; every other tail is
    /// somebody else's public command and is left alone.
    EnsembleSubcommandsOnly,
}

/// One private `::tcl::` ensemble-backing sub-namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateTclNamespace {
    /// The bare segment under `::tcl::`, which doubles as the public
    /// ensemble command it backs (`dict` backs both `::tcl::dict::*` and
    /// the public `dict` command).
    pub namespace: &'static str,
    /// How far this namespace's privacy claim reaches.
    pub tail_rule: TailRule,
}

/// The 11 private `::tcl::` ensemble-backing sub-namespaces, confirmed
/// against real Tcl (8.4.20 through 9.1b0 C source, and live tclsh 8.6/9.0)
/// while investigating issue #988.
///
/// Every entry but `chan` is [`TailRule::Whole`]: nothing public is
/// published under `::tcl::dict`, `::tcl::string`, `::tcl::clock`, and the
/// rest, so a call to any member of them is a call into Tcl's own
/// internals. `chan` is [`TailRule::EnsembleSubcommandsOnly`] because
/// tcllib's `virtchannel_base` / `virtchannel_transform` modules publish
/// public packages there (`tcl::chan::memchan`, `tcl::chan::cat`,
/// `tcl::chan::fifo`, …) — flagging those would contradict the registry,
/// which carries a real [`crate::spec::CommandSpec`] for each of them.
pub const PRIVATE_TCL_NAMESPACES: &[PrivateTclNamespace] = &[
    PrivateTclNamespace {
        namespace: "dict",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "string",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "array",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "file",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "info",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "clock",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "binary",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "namespace",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "encoding",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "zlib",
        tail_rule: TailRule::Whole,
    },
    PrivateTclNamespace {
        namespace: "chan",
        tail_rule: TailRule::EnsembleSubcommandsOnly,
    },
];

/// A direct call into one of [`PRIVATE_TCL_NAMESPACES`], classified from a
/// command head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateTclNamespaceCall {
    /// The public ensemble command to use instead (e.g. `"dict"`).
    pub public_command: &'static str,
    /// The tail written after the namespace (`create` for
    /// `::tcl::dict::create`, `a::b` for `::tcl::dict::a::b`).
    pub tail: String,
    /// The concrete replacement call — the public command followed by the
    /// same tail (`"dict create"`) — **only** when that tail is a genuine
    /// subcommand of the public ensemble under the active dialect, i.e.
    /// when the rewrite is a legal edit. `None` for a tail that is private
    /// machinery with no public spelling (`::tcl::clock::GetSystemTimeZone`,
    /// `::tcl::dict::mycustom`): the call is still flagged, but suggesting
    /// `clock GetSystemTimeZone` would corrupt the code.
    pub suggestion: Option<String>,
}

/// Classify `cmd_name` — the literal command-head text as written (e.g.
/// `::tcl::dict::create` or `tcl::dict::create`) — as a direct call into one
/// of the 11 private `::tcl::` namespaces, resolving the tail against
/// `registry` for `dialect`.
///
/// Accepts both the fully qualified (`::tcl::…`) and bare (`tcl::…`)
/// spellings. Matches the namespace segment **exactly**, not as a
/// substring, so a near-miss (`::tcl::dictionary::foo`) or a user's own
/// namespace nested under `tcl::` (`::tcl::mycustom::foo`) is never
/// flagged. Public, documented namespaces that also live directly under
/// `tcl::` (`tcl::mathop::+`, `tcl::mathfunc::sin`, `tcl::prefix`) are
/// likewise unaffected, since none of them appear in
/// [`PRIVATE_TCL_NAMESPACES`].
///
/// Returns `None` when:
/// * the namespace segment has no tail at all (`::tcl::dict` alone) — there
///   is nothing to suggest in its place;
/// * `cmd_name` is itself a registered command for `dialect` that the
///   registry does *not* mark as an ensemble's private backing — it knows
///   `tcl::chan::memchan` as a real tcllib package command, so that is
///   public by construction, whatever namespace it happens to live in,
///   while `::tcl::dict::create`'s standalone spec carries
///   [`crate::spec::CommandSpec::implementation_namespace`] and stays
///   private;
/// * the namespace is [`TailRule::EnsembleSubcommandsOnly`] and the tail is
///   not one of the public ensemble's subcommands.
#[must_use]
pub fn classify_private_tcl_namespace_call(
    cmd_name: &str,
    registry: &crate::CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<PrivateTclNamespaceCall> {
    let rest = cmd_name.strip_prefix("::").unwrap_or(cmd_name);
    let rest = rest.strip_prefix("tcl::")?;
    let (namespace, tail) = rest.split_once("::")?;
    if tail.is_empty() {
        return None;
    }
    let entry = PRIVATE_TCL_NAMESPACES
        .iter()
        .find(|ns| ns.namespace == namespace)?;
    // A command the registry knows by this exact qualified name is a real,
    // documented command — unless the registry itself marks it as an
    // ensemble's private backing.  `dict`'s standalone `::tcl::dict::*`
    // spellings are registered (so hover and arity work on them) and carry
    // [`crate::spec::CommandSpec::implementation_namespace`]; tcllib's
    // `tcl::chan::memchan` and friends are registered without one.
    if registry
        .get_for_surface(cmd_name, dialect)
        .is_some_and(|spec| spec.implementation_namespace.is_none())
    {
        return None;
    }
    let public_command = entry.namespace;
    let is_ensemble_subcommand = registry
        .get_for_surface(public_command, dialect)
        .and_then(|spec| spec.subcommand(tail))
        .is_some_and(|sub| sub.surface.is_none_or(|d| surface_admits(d, dialect.as_ref())));
    if entry.tail_rule == TailRule::EnsembleSubcommandsOnly && !is_ensemble_subcommand {
        return None;
    }
    Some(PrivateTclNamespaceCall {
        public_command,
        tail: tail.to_string(),
        suggestion: is_ensemble_subcommand.then(|| format!("{public_command} {tail}")),
    })
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SurfaceQuery, Family};
    use tcl_dialect::model::{SpecSurface};
    use super::*;

    fn classify(cmd: &str) -> Option<PrivateTclNamespaceCall> {
        let registry = crate::model::ingress::static_context_for("tcl9.0").commands();
        classify_private_tcl_namespace_call(cmd, registry, Some(SurfaceQuery::core(Family::Tcl, "9.0")))
    }

    #[test]
    fn qualified_and_bare_spellings_both_classify() {
        for cmd in ["::tcl::dict::create", "tcl::dict::create"] {
            let call = classify(cmd).unwrap();
            assert_eq!(call.public_command, "dict");
            assert_eq!(call.suggestion.as_deref(), Some("dict create"));
        }
    }

    #[test]
    fn whole_namespace_flags_a_non_subcommand_tail_without_a_suggestion() {
        // `GetSystemTimeZone` is real private `::tcl::clock` machinery but
        // no `clock` subcommand, so there is nothing safe to suggest.
        let call = classify("::tcl::clock::GetSystemTimeZone").unwrap();
        assert_eq!(call.public_command, "clock");
        assert_eq!(call.suggestion, None);
        // A multi-level tail likewise never yields a `dict a::b` suggestion.
        assert_eq!(classify("::tcl::dict::a::b").unwrap().suggestion, None);
        // Nor does a subcommand name the user invented.
        assert_eq!(classify("::tcl::dict::mycustom").unwrap().suggestion, None);
    }

    #[test]
    fn every_whole_namespace_classifies() {
        for ns in PRIVATE_TCL_NAMESPACES
            .iter()
            .filter(|n| n.tail_rule == TailRule::Whole)
        {
            let cmd = format!("::tcl::{}::sub", ns.namespace);
            let call = classify(&cmd).unwrap_or_else(|| panic!("expected {cmd} to classify"));
            assert_eq!(call.public_command, ns.namespace);
        }
    }

    #[test]
    fn public_tcllib_channel_packages_are_never_private() {
        // The whole `tcl::chan::*` public surface (tcllib
        // `modules/virtchannel_base`) must stay clear of W143.
        for cmd in [
            "tcl::chan::memchan",
            "::tcl::chan::memchan",
            "tcl::chan::fifo",
            "tcl::chan::cat",
            "tcl::chan::null",
            "tcl::chan::random",
            "tcl::chan::string",
            "tcl::chan::variable",
            "tcl::chan::zero",
        ] {
            assert_eq!(classify(cmd), None, "{cmd} is a public tcllib package");
        }
        // An unregistered name under the shared namespace is not assumed
        // private either — only a genuine `chan` subcommand is.
        assert_eq!(classify("tcl::chan::somebodyelses"), None);
    }

    #[test]
    fn genuine_chan_ensemble_backing_still_classifies() {
        let call = classify("::tcl::chan::truncate").unwrap();
        assert_eq!(call.public_command, "chan");
        assert_eq!(call.suggestion.as_deref(), Some("chan truncate"));
    }

    #[test]
    fn near_miss_namespace_not_flagged() {
        assert_eq!(classify("::tcl::dictionary::foo"), None);
    }

    #[test]
    fn users_own_tcl_scoped_namespace_not_flagged() {
        assert_eq!(classify("::tcl::mycustom::foo"), None);
    }

    #[test]
    fn public_documented_tcl_namespaces_not_flagged() {
        for cmd in ["::tcl::mathop::+", "tcl::mathop::+"] {
            assert_eq!(classify(cmd), None);
        }
        for cmd in ["::tcl::mathfunc::sin", "tcl::mathfunc::sin"] {
            assert_eq!(classify(cmd), None);
        }
        // `tcl::prefix` is a real, public, documented command (Tcl 8.6+),
        // invoked as `tcl::prefix match ...` — a single command word with an
        // ordinary subcommand argument, not a `::`-qualified private
        // namespace member, so it has no `::` tail to classify at all.
        for cmd in ["tcl::prefix", "::tcl::prefix"] {
            assert_eq!(classify(cmd), None);
        }
    }

    #[test]
    fn ordinary_public_command_not_flagged() {
        assert_eq!(classify("dict"), None);
        assert_eq!(classify("dict create"), None);
    }

    #[test]
    fn bare_namespace_with_no_tail_not_flagged() {
        assert_eq!(classify("::tcl::dict"), None);
        assert_eq!(classify("::tcl::dict::"), None);
    }
}
