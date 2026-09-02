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

//! Ensemble option tables, subcommand resolution, and the ensemble-flavoured
//! `must be …` enumeration — the shared owner both runtimes resolve
//! `namespace ensemble` and ensemble dispatch through (`tclEnsemble.c`).
//!
//! Three facts C keeps in one file and the runtimes used to keep twice each:
//!
//! - the **option tables**. `namespace ensemble create` and `namespace
//!   ensemble configure` do **not** share a table: `create` has `-command`
//!   and no `-namespace`, `configure` has `-namespace` (read-only) and no
//!   `-command` (`ensembleCreateOptions` / `ensembleConfigOptions`,
//!   `tclEnsemble.c:57-71`). Both are resolved with `Tcl_GetIndexFromObj`
//!   flags `0`, so every option abbreviates.
//! - the **subcommand scan**. `NsEnsembleImplementationCmd`
//!   (`tclEnsemble.c:1772-1852`) tries the exact name first and then, unless
//!   `-prefixes 0`, a unique prefix — [`prefix::scan`]'s rule with one
//!   documented divergence, see [`resolve_subcommand`].
//! - the **enumeration wording**. C's ensemble error builder writes a comma
//!   before `or` even for two entries (`bar, or baz`), unlike
//!   `Tcl_GetIndexFromObj`'s `a or b` — [`subcommand_choices`].

use std::cell::{Cell, RefCell};
use std::ops::Range;

use crate::prefix::{OptionTable, Resolution};

/// The stable command-token state behind a `namespace ensemble` command.
///
/// C Tcl attaches an [`EnsembleConfig`](https://www.tcl-lang.org/man/tcl9.0/TclLib/Ensemble.htm)
/// to a `Command` token, not to the command's current table name.  The token
/// survives reconfiguration, rename, and imported invocation; deletion marks
/// that same token dead even when an unknown-handler callback still holds it.
/// Both Rust runtimes use this owner so those lifecycle rules cannot drift.
#[derive(Debug)]
pub struct EnsembleToken<C, N> {
    config: RefCell<C>,
    name: RefCell<N>,
    dead: Cell<bool>,
}

impl<C, N> EnsembleToken<C, N> {
    /// Create a live token with its initial configuration and fully-qualified
    /// command name.
    pub fn new(config: C, name: N) -> Self {
        Self {
            config: RefCell::new(config),
            name: RefCell::new(name),
            dead: Cell::new(false),
        }
    }

    /// Replace the live configuration in place, preserving command identity.
    pub fn configure(&self, config: C) {
        *self.config.borrow_mut() = config;
    }

    /// Move the command token to its new fully-qualified name.
    pub fn rename(&self, name: N) {
        *self.name.borrow_mut() = name;
    }

    /// Mark the command token deleted. This is deliberately irreversible: a
    /// new ensemble created at the same name receives a different token.
    pub fn mark_deleted(&self) {
        self.dead.set(true);
    }

    /// Whether the command token was deleted while an invocation retained it.
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.dead.get()
    }
}

impl<C: Clone, N> EnsembleToken<C, N> {
    /// Snapshot the current configuration without holding a borrow across a
    /// callback that may reconfigure the token.
    #[must_use]
    pub fn config(&self) -> C {
        self.config.borrow().clone()
    }
}

impl<C, N: Clone> EnsembleToken<C, N> {
    /// The token's current fully-qualified name. Imported spellings do not
    /// change it; renaming the source command does.
    #[must_use]
    pub fn name(&self) -> N {
        self.name.borrow().clone()
    }
}

/// Exact Tcl error raised when an ensemble unknown handler deletes the active
/// command token before returning successfully (`EnsembleUnknownCallback`).
pub const UNKNOWN_DELETED_MESSAGE: &str = "unknown subcommand handler deleted its ensemble";

/// Tcl list form of the corresponding `-errorcode`.
pub const UNKNOWN_DELETED_ERROR_CODE: &str = "TCL ENSEMBLE UNKNOWN_DELETED";

