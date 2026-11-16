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

//! The token kinds, the legend the server announces and the modifier bits.

/// Indexed enum for the token types we emit.  Numeric
/// values must align with the order returned by
/// [`legend_token_types`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(super) enum TokenKind {
    Keyword = 0,
    Function = 1,
    Variable = 2,
    String = 3,
    Number = 4,
    Comment = 5,
    Namespace = 6,
    /// Regular-expression pattern argument (`regexp` / `regsub`).
    Regexp = 7,
    /// iRules event name (`when EVENT`).
    Event = 8,
    /// Regex group / flags: `(`, `)`, `(?:`, `(?imsx)`.
    RegexpGroup = 9,
    /// Regex character class: `[...]`, `\d` / `\w` / `\s`, `.`.
    RegexpCharClass = 10,
    /// Regex quantifier: `*` `+` `?` `{n,m}` and lazy variants.
    RegexpQuantifier = 11,
    /// Regex anchor: `^` `$` `\A` `\Z` `\z` `\b` `\B` `\m` `\M` `\y` `\Y`.
    RegexpAnchor = 12,
    /// Regex escape sequence: `\n` `\t` `\xHH` `\uHHHH` `\<meta>`.
    RegexpEscape = 13,
    /// Regex backreference: `\0`–`\9`.
    RegexpBackref = 14,
    /// Regex alternation pipe: `|`.
    RegexpAlternation = 15,
    /// `format`/`scan` `%` introducer and `$` position separator.
    FormatPercent = 16,
    /// `format`/`scan` conversion type letter (`d` `s` `f` `x` …).
    FormatSpec = 17,
    /// `format`/`scan` flags (`-` `+` `0` `#` space) and length modifier.
    FormatFlag = 18,
    /// `format`/`scan` numeric width / precision values.
    FormatWidth = 19,
    /// `clock format`/`scan` `%` introducer.
    ClockPercent = 20,
    /// `clock` specifier letter (`Y` `m` `d` `H` `M` `S` …).
    ClockSpec = 21,
    /// Reserved protocol slot for a clock locale modifier. The shared clock
    /// runtime currently does not implement `%E`/`%O`, so no token emits it.
    #[allow(dead_code)]
    ClockModifier = 22,
    /// `binary format`/`scan` specifier letter (`a` `A` `c` `i` `w` …).
    BinarySpec = 23,
    /// `binary` repeat count (numeric).
    BinaryCount = 24,
    /// `binary` modifier: `u` / `s` (signed/unsigned) or `*` (all).
    BinaryFlag = 25,
    /// Operator — an `expr` operator (`+`, `==`, `&&`, …) or the `regsub`
    /// whole-match replacement backref `\&`.
    Operator = 26,
    /// BIG-IP object name referenced from iRules code (pool, data group,
    /// virtual, node, …).
    Object = 27,
    /// A recognised `-option` switch on a command (`regexp -nocase`).
    Decorator = 28,
    /// A backslash escape sequence inside a string/bareword (`\n`, `\t`, …).
    Escape = 29,
    /// A registry-known closed-set argument value (`string is alnum`,
    /// `HTTP::respond 200 content`, `when … timing enable`).
    EnumMember = 30,
    /// The literal value argument of a recognised value-taking option
    /// (`-name fitted`, `-type value`, `-min 0.4`) — highlighted distinctly
    /// from the option switch itself (`Decorator`) and from a plain string.
    OptionValue = 31,
    // APL (iApp presentation language) — the bespoke `tcl-apl` token set,
    // emitted only by [`apl_full`].  A `.apl` file is not Tcl, so these never
    // co-occur with the types above.  Append-only: the indices are wire format.
    /// APL block keyword: `section`, `text`, `table`, `row`.
    AplSection = 32,
    /// APL field-type keyword: `string`, `choice`, `password`, …
    AplFieldType = 33,
    /// APL field attribute: `default`, `display`, `required`, `validator`.
    AplAttribute = 34,
    /// The name following an APL block keyword.
    AplSectionName = 35,
    /// The name following an APL field-type keyword.
    AplFieldName = 36,
    /// The APL `define` keyword.
    AplDefine = 37,
    /// The name bound by an APL `define`.
    AplDefineName = 38,
    /// An APL preprocessor directive: `#include`, `#inline`.
    AplDirective = 39,
    /// The APL `optional` guard keyword.
    AplOptional = 40,
    /// A known validator name inside an APL `validator "…"` value.
    AplValidator = 41,
    // BIG-IP config (`bigip.conf`, `.scf`) — emitted only by [`bigip_conf_full`]
    // for `tcl-bigip` documents.  (`Object` above is shared: it already types
    // BIG-IP object references inside *iRules*.)  Append-only: the indices are
    // wire format.
    /// A partition name (`/Common/…`).
    Partition = 42,
    /// An object name known to be a pool.
    Pool = 43,
    /// An object name known to be a monitor.
    Monitor = 44,
    /// An object name known to be a profile.
    Profile = 45,
    /// An object name known to be a VLAN or trunk.
    Vlan = 46,
    /// A BIG-IP network interface name (`1.1`, `mgmt`).  Named `bigipInterface`
    /// in the legend, **not** `interface`: reusing the standard LSP `interface`
    /// type for this would shadow its real meaning.
    BigipInterface = 47,
    /// An IPv4 literal, with optional CIDR suffix.
    IpAddress = 48,
    /// A TCP/UDP port number.
    Port = 49,
    /// A route domain (`%0`).
    RouteDomain = 50,
    /// A fully-qualified domain name.
    Fqdn = 51,
    /// A user name.
    Username = 52,
    /// An encrypted / secret value.
    Encrypted = 53,
    /// A procedure / method / constructor parameter, in its declaring parameter
    /// list.  The standard LSP type, so a theme distinguishes an argument from
    /// an ordinary local.
    Parameter = 54,
    /// A `TclOO` / snit / itcl **method** — its declared name (`method foo {…}`)
    /// and its call sites (`my foo`, `$obj foo`).  The standard LSP type, so a
    /// method is not conflated with a free procedure.
    Method = 55,
    /// A class name — `oo::class create Shape`, `oo::define Shape`.
    Class = 56,
}

