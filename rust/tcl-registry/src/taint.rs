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

//! Registry-side taint metadata queries.
//!
//! Owns the small subcommand-shaped facts (`chan gets`, `chan read`,
//! `encoding convertfrom`) and the iRules namespace-prefix table that
//! used to live as hardcoded lists inside the compiler's taint
//! analyser. The compiler's `tcl_compiler::taint` module now asks
//! the registry "is this call a source / sink / sanitiser?" rather
//! than maintaining its own command-name set.

use crate::dialects::DialectSet;
use crate::registry::CommandRegistry;
use crate::traits::Traits;
use crate::types::TclType;
use bitflags::bitflags;
use tcl_core_types::DiagCode;

bitflags! {
    /// Properties carried by a tainted value — the taint *colour* lattice.
    ///
    /// Colours compose with
    /// `|`; the lattice *join* of two colours is their intersection
    /// (`&`): a property only survives a control-flow merge when every
    /// incoming path proves it.
    ///
    /// This is the registry-owned definition of the colour set so spec
    /// data (`taint_transform`, `taint_double_encode_colour`,
    /// `taint_sink_safe_colour`) can name colours, and the consumer
    /// (`tcl_compiler::taint`) re-exports it rather than maintaining a
    /// parallel copy.
    ///
    /// Bit layout matches the existing `tcl_compiler::taint::TaintColour`
    /// for bits 0..=14; `PATH_JOINED` (`file join` output) and `CHANNEL`
    /// (I/O channel handle) extend it to the full colour set.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TaintColour: u32 {
        /// Value is attacker-controlled.
        const TAINTED            = 1 << 0;
        /// Always starts with `/` (`HTTP::uri`, `HTTP::path`).
        const PATH_PREFIXED      = 1 << 1;
        /// Provably starts with a non-`-` literal.
        const NON_DASH_PREFIXED  = 1 << 2;
        /// Proven to contain no CR/LF characters.
        const CRLF_FREE          = 1 << 3;
        /// Token-safe atom (no shell metachar splitting).
        const SHELL_ATOM         = 1 << 4;
        /// Canonical Tcl list representation.
        const LIST_CANONICAL     = 1 << 5;
        /// Regex-escaped literal payload.
        const REGEX_LITERAL      = 1 << 6;
        /// Path has been normalised (no raw traversal form).
        const PATH_NORMALISED    = 1 << 7;
        /// Normalised path verified within an intended directory.
        const PATH_BOUNDED       = 1 << 8;
        /// Valid HTTP header-token charset.
        const HEADER_TOKEN_SAFE  = 1 << 9;
        /// HTML-escaped text context.
        const HTML_ESCAPED       = 1 << 10;
        /// URL-encoded text context.
        const URL_ENCODED        = 1 << 11;
        /// IPv4 or IPv6 address (digits, dots, colons).
        const IP_ADDRESS         = 1 << 12;
        /// Integer 0-65535.
        const PORT               = 1 << 13;
        /// Fully qualified domain name.
        const FQDN               = 1 << 14;
        /// Assembled via `[file join]` (portable, not canonicalised).
        const PATH_JOINED        = 1 << 15;
        /// I/O channel handle (`open`, `socket`, `chan create`, `HSL::open`).
        const CHANNEL            = 1 << 16;
    }
}

/// Constraint on a setter-form argument — the registry-driven
/// replacement for the hardcoded `SETTER_CONSTRAINTS` table in
/// `tcl_compiler::taint`.
///
/// Attached to a command via
/// [`crate::CommandSpec::setter_constraints`]; the consumer's IRULE3101
/// check reads the table from the registry instead of its 2-entry
/// hardcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetterConstraint {
    /// Which argument the constraint applies to (0-based after the
    /// command name).
    pub arg_index: u8,
    /// Literal prefix the argument must start with (e.g. `"/"`).
    pub required_prefix: &'static str,
    /// Diagnostic code emitted on violation (e.g. `"IRULE3101"`).
    pub code: DiagCode,
    /// Human-readable explanation for the diagnostic.
    pub message: &'static str,
}

/// Namespaces whose commands return attacker-controlled data when
/// invoked under the iRules dialect.
///
/// Keeping the prefix table here means any consumer (LSP feature
/// providers, future native server, alternate diagnostics) sees the
/// same iRules source classification without duplicating the table.
pub const IRULES_TAINT_SOURCE_PREFIXES: &[&str] = &[
    "HTTP::", "URI::", "IP::", "TCP::", "UDP::", "SSL::", "STREAM::",
];