/// Word positions for one ensemble invocation.
///
/// `leading_words` is zero for an argv that omits the command word and one for
/// Tcl's ordinary objv layout. Keeping this calculation in the shared ensemble
/// owner prevents the runtimes from disagreeing about which words are
/// `-parameters`, the subcommand, and the subcommand arguments after an
/// `-unknown` callback changes the parameter list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvocationLayout {
    /// The values supplied for `-parameters`.
    pub parameters: Range<usize>,
    /// The subcommand word.
    pub subcommand: usize,
    /// First word forwarded after the subcommand.
    pub arguments: usize,
}

/// Resolve the word layout for an ensemble invocation.
///
/// Returns `None` when the invocation has no subcommand after its parameter
/// words. Callers must recompute the layout from the token's live configuration
/// after an `-unknown` callback because Tcl permits that callback to change
/// `-parameters` before returning a replacement prefix.
#[must_use]
pub fn invocation_layout(
    word_count: usize,
    leading_words: usize,
    parameter_count: usize,
) -> Option<InvocationLayout> {
    let subcommand = leading_words.checked_add(parameter_count)?;
    (subcommand < word_count).then_some(InvocationLayout {
        parameters: leading_words..subcommand,
        subcommand,
        arguments: subcommand + 1,
    })
}

/// `namespace ensemble`'s own subcommand table (`ensembleSubcommands`,
/// `tclEnsemble.c:50`) — resolved with flags `0`, so `namespace ensemble cr`
/// is `create` and the miss reads `bad subcommand "…": must be …`.
pub const SUBCOMMANDS: OptionTable<'static> =
    OptionTable::abbreviating("subcommand", &["configure", "create", "exists"]);

/// `namespace ensemble create`'s option table (`ensembleCreateOptions`,
/// `tclEnsemble.c:57-60`) — note it has **no** `-namespace`: an ensemble is
/// always created over the namespace the command runs in.
pub const CREATE_OPTIONS: OptionTable<'static> = OptionTable::abbreviating(
    "option",
    &[
        "-command",
        "-map",
        "-parameters",
        "-prefixes",
        "-subcommands",
        "-unknown",
    ],
);

/// `namespace ensemble configure`'s option table (`ensembleConfigOptions`,
/// `tclEnsemble.c:66-69`) — `-namespace` is readable but rejected as a
/// *write* with `option -namespace is read-only`, and there is no `-command`.
pub const CONFIG_OPTIONS: OptionTable<'static> = OptionTable::abbreviating(
    "option",
    &[
        "-map",
        "-namespace",
        "-parameters",
        "-prefixes",
        "-subcommands",
        "-unknown",
    ],
);

/// The options `namespace ensemble create` and `configure` have in common —
/// everything that lands in the ensemble's stored configuration. `-command`
/// (create-only, it names the command rather than configuring it) and
/// `-namespace` (configure-only, and read-only there) are the two that do not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedOption {
    /// `-map dict`
    Map,
    /// `-parameters list`
    Parameters,
    /// `-prefixes boolean`
    Prefixes,
    /// `-subcommands list`
    Subcommands,
    /// `-unknown prefix`
    Unknown,
}

/// Enforce C's invariant that every ensemble `-map` implementation is a
/// non-empty command-prefix list. Callers parse and canonicalise their dict
/// through the shared value seam (or a byte-native dict representation), then
/// hand the resulting entries here before storing them.
///
/// # Errors
/// `ensemble subcommand implementations must be non-empty lists` when any
/// target prefix is empty.
pub fn validate_map_targets<K, V>(map: &[(K, Vec<V>)]) -> Result<(), crate::CmdError> {
    if map.iter().any(|(_, prefix)| prefix.is_empty()) {
        Err(crate::CmdError::new(
            "ensemble subcommand implementations must be non-empty lists",
        ))
    } else {
        Ok(())
    }
}

/// A resolved `namespace ensemble create` option (the index into
/// [`CREATE_OPTIONS`], named).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateOption {
    /// `-command name`
    Command,
    /// `-map dict`
    Map,
    /// `-parameters list`
    Parameters,
    /// `-prefixes boolean`
    Prefixes,
    /// `-subcommands list`
    Subcommands,
    /// `-unknown prefix`
    Unknown,
}