impl TokenKind {
    /// Whether Sublime needs a custom scope entry for this token type.
    /// Standard LSP token types are already styled by the client.
    const fn needs_sublime_custom_scope(self) -> bool {
        !matches!(
            self,
            Self::Class
                | Self::Comment
                | Self::Decorator
                | Self::EnumMember
                | Self::Event
                | Self::Function
                | Self::Keyword
                | Self::Method
                | Self::Namespace
                | Self::Number
                | Self::Operator
                | Self::OptionValue
                | Self::Parameter
                | Self::Regexp
                | Self::String
                | Self::Variable
        )
    }
}

/// The typed semantic-token variants in wire order. Keep this beside the
/// private kind enum so editor projections derive custom names from the same
/// semantic owner rather than maintaining another string vocabulary.
pub(super) const ALL_TOKEN_KINDS: &[TokenKind] = &[
    TokenKind::Keyword,
    TokenKind::Function,
    TokenKind::Variable,
    TokenKind::String,
    TokenKind::Number,
    TokenKind::Comment,
    TokenKind::Namespace,
    TokenKind::Regexp,
    TokenKind::Event,
    TokenKind::RegexpGroup,
    TokenKind::RegexpCharClass,
    TokenKind::RegexpQuantifier,
    TokenKind::RegexpAnchor,
    TokenKind::RegexpEscape,
    TokenKind::RegexpBackref,
    TokenKind::RegexpAlternation,
    TokenKind::FormatPercent,
    TokenKind::FormatSpec,
    TokenKind::FormatFlag,
    TokenKind::FormatWidth,
    TokenKind::ClockPercent,
    TokenKind::ClockSpec,
    TokenKind::ClockModifier,
    TokenKind::BinarySpec,
    TokenKind::BinaryCount,
    TokenKind::BinaryFlag,
    TokenKind::Operator,
    TokenKind::Object,
    TokenKind::Decorator,
    TokenKind::Escape,
    TokenKind::EnumMember,
    TokenKind::OptionValue,
    TokenKind::AplSection,
    TokenKind::AplFieldType,
    TokenKind::AplAttribute,
    TokenKind::AplSectionName,
    TokenKind::AplFieldName,
    TokenKind::AplDefine,
    TokenKind::AplDefineName,
    TokenKind::AplDirective,
    TokenKind::AplOptional,
    TokenKind::AplValidator,
    TokenKind::Partition,
    TokenKind::Pool,
    TokenKind::Monitor,
    TokenKind::Profile,
    TokenKind::Vlan,
    TokenKind::BigipInterface,
    TokenKind::IpAddress,
    TokenKind::Port,
    TokenKind::RouteDomain,
    TokenKind::Fqdn,
    TokenKind::Username,
    TokenKind::Encrypted,
    TokenKind::Parameter,
    TokenKind::Method,
    TokenKind::Class,
];