/// Return `true` when invoking `command` with `args` under `dialect`
/// produces attacker-controlled data.
///
/// Sources are identified by:
///
/// * the [`Traits::TAINT_SOURCE`] flag on the matched
///   [`crate::CommandSpec`] (pure trait dispatch — `gets`, `read`,
///   `exec`, `socket`);
/// * the [`Traits::UNNORMALISED_HTTP_GETTER`] flag (registry-driven
///   HTTP getter);
/// * the [`Traits::TAINT_SOURCE`] flag on the matched
///   [`crate::SubCommand`], for subcommand-shaped sources such as
///   `chan gets` / `chan read` / `encoding convertfrom`; and
/// * the registry's dialect-agnostic taint-source index
///   ([`CommandRegistry::taint_source`]) — the iRules namespace getters
///   (`HTTP::path`, `IP::client_addr`, …), each declaring its source
///   colour on its own [`crate::CommandSpec::taint_source`]. The index is
///   global, so these fire
///   in every dialect (even `tcl8.6`).
#[must_use]
pub fn is_taint_source(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: DialectSet,
) -> bool {
    let _ = dialect;
    if let Some(spec) = registry.get(command) {
        if spec
            .traits
            .intersects(Traits::TAINT_SOURCE | Traits::UNNORMALISED_HTTP_GETTER)
        {
            return true;
        }
        // Tcl ensemble dispatch accepts a unique prefix (`chan g` ⇒ `gets`,
        // `encoding convertf` ⇒ `convertfrom`), so a source subcommand must be
        // resolved prefix-aware or an abbreviation dodges the classification —
        // a taint-source false negative. Dialect-agnostic:
        // classifying a source in *every* dialect only ever catches more (the
        // safe direction for a security source), matching the prior behaviour.
        if let Some(sub_name) = args.first().copied()
            && let Some(sub) = spec.resolve_subcommand(sub_name)
            && sub.traits.contains(Traits::TAINT_SOURCE)
        {
            return true;
        }
    }
    registry.taint_source(command).is_some()
}

/// The taint colour a source `command` (with `args`) stamps on its
/// return value, augmented with derived safety properties, or `None`
/// when the call is not a taint source.
///
/// The base colour comes from
/// the command's own [`crate::CommandSpec::taint_source`] (surfaced
/// dialect-agnostically via [`CommandRegistry::taint_source`]); a
/// trait-detected source with no index entry (`gets`, `read`, …) is plain
/// `TAINTED`. The indexed colour is the getter-form result, so its
/// non-`TAINTED` bits apply only when `args` is empty (the
/// path/IP/port/FQDN getters are keyed on `Arity(0, 0)`).
#[must_use]
pub fn taint_source_colour(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: DialectSet,
) -> Option<TaintColour> {
    if !is_taint_source(registry, command, args, dialect) {
        return None;
    }
    let indexed = registry
        .taint_source(command)
        .unwrap_or(TaintColour::TAINTED);
    // The path/IP/port/FQDN colours describe the getter (0-arg) form only.
    let base = if args.is_empty() {
        indexed
    } else {
        TaintColour::TAINTED
    };
    Some(augment_source_colours(base | TaintColour::TAINTED))
}

/// Add the conservative derived properties a source colour implies.
/// A path-prefixed value also proves `NON_DASH_PREFIXED`; an IP / port /
/// FQDN value proves `NON_DASH_PREFIXED`, `CRLF_FREE`, and `SHELL_ATOM`.
#[must_use]
pub fn augment_source_colours(colour: TaintColour) -> TaintColour {
    let mut out = colour;
    if out.contains(TaintColour::PATH_PREFIXED) {
        out |= TaintColour::NON_DASH_PREFIXED;
    }
    if out.intersects(TaintColour::IP_ADDRESS | TaintColour::PORT | TaintColour::FQDN) {
        out |= TaintColour::NON_DASH_PREFIXED | TaintColour::CRLF_FREE | TaintColour::SHELL_ATOM;
    }
    out
}

/// Return `true` when `command` carries the iRules data-getter trait
/// or starts with one of the [`IRULES_TAINT_SOURCE_PREFIXES`]
/// namespaces. The prefix fallback covers iRules commands that are
/// registered without the explicit trait.
#[must_use]
pub fn is_irules_data_getter(registry: &CommandRegistry, command: &str) -> bool {
    if let Some(spec) = registry.get(command)
        && spec.traits.contains(Traits::IRULES_DATA_GETTER)
    {
        return true;
    }
    IRULES_TAINT_SOURCE_PREFIXES
        .iter()
        .any(|p| command.starts_with(p))
}