impl CreateOption {
    /// Resolve one create-option word (abbreviations included), or the C miss
    /// message bytes (`bad option "-x": must be …`).
    ///
    /// # Errors
    /// `Tcl_GetIndexFromObj`'s bad/ambiguous option message.
    pub fn resolve(word: &[u8]) -> Result<Self, Vec<u8>> {
        Ok(match CREATE_OPTIONS.index_of(word)? {
            0 => Self::Command,
            1 => Self::Map,
            2 => Self::Parameters,
            3 => Self::Prefixes,
            4 => Self::Subcommands,
            _ => Self::Unknown,
        })
    }

    /// The shared configuration option this is, or `None` for `-command`,
    /// which names the ensemble command rather than configuring it.
    #[must_use]
    pub const fn shared(self) -> Option<SharedOption> {
        match self {
            Self::Command => None,
            Self::Map => Some(SharedOption::Map),
            Self::Parameters => Some(SharedOption::Parameters),
            Self::Prefixes => Some(SharedOption::Prefixes),
            Self::Subcommands => Some(SharedOption::Subcommands),
            Self::Unknown => Some(SharedOption::Unknown),
        }
    }
}

/// A resolved `namespace ensemble configure` option (the index into
/// [`CONFIG_OPTIONS`], named).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOption {
    /// `-map dict`
    Map,
    /// `-namespace` — readable only.
    Namespace,
    /// `-parameters list`
    Parameters,
    /// `-prefixes boolean`
    Prefixes,
    /// `-subcommands list`
    Subcommands,
    /// `-unknown prefix`
    Unknown,
}

impl ConfigOption {
    /// Resolve one configure-option word (abbreviations included), or the C
    /// miss message bytes.
    ///
    /// # Errors
    /// `Tcl_GetIndexFromObj`'s bad/ambiguous option message.
    pub fn resolve(word: &[u8]) -> Result<Self, Vec<u8>> {
        Ok(match CONFIG_OPTIONS.index_of(word)? {
            0 => Self::Map,
            1 => Self::Namespace,
            2 => Self::Parameters,
            3 => Self::Prefixes,
            4 => Self::Subcommands,
            _ => Self::Unknown,
        })
    }

    /// The shared configuration option this is, or `None` for `-namespace`,
    /// which C rejects as a *write* (`option -namespace is read-only`).
    #[must_use]
    pub const fn shared(self) -> Option<SharedOption> {
        match self {
            Self::Namespace => None,
            Self::Map => Some(SharedOption::Map),
            Self::Parameters => Some(SharedOption::Parameters),
            Self::Prefixes => Some(SharedOption::Prefixes),
            Self::Subcommands => Some(SharedOption::Subcommands),
            Self::Unknown => Some(SharedOption::Unknown),
        }
    }

    /// The canonical spelling, for the `configure` query result's key order.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Map => "-map",
            Self::Namespace => "-namespace",
            Self::Parameters => "-parameters",
            Self::Prefixes => "-prefixes",
            Self::Subcommands => "-subcommands",
            Self::Unknown => "-unknown",
        }
    }

    /// Every configure option, in C's table (and query-dict) order.
    #[must_use]
    pub const fn all() -> [Self; 6] {
        [
            Self::Map,
            Self::Namespace,
            Self::Parameters,
            Self::Prefixes,
            Self::Subcommands,
            Self::Unknown,
        ]
    }
}