/// The iApps dialect key — the dialect an APL presentation's embedded `[ … ]`
/// Tcl is written in.
pub(super) const IAPPS_DIALECT: &str = "f5-iapps";

/// The token-type / token-modifier legend the server
/// advertises during `initialize`.
#[must_use]
pub fn legend_token_types() -> Vec<&'static str> {
    vec![
        "keyword",
        "function",
        "variable",
        "string",
        "number",
        "comment",
        "namespace",
        "regexp",
        "event",
        "regexpGroup",
        "regexpCharClass",
        "regexpQuantifier",
        "regexpAnchor",
        "regexpEscape",
        "regexpBackref",
        "regexpAlternation",
        "formatPercent",
        "formatSpec",
        "formatFlag",
        "formatWidth",
        "clockPercent",
        "clockSpec",
        "clockModifier",
        "binarySpec",
        "binaryCount",
        "binaryFlag",
        "operator",
        "object",
        "decorator",
        "escape",
        "enumMember",
        // Option-value words (`-name fitted`): the standard LSP `property`
        // type gives them a distinct colour from the option switch and from a
        // plain string in default themes.
        "property",
        // APL (iApp presentation) — emitted only for `tcl-apl` documents (see
        // [`apl_full`]).  Index-aligned with the `Apl*` `TokenKind` variants.
        "aplSection",
        "aplFieldType",
        "aplAttribute",
        "aplSectionName",
        "aplFieldName",
        "aplDefine",
        "aplDefineName",
        "aplDirective",
        "aplOptional",
        "aplValidator",
        // BIG-IP config — emitted only for `tcl-bigip` documents (see
        // [`bigip_conf_full`]).  Index-aligned with the `TokenKind` variants.
        "partition",
        "pool",
        "monitor",
        "profile",
        "vlan",
        "bigipInterface",
        "ipAddress",
        "port",
        "routeDomain",
        "fqdn",
        "username",
        "encrypted",
        // Standard LSP types — VS Code styles them out of the box.
        "parameter",
        "method",
        "class",
    ]
}

/// Semantic-token types that need a Sublime custom scope. The names are
/// projected from the advertised legend through the typed token variants.
#[must_use]
pub fn legend_custom_token_types() -> Option<Vec<&'static str>> {
    let legend = legend_token_types();
    if ALL_TOKEN_KINDS.len() != legend.len()
        || ALL_TOKEN_KINDS
            .iter()
            .enumerate()
            .any(|(index, kind)| *kind as usize != index)
    {
        return None;
    }
    ALL_TOKEN_KINDS
        .iter()
        .filter(|kind| kind.needs_sublime_custom_scope())
        .map(|kind| legend.get(*kind as usize).copied())
        .collect()
}

/// Token-modifiers part of the legend.  Order is fixed and must
/// align with the `1 << index` bits in [`MOD_DEFAULT_LIBRARY`] etc.
#[must_use]
pub fn legend_token_modifiers() -> Vec<&'static str> {
    vec!["declaration", "definition", "readonly", "defaultLibrary"]
}

/// `defaultLibrary` modifier bit (legend index 3) — set on a
/// command head that resolves to a registry built-in.
pub(super) const MOD_DEFAULT_LIBRARY: u32 = 1 << 3;

/// `definition` modifier bit (legend index 1) — set on the name token of a
/// `proc` definition.
pub(super) const MOD_DEFINITION: u32 = 1 << 1;

/// `declaration` modifier bit (legend index 0) — set on a variable name a
/// command declares / writes (`set x`, `incr n`, `global v`, `lassign … a`).
pub(super) const MOD_DECLARATION: u32 = 1 << 0;