/// Return `true` when `command` (with optional subcommand in `args`)
/// is a sanitiser — its return value is a fixed numeric type that
/// cannot carry taint through it.
///
/// Subcommand specs are checked first so `string length` and
/// `string is integer` register as sanitisers even though the
/// top-level `string` command has no return type.
#[must_use]
pub fn is_sanitiser(registry: &CommandRegistry, command: &str, args: &[&str]) -> bool {
    fn is_fixed_numeric(t: Option<TclType>) -> bool {
        matches!(t, Some(TclType::Int | TclType::Boolean))
    }
    let Some(spec) = registry.get(command) else {
        return false;
    };
    // Prefix-aware (`string le` ⇒ `length`) so a legal abbreviation of a
    // sanitiser is still recognised — otherwise a spurious T101 fires where the
    // full spelling is correctly suppressed.
    if let Some(sub_name) = args.first().copied()
        && let Some(sub) = spec.resolve_subcommand(sub_name)
        && is_fixed_numeric(sub.return_type)
    {
        return true;
    }
    is_fixed_numeric(spec.return_type)
}

/// Single-pass taint-sink classification for a `(command, subcommand)`
/// pair.
///
/// The consumer (`tcl_compiler::taint`) reads this instead of
/// re-deriving sink categories from command-name sets. Carries every
/// sink flag the `_sinks._classify_sink` pass needs in one struct so
/// callers do a single registry lookup.
///
/// `dialect`-filtered: pass the active [`DialectSet`]; specs that don't
/// support it are ignored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaintSinkInfo {
    /// Dangerous code-execution sink (`eval`, `expr`, `exec`, `uplevel`,
    /// `subst`, `coroprobe`/`coroinject`) — T100. From
    /// [`Traits::TAINT_SINK`] or [`Traits::EVALUATES_CODE`] (the latter
    /// covers commands that evaluate a runtime command reference without
    /// being a fixed-sink builtin, e.g. `coroprobe`/`coroinject`).
    pub is_code_sink: bool,
    /// Output-sink diagnostic code (T101 / IRULE3001 / IRULE3002 /
    /// IRULE3004), if the matched subcommand qualifies.
    pub output_sink: Option<&'static str>,
    /// Whether the output sink is subcommand-qualified — i.e. its label
    /// should read `"<cmd> <sub>"`.
    pub output_sink_is_subcommand_qualified: bool,
    /// Log-injection sink diagnostic code (IRULE3003).
    pub log_sink: Option<&'static str>,
    /// Network-address sink (SSRF) — T104.
    pub is_network_sink: bool,
    /// Subcommands that evaluate code in another interpreter (T105),
    /// empty when none.
    pub interp_eval_subcommands: &'static [&'static str],
}

/// Classify all taint-sink properties of `command` (with optional
/// `subcommand`) in a single pass, filtered to `dialect`.
///
/// Returns the default (all-clear) [`TaintSinkInfo`] when the command is
/// unknown.
#[must_use]
pub fn classify_taint_sinks(
    registry: &CommandRegistry,
    command: &str,
    subcommand: Option<&str>,
    dialect: DialectSet,
) -> TaintSinkInfo {
    let Some(spec) = registry.get(command) else {
        return TaintSinkInfo::default();
    };
    // An empty `dialect` means "no dialect filter" — a `None`-like
    // short-circuit. Only a
    // concrete dialect set gates dialect-specific specs.
    if !dialect.is_empty() && !spec.supports_dialect(dialect) {
        return TaintSinkInfo::default();
    }

    let mut info = TaintSinkInfo {
        is_code_sink: spec
            .traits
            .intersects(Traits::TAINT_SINK | Traits::EVALUATES_CODE),
        ..TaintSinkInfo::default()
    };

    if let Some(code) = spec.taint_output_sink {
        let subs = spec.taint_output_sink_subcommands;
        // Tcl ensemble dispatch accepts a unique prefix (`HTTP::cookie ins` ⇒
        // `insert`), so the abbreviation is resolved to its canonical
        // subcommand name before the sink-membership test — an exact `contains`
        // let a prefix-abbreviated sink dodge classification.
        // Falls back to the raw word when the subcommand isn't a registered
        // `SubCommand` (nothing to resolve against).
        let canonical = subcommand
            .and_then(|s| spec.resolve_subcommand(s).map(|sub| sub.name))
            .or(subcommand);
        if subs.is_empty() || canonical.is_some_and(|s| subs.contains(&s)) {
            info.output_sink = Some(code);
            info.output_sink_is_subcommand_qualified = !subs.is_empty();
        }
    }
    info.log_sink = spec.taint_log_sink;
    info.is_network_sink = spec.taint_network_sink_args.is_some();
    info.interp_eval_subcommands = spec.taint_interp_eval_subcommands;
    info
}