/// Resolve an ensemble subcommand word against the (sorted) subcommand set:
/// the index of the matched entry, or `None` for unknown/ambiguous.
///
/// This is [`prefix::scan`](crate::prefix::scan)'s exact-then-unique-prefix
/// rule with the one place `NsEnsembleImplementationCmd` differs from
/// `Tcl_GetIndexFromObjStruct`: the **empty word**. `Tcl_GetIndexFromObj`
/// forces the error path for an empty key, but the ensemble scan is a
/// `strncmp` over the word's length, so an empty subcommand prefixes every
/// entry and *resolves* against a one-entry table (tclsh 8.6.16/9.0.4:
/// an ensemble with the single subcommand `go` runs `go` for `$ens {}`).
#[must_use]
pub fn resolve_subcommand<T: AsRef<[u8]>>(subs: &[T], sub: &[u8], prefixes: bool) -> Option<usize> {
    if let Some(i) = subs.iter().position(|s| s.as_ref() == sub) {
        return Some(i); // the exact-name hash probe C does first
    }
    if !prefixes {
        return None;
    }
    if sub.is_empty() {
        // C's `strncmp(sub, entry, 0) == 0` matches every entry.
        return (subs.len() == 1).then_some(0);
    }
    match crate::prefix::scan(subs, sub, false) {
        Resolution::Exact(i) | Resolution::UniquePrefix(i) => Some(i),
        Resolution::Ambiguous | Resolution::NoMatch => None,
    }
}

/// The ensemble `must be …` clause. C's ensemble error builder joins with
/// `", "` and prefixes the last entry with `or `, so **two** entries render
/// as `bar, or baz` — unlike [`prefix::choice_list`](crate::prefix::choice_list),
/// which follows `Tcl_GetIndexFromObj`'s `bar or baz`. This is the one
/// documented ensemble wording exception in the shared-utility contract; it
/// lives here so neither runtime re-derives it.
#[must_use]
pub fn subcommand_choices<T: AsRef<[u8]>>(subs: &[T]) -> Vec<u8> {
    let mut out = Vec::new();
    let last = subs.len().saturating_sub(1);
    for (i, s) in subs.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b", ");
            if i == last {
                out.extend_from_slice(b"or ");
            }
        }
        out.extend_from_slice(s.as_ref());
    }
    out
}