/// Colour added to a tainted value by `command` (optionally its
/// `subcommand`) — the sanitising-transform query. Subcommand transform
/// takes priority over the command-level one.
///
/// Registry-side counterpart of `runtime.taint_transform_map`.
#[must_use]
pub fn taint_transform(
    registry: &CommandRegistry,
    command: &str,
    subcommand: Option<&str>,
) -> Option<TaintColour> {
    let spec = registry.get(command)?;
    if let Some(sub_name) = subcommand
        && let Some(sub) = spec.resolve_subcommand(sub_name)
        && sub.taint_transform.is_some()
    {
        return sub.taint_transform;
    }
    spec.taint_transform
}

/// Colour whose presence on the input means `command`/`subcommand`
/// would double-encode the value (T106). Subcommand takes priority.
///
/// Registry-side counterpart of `runtime.taint_double_encode_map`.
#[must_use]
pub fn taint_double_encode_colour(
    registry: &CommandRegistry,
    command: &str,
    subcommand: Option<&str>,
) -> Option<TaintColour> {
    let spec = registry.get(command)?;
    if let Some(sub_name) = subcommand
        && let Some(sub) = spec.resolve_subcommand(sub_name)
        && sub.taint_double_encode_colour.is_some()
    {
        return sub.taint_double_encode_colour;
    }
    spec.taint_double_encode_colour
}

/// Colour that suppresses the T100 dangerous-sink warning for
/// `command` (e.g. `SHELL_ATOM` for `exec`).
///
/// Registry-side counterpart of `runtime.taint_sink_safe_colours`.
#[must_use]
pub fn taint_sink_safe_colour(registry: &CommandRegistry, command: &str) -> Option<TaintColour> {
    registry.get(command)?.taint_sink_safe_colour
}

/// Setter-form constraints declared on `command` (IRULE3101) — read by
/// `tcl_compiler::taint::find_setter_constraint_warnings` straight from
/// the spec (`HTTP::uri` / `HTTP::path` declare the `/`-prefix rule), so
/// the constraint table is owned by the registry. Empty slice when none.
#[must_use]
pub fn setter_constraints(
    registry: &CommandRegistry,
    command: &str,
) -> &'static [SetterConstraint] {
    registry.get(command).map_or(&[], |s| s.setter_constraints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_is_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(is_taint_source(
            &registry,
            "gets",
            &["stdin"],
            DialectSet::empty()
        ));
    }

    #[test]
    fn chan_gets_is_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(is_taint_source(
            &registry,
            "chan",
            &["gets", "stdin"],
            DialectSet::empty()
        ));
    }

    #[test]
    fn chan_configure_is_not_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(!is_taint_source(
            &registry,
            "chan",
            &["configure", "$ch"],
            DialectSet::empty()
        ));
    }

    #[test]
    fn http_uri_is_an_irules_source() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        assert!(is_taint_source(
            &registry,
            "HTTP::uri",
            &[],
            DialectSet::IRULES
        ));
    }

    #[test]
    fn http_uri_is_a_source_in_every_dialect() {
        // `TAINT_HINTS` is an import-time global, so `HTTP::uri`
        // is a taint source even when analysing a non-iRules document
        // (e.g. `tcl8.6`, where the iRules spec set is not loaded). This
        // is what lets the generic option-injection / sink checks fire on
        // iRules data regardless of the document's declared dialect.
        let registry = CommandRegistry::build_default();
        assert!(is_taint_source(
            &registry,
            "HTTP::uri",
            &[],
            DialectSet::empty()
        ));
    }

    #[test]
    fn http_path_source_colour_is_path_prefixed() {
        // The getter form carries `PATH_PREFIXED`, augmented to
        // `NON_DASH_PREFIXED` — the option-injection-safe colour set.
        let registry = CommandRegistry::build_default();
        let colour =
            taint_source_colour(&registry, "HTTP::path", &[], DialectSet::empty()).unwrap();
        assert!(colour.contains(TaintColour::TAINTED));
        assert!(colour.contains(TaintColour::PATH_PREFIXED));
        assert!(colour.contains(TaintColour::NON_DASH_PREFIXED));
    }

    #[test]
    fn ip_and_port_source_colours_are_augmented() {
        // IP / port getters prove NON_DASH_PREFIXED + CRLF_FREE +
        // SHELL_ATOM on top of their IP_ADDRESS / PORT colour.
        let registry = CommandRegistry::build_default();
        let ip =
            taint_source_colour(&registry, "IP::client_addr", &[], DialectSet::empty()).unwrap();
        assert!(
            ip.contains(TaintColour::IP_ADDRESS | TaintColour::CRLF_FREE | TaintColour::SHELL_ATOM)
        );
        let port =
            taint_source_colour(&registry, "TCP::remote_port", &[], DialectSet::empty()).unwrap();
        assert!(port.contains(TaintColour::PORT | TaintColour::NON_DASH_PREFIXED));
    }

    #[test]
    fn plain_irules_source_is_bare_tainted() {
        // A prefix-matched getter without a special colour is plain
        // TAINTED — no mitigating colours.
        let registry = CommandRegistry::build_default();
        let colour =
            taint_source_colour(&registry, "HTTP::header", &["host"], DialectSet::empty()).unwrap();
        assert_eq!(colour, TaintColour::TAINTED);
    }

    #[test]
    fn non_source_has_no_source_colour() {
        let registry = CommandRegistry::build_default();
        assert!(
            taint_source_colour(&registry, "string", &["length", "$x"], DialectSet::empty())
                .is_none()
        );
    }

    #[test]
    fn string_length_is_a_sanitiser() {
        let registry = CommandRegistry::build_default();
        assert!(is_sanitiser(&registry, "string", &["length", "$x"]));
    }

    /// `encoding convertfrom` is now driven by a `Traits::TAINT_SOURCE`
    /// flag on the matched `SubCommand` — no command-name pattern.
    #[test]
    fn encoding_convertfrom_is_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(is_taint_source(
            &registry,
            "encoding",
            &["convertfrom", "utf-8", "$bytes"],
            DialectSet::empty(),
        ));
    }

    /// Other `encoding` subcommands stay clean — proves the new
    /// dispatch is per-subcommand and does not over-match.
    #[test]
    fn encoding_system_is_not_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(!is_taint_source(
            &registry,
            "encoding",
            &["system"],
            DialectSet::empty(),
        ));
    }

    /// `SubCommand::DEFAULT` carries no traits; this guards against
    /// accidental drift if the field grows defaults later.
    #[test]
    fn subcommand_default_traits_are_empty() {
        use crate::spec::SubCommand;
        assert!(SubCommand::DEFAULT.traits.is_empty());
    }

    /// The taint/security fields default to clear, so a
    /// spec that doesn't opt in is never misclassified.
    #[test]
    fn default_spec_has_no_taint_metadata() {
        use crate::spec::{CommandSpec, SubCommand};
        let c = CommandSpec::DEFAULT;
        assert!(c.taint_source.is_none());
        assert!(c.taint_output_sink.is_none());
        assert!(c.taint_log_sink.is_none());
        assert!(c.taint_network_sink_args.is_none());
        assert!(c.taint_transform.is_none());
        assert!(c.taint_double_encode_colour.is_none());
        assert!(c.taint_sink_safe_colour.is_none());
        assert!(c.taint_sink_gate.is_none());
        assert!(c.credential_options.is_empty());
        assert!(c.sensitive_headers.is_empty());
        assert!(c.setter_constraints.is_empty());
        assert!(c.taint_interp_eval_subcommands.is_empty());
        assert!(c.taint_output_sink_subcommands.is_empty());

        let s = SubCommand::DEFAULT;
        assert!(s.taint_transform.is_none());
        assert!(s.taint_double_encode_colour.is_none());
        assert!(s.taint_output_sink.is_none());
        assert!(s.credential_arg.is_none());
        assert!(s.sensitive_headers.is_empty());
    }

    /// `classify_taint_sinks` returns the all-clear default for an
    /// unknown command rather than panicking.
    #[test]
    fn classify_unknown_command_is_clear() {
        let registry = CommandRegistry::build_default();
        let info = classify_taint_sinks(&registry, "no_such_cmd", None, DialectSet::empty());
        assert_eq!(info, TaintSinkInfo::default());
        assert!(!info.is_code_sink);
        assert!(info.output_sink.is_none());
    }

    /// `exec` suppresses T100 on a `SHELL_ATOM` value,
    /// `eval`/`uplevel` on `LIST_CANONICAL`.
    #[test]
    fn sink_safe_colours_are_populated() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            taint_sink_safe_colour(&registry, "exec"),
            Some(TaintColour::SHELL_ATOM)
        );
        assert_eq!(
            taint_sink_safe_colour(&registry, "eval"),
            Some(TaintColour::LIST_CANONICAL)
        );
        assert_eq!(
            taint_sink_safe_colour(&registry, "uplevel"),
            Some(TaintColour::LIST_CANONICAL)
        );
    }

    /// `puts` is a T101 output sink; `socket` /
    /// `http::geturl` are network sinks; `interp` carries the T105
    /// interp-eval subcommands.
    #[test]
    fn sink_classification_is_populated() {
        let registry = CommandRegistry::build_default();
        let puts = classify_taint_sinks(&registry, "puts", None, DialectSet::empty());
        assert_eq!(puts.output_sink, Some("T101"));
        assert!(!puts.output_sink_is_subcommand_qualified);

        let socket = classify_taint_sinks(&registry, "socket", None, DialectSet::empty());
        assert!(socket.is_network_sink);

        let geturl = classify_taint_sinks(&registry, "http::geturl", None, DialectSet::empty());
        assert!(geturl.is_network_sink);
        assert_eq!(
            registry.get("http::geturl").unwrap().credential_options,
            &["-headers"]
        );

        let interp = classify_taint_sinks(&registry, "interp", Some("eval"), DialectSet::empty());
        assert_eq!(interp.interp_eval_subcommands, &["eval", "invokehidden"]);
    }

    /// The sanitising transforms (`URI::encode`,
    /// `file join`/`file normalize`) resolve through the accessors,
    /// command- and subcommand-level.
    #[test]
    fn transforms_are_populated() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        assert_eq!(
            taint_transform(&registry, "URI::encode", None),
            Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE))
        );
        assert_eq!(
            taint_double_encode_colour(&registry, "HTML::encode", None),
            Some(TaintColour::HTML_ESCAPED)
        );
        assert_eq!(
            taint_transform(&registry, "file", Some("join")),
            Some(TaintColour::PATH_JOINED)
        );
        assert_eq!(
            taint_transform(&registry, "file", Some("normalize")),
            Some(TaintColour::PATH_NORMALISED)
        );
    }

    /// `HTTP::cookie insert` is an IRULE3002 sink but
    /// `HTTP::cookie domain` is not (subcommand-qualified).
    #[test]
    fn cookie_output_sink_is_subcommand_qualified() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let dialect = DialectSet::IRULES;
        let insert = classify_taint_sinks(&registry, "HTTP::cookie", Some("insert"), dialect);
        assert_eq!(insert.output_sink, Some("IRULE3002"));
        assert!(insert.output_sink_is_subcommand_qualified);

        let domain = classify_taint_sinks(&registry, "HTTP::cookie", Some("domain"), dialect);
        assert_eq!(domain.output_sink, None);
    }

    /// a unique-prefix abbreviation of a sink subcommand
    /// (`HTTP::cookie ins` ⇒ `insert`) must still be classified as the
    /// IRULE3002 sink — an exact `contains` let it dodge the check.
    #[test]
    fn cookie_output_sink_matches_prefix_abbreviation() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let dialect = DialectSet::IRULES;
        for abbr in ["ins", "inse", "insert", "rep", "replace"] {
            let info = classify_taint_sinks(&registry, "HTTP::cookie", Some(abbr), dialect);
            assert_eq!(
                info.output_sink,
                Some("IRULE3002"),
                "`HTTP::cookie {abbr}` should resolve to a sink",
            );
        }
        // A non-sink subcommand (and its abbreviation) still isn't a sink.
        let dom = classify_taint_sinks(&registry, "HTTP::cookie", Some("dom"), dialect);
        assert_eq!(dom.output_sink, None);
    }

    /// The registry-driven setter-constraint table is
    /// populated for `HTTP::uri` / `HTTP::path`.
    #[test]
    fn setter_constraints_are_populated() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let uri = setter_constraints(&registry, "HTTP::uri");
        assert_eq!(uri.len(), 1);
        assert_eq!(uri[0].required_prefix, "/");
        assert_eq!(uri[0].code.as_str(), "IRULE3101");
        assert_eq!(setter_constraints(&registry, "HTTP::path").len(), 1);
    }
}