/// The full ensemble dispatch miss message for `sub` against `subs`
/// (`NsEnsembleImplementationCmd`, `tclEnsemble.c:1941-1966`):
///
/// - an ensemble whose namespace exports nothing at all gets
///   `unknown subcommand "x": namespace ::ns does not export any commands`
///   (`ns_fqn` supplies `::ns`);
/// - otherwise `unknown or ambiguous subcommand "x": must be …` when
///   `-prefixes` is on, and `unknown subcommand "x": must be …` when it is
///   off (an exact-only ensemble can never be *ambiguous*).
#[must_use]
pub fn unknown_subcommand_message<T: AsRef<[u8]>>(
    subs: &[T],
    sub: &[u8],
    prefixes: bool,
    ns_fqn: &[u8],
) -> Vec<u8> {
    if subs.is_empty() {
        let mut m = b"unknown subcommand \"".to_vec();
        m.extend_from_slice(sub);
        m.extend_from_slice(b"\": namespace ");
        m.extend_from_slice(ns_fqn);
        m.extend_from_slice(b" does not export any commands");
        return m;
    }
    let mut m = if prefixes {
        b"unknown or ambiguous subcommand \"".to_vec()
    } else {
        b"unknown subcommand \"".to_vec()
    };
    m.extend_from_slice(sub);
    m.extend_from_slice(b"\": must be ");
    m.extend_from_slice(&subcommand_choices(subs));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&[u8]]) -> Vec<Vec<u8>> {
        items.iter().map(|s| s.to_vec()).collect()
    }

    #[test]
    fn create_table_has_no_namespace_option() {
        // tclsh 8.6.16/9.0.4: `namespace ensemble create -namespace ::x` →
        // bad option "-namespace": must be -command, -map, -parameters,
        // -prefixes, -subcommands, or -unknown.
        assert_eq!(
            String::from_utf8(CreateOption::resolve(b"-namespace").unwrap_err()).unwrap(),
            "bad option \"-namespace\": must be -command, -map, -parameters, -prefixes, -subcommands, or -unknown"
        );
        assert_eq!(
            CreateOption::resolve(b"-parameters"),
            Ok(CreateOption::Parameters)
        );
        // Abbreviations come free from flags-0 `Tcl_GetIndexFromObj`.
        assert_eq!(CreateOption::resolve(b"-comm"), Ok(CreateOption::Command));
        assert_eq!(
            CreateOption::resolve(b"-sub"),
            Ok(CreateOption::Subcommands)
        );
        // `-p` prefixes both -parameters and -prefixes.
        assert_eq!(
            String::from_utf8(CreateOption::resolve(b"-p").unwrap_err()).unwrap(),
            "ambiguous option \"-p\": must be -command, -map, -parameters, -prefixes, -subcommands, or -unknown"
        );
    }

    #[test]
    fn config_table_has_namespace_but_no_command() {
        assert_eq!(
            ConfigOption::resolve(b"-namespace"),
            Ok(ConfigOption::Namespace)
        );
        assert_eq!(
            ConfigOption::resolve(b"-sub"),
            Ok(ConfigOption::Subcommands)
        );
        assert_eq!(
            String::from_utf8(ConfigOption::resolve(b"-command").unwrap_err()).unwrap(),
            "bad option \"-command\": must be -map, -namespace, -parameters, -prefixes, -subcommands, or -unknown"
        );
    }

    #[test]
    fn ensemble_subcommand_scan_matches_c() {
        let subs = v(&[b"bar", b"baz"]);
        assert_eq!(resolve_subcommand(&subs, b"bar", true), Some(0));
        assert_eq!(resolve_subcommand(&subs, b"ba", true), None);
        assert_eq!(resolve_subcommand(&subs, b"ba", false), None);
        assert_eq!(resolve_subcommand(&subs, b"bar", false), Some(0));
        // An exact match beats a longer prefix-sharing entry.
        let subs2 = v(&[b"bar", b"barx"]);
        assert_eq!(resolve_subcommand(&subs2, b"bar", true), Some(0));
        // The empty word: C's zero-length strncmp prefixes every entry, so a
        // one-entry table resolves and a two-entry table is ambiguous.
        assert_eq!(resolve_subcommand(&v(&[b"go"]), b"", true), Some(0));
        assert_eq!(resolve_subcommand(&subs, b"", true), None);
        assert_eq!(resolve_subcommand::<Vec<u8>>(&[], b"", true), None);
    }

    #[test]
    fn invocation_layout_supports_both_runtime_argv_shapes() {
        assert_eq!(
            invocation_layout(4, 0, 2),
            Some(InvocationLayout {
                parameters: 0..2,
                subcommand: 2,
                arguments: 3,
            })
        );
        assert_eq!(
            invocation_layout(5, 1, 2),
            Some(InvocationLayout {
                parameters: 1..3,
                subcommand: 3,
                arguments: 4,
            })
        );
        assert_eq!(invocation_layout(2, 0, 2), None);
        assert_eq!(invocation_layout(3, 1, 2), None);
        assert_eq!(invocation_layout(usize::MAX, usize::MAX, 1), None);
    }

    #[test]
    fn enumeration_keeps_the_comma_before_or_for_two() {
        assert_eq!(subcommand_choices(&v(&[b"go"])), b"go");
        assert_eq!(subcommand_choices(&v(&[b"bar", b"baz"])), b"bar, or baz");
        assert_eq!(subcommand_choices(&v(&[b"a", b"b", b"c"])), b"a, b, or c");
        // The `Tcl_GetIndexFromObj` join differs for two — kept apart on purpose.
        assert_eq!(crate::prefix::choice_list(&["bar", "baz"]), "bar or baz");
    }

    #[test]
    fn miss_messages_match_tclsh() {
        let subs = v(&[b"bar", b"baz"]);
        assert_eq!(
            unknown_subcommand_message(&subs, b"ba", true, b"::e7"),
            b"unknown or ambiguous subcommand \"ba\": must be bar, or baz".to_vec()
        );
        assert_eq!(
            unknown_subcommand_message(&subs, b"ba", false, b"::e7"),
            b"unknown subcommand \"ba\": must be bar, or baz".to_vec()
        );
        assert_eq!(
            unknown_subcommand_message::<Vec<u8>>(&[], b"zz", true, b"::e8"),
            b"unknown subcommand \"zz\": namespace ::e8 does not export any commands".to_vec()
        );
    }
}
