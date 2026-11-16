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

//! Semantic-tokens provider.
//!
//! Produces an LSP-encoded semantic-tokens stream covering
//! the common Tcl token categories:
//!
//! * **Keyword** — command heads carrying the registry's
//!   `LANGUAGE_KEYWORD` trait (`if`, `while`, `for`, `foreach`,
//!   `switch`, `return`, `break`, `continue`, `try`, `catch`,
//!   `proc`, `namespace`, `when`, `oo::*`, …) plus the non-command
//!   clause / `TclOO` sub-keywords (`else`, `elseif`, `method`,
//!   `constructor`, …).
//! * **Function** — every other command-head token (user
//!   procs + built-in commands).
//! * **Variable** — `$name` / `${name}` substitutions.
//! * **String** — braced literals (`{...}`) and double-quoted
//!   strings.
//! * **Number** — integer / float literals.
//! * **Comment** — `# ...` comment lines.
//! * **Namespace** — namespace-qualified names containing
//!   `::`.
//! * **Regexp** — the regex-pattern argument of `regexp` / `regsub`
//!   (registry `pattern_type == Regex`, option-skipped positional),
//!   sub-tokenised into ARE components (`RegexpGroup` /
//!   `RegexpCharClass` / `RegexpQuantifier` / `RegexpAnchor` /
//!   `RegexpEscape` / `RegexpBackref` / `RegexpAlternation`).
//! * **Event** — an iRules `when EVENT` event name.
//! * **Format** — `format` / `scan` conversion strings
//!   (`FormatPercent` / `FormatFlag` / `FormatWidth` / `FormatSpec`),
//!   `clock format` / `scan` field strings (`ClockPercent` /
//!   `ClockSpec` / `ClockModifier`), `binary format` / `scan`
//!   field strings (`BinarySpec` / `BinaryCount` / `BinaryFlag`), and
//!   `regsub` replacement backrefs (`\1` → `Number`, `\&` → `Operator`).
//! * **Object** — BIG-IP object names (pools, data groups, virtuals,
//!   nodes, …) referenced from iRules code, under the `f5-irules`
//!   dialect (see [`crate::irules_object_refs`]).
//!
//! The legend is exposed via [`legend_token_types`] and
//! [`legend_token_modifiers`] so the server advertises it in
//! the LSP `initialize` capabilities response.
//!
//! Additional variants:
//!
//! * Range variant ([`range`]) — same encoding as [`full`]
//!   filtered to tokens whose start position falls inside
//!   the request range.  Server advertises `range: true`.
//! * Delta variant — when the client's `previousResultId`
//!   matches the per-URI cached stream, the server returns the
//!   minimal token-aligned edit computed by [`diff`] (an empty
//!   edit list when nothing changed); a stale / unknown previous
//!   id falls back to a fresh full stream.
//!
//! Two **document-mode** grammars are handled here too, because neither is Tcl
//! and running the Tcl tokeniser over them mis-colours the file rather than
//! merely under-colouring it (each braced block reads as one literal word, so
//! whole *lines* come out as `string`):
//!
//! * [`bigip_conf_full`] — BIG-IP config (`bigip.conf` / `.scf`): partition
//!   paths (`/Common/…`), IPv4 / route-domain / port literals, and the object
//!   taxonomy.  A `ltm rule { … }` stanza's body is iRules code, so it is
//!   re-walked as Tcl.
//! * [`apl_full`] — APL (iApp presentation).  Its `[ … ]` bracket expressions
//!   are embedded Tcl and are likewise re-walked as Tcl.
//!
//! Both lexers live in `tcl-bigip` and are non-overlapping by construction; the
//! iRules object-reference overlay (the code-relevant half of the BIG-IP
//! taxonomy) applies inside embedded rule bodies exactly as it does in a
//! standalone `.irul`.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::types::{ProcArgTrait, ProcDef};
use tcl_compiler::analyser::{AnalysisResult, ClassHierarchy};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LineIndex, Span, Token, TokenType};

use crate::definition::utf16_len;
use tcl_dialect::NumberSyntax;
use tcl_registry::CommandRegistry;
use tcl_registry::definer::DefinitionBodyGrammar;

mod original;
mod receiver_source;

/// Encoded semantic-tokens response.  The `data` array is
/// the LSP packed integer encoding (5 ints per token: line
/// delta, column delta, length, type, modifiers).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticTokens {
    /// Packed integer data.
    pub data: Vec<u32>,
}

/// Indexed enum for the token types we emit.  Numeric
/// values must align with the order returned by
/// [`legend_token_types`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum TokenKind {
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
const ALL_TOKEN_KINDS: &[TokenKind] = &[
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
const IAPPS_DIALECT: &str = "f5-iapps";

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
const MOD_DEFAULT_LIBRARY: u32 = 1 << 3;

/// `definition` modifier bit (legend index 1) — set on the name token of a
/// `proc` definition.
const MOD_DEFINITION: u32 = 1 << 1;

/// `declaration` modifier bit (legend index 0) — set on a variable name a
/// command declares / writes (`set x`, `incr n`, `global v`, `lassign … a`).
const MOD_DECLARATION: u32 = 1 << 0;

/// Extra "this argument names a written variable" positions for commands the
/// static [`CommandRegistry`] does not model — user procs whose parameters the
/// analyser inferred to alias a caller variable (`upvar $param`), and
/// `# tcl-lsp: stub … :var` / `:var_read` declarations.  Keyed by command /
/// proc name; each value is the 0-based argument indices (head excluded) that
/// name a variable — split by direction, since a *written* target highlights as
/// a `Variable` declaration and a *read* reference as a plain `Variable`.  Lets
/// the retag highlight `myset arr(key) …` / `myexists arr(key)` the same way it
/// highlights `set arr(key) …` / `info exists arr(key)`.
///
/// Built from an [`AnalysisResult`] (single file) or merged across a project's
/// files.  Empty (and cost-free) on the pure-segmentation path, where only the
/// static registry roles apply.  Stub roles are source-derived and unioned in
/// at token-collection time, so they apply on every path without threading.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VarNameArgRoles {
    write: FxHashMap<String, Vec<u32>>,
    read: FxHashMap<String, Vec<u32>>,
    /// Argument positions whose value the analyser inferred to name a *command*
    /// ([`ProcArgTrait::Command`] — a `$param` command head or a `CommandPrefix`
    /// argument), so a literal call-site arg highlights as a command.
    command: FxHashMap<String, Vec<u32>>,
    /// Names this index *abstained* on because the procs it was built from
    /// disagreed about their indices, per direction.
    ///
    /// Carried rather than discarded so [`VarNameArgRoles::merge`] can fold
    /// per-file indexes into a project-wide one that equals
    /// [`VarNameArgRoles::from_procs`] over every file's procs at once: without
    /// it, a name one file already dropped as ambiguous would silently be
    /// re-adopted from another file's unambiguous entry.
    ambiguous: RoleAmbiguity,
    original: Vec<OriginalProcArgRoles>,
    project_source:
        Vec<tcl_compiler::registry_invocation::source_structure::OriginalProjectProcedureRoles>,
}

/// Retained byte publication and declaration identity for inferred assistance.
/// These roles establish no actual variable binding or compiler preparation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalProcArgRoles {
    name: tcl_compiler::signature_scan::scope::SignatureSourceCommand,
    site: Option<tcl_compiler::command_binding::CommandAllocationSite>,
    write: Vec<u32>,
    read: Vec<u32>,
    command: Vec<u32>,
}

impl OriginalProcArgRoles {
    fn new(
        name: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        site: Option<&tcl_compiler::command_binding::CommandAllocationSite>,
        proc_def: &ProcDef,
    ) -> Self {
        Self {
            name: name.clone(),
            site: site.cloned(),
            write: proc_var_write_indices(proc_def),
            read: proc_var_read_indices(proc_def),
            command: proc_command_indices(proc_def),
        }
    }

    fn same_roles(&self, other: &Self) -> bool {
        self.write == other.write && self.read == other.read && self.command == other.command
    }
}

/// The per-direction abstention sets of a [`VarNameArgRoles`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RoleAmbiguity {
    write: FxHashSet<String>,
    read: FxHashSet<String>,
    command: FxHashSet<String>,
}

impl VarNameArgRoles {
    /// Retain genuine declaring source/formal owners for conditional cross-file
    /// display. Report maps never select a call or reconstruct a formal binding.
    #[must_use]
    pub fn from_project_source_roles(
        providers: Vec<
            tcl_compiler::registry_invocation::source_structure::OriginalProjectProcedureRoles,
        >,
    ) -> Self {
        Self {
            project_source: providers,
            ..Self::default()
        }
    }

    /// Exact original cross-document provider transport, independently of the
    /// compatibility maps used by standalone callers.
    #[must_use]
    pub fn project_source_roles(
        &self,
    ) -> &[tcl_compiler::registry_invocation::source_structure::OriginalProjectProcedureRoles] {
        &self.project_source
    }

    /// Infer the variable-name argument positions of every proc in `analysis`.
    #[must_use]
    pub fn from_analysis(analysis: &AnalysisResult) -> Self {
        let mut result =
            Self::from_original_declarations(analysis.original_procedure_declarations());
        if analysis.allows_lexical_declaration_advice() {
            let lexical = Self::from_procs(
                analysis
                    .all_procs
                    .values()
                    .filter(|proc_def| proc_def.source_name.is_none()),
            );
            result = Self::merge([&result, &lexical]);
        }
        result
    }

    /// Preserve original declarations even when their UI names collide.
    #[must_use]
    pub fn from_original_declarations<'a>(
        declarations: impl IntoIterator<
            Item = &'a tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                ProcDef,
            >,
        >,
    ) -> Self {
        Self {
            original: declarations
                .into_iter()
                .map(|record| {
                    OriginalProcArgRoles::new(
                        record.name(),
                        Some(record.declaration_site()),
                        record.metadata(),
                    )
                })
                .collect(),
            ..Self::default()
        }
    }

    /// Infer from an iterator of proc definitions — a single file's, or a whole
    /// project's files chained together.  A proc name that resolves to two
    /// *different non-empty* index sets across the iterator is dropped as
    /// ambiguous, so the result is independent of iteration order — matching the
    /// highlight-only, sound-by-abstention posture of the cross-file class
    /// index.  An empty index set (the proc has no by-reference argument in that
    /// direction) never participates: it neither seeds nor conflicts an entry,
    /// so a definition that contributes roles is not cancelled by one that
    /// contributes none.
    #[must_use]
    pub fn from_procs<'a>(procs: impl IntoIterator<Item = &'a ProcDef>) -> Self {
        let mut write = RoleMapBuilder::default();
        let mut read = RoleMapBuilder::default();
        let mut command = RoleMapBuilder::default();
        let mut original = Vec::new();
        for proc in procs {
            if let Some(name) = &proc.source_name {
                original.push(OriginalProcArgRoles::new(name, None, proc));
                continue;
            }
            let keys = proc_name_keys(proc);
            write.insert(&keys, &proc_var_write_indices(proc));
            read.insert(&keys, &proc_var_read_indices(proc));
            command.insert(&keys, &proc_command_indices(proc));
        }
        let mut result = Self::from_builders(write, read, command);
        result.original = original;
        result
    }

    /// Fold per-file indexes into one project-wide index, applying the same
    /// abstain-on-conflict rule [`Self::from_procs`] applies within a file — and
    /// inheriting each part's own abstentions, so the result is exactly
    /// `from_procs` over every part's procs concatenated, and independent of the
    /// order the parts arrive in.
    #[must_use]
    pub fn merge<'a>(parts: impl IntoIterator<Item = &'a Self>) -> Self {
        let mut write = RoleMapBuilder::default();
        let mut read = RoleMapBuilder::default();
        let mut command = RoleMapBuilder::default();
        let mut original = Vec::new();
        let mut project_source = Vec::new();
        for part in parts {
            project_source.extend(part.project_source.iter().cloned());
            original.extend(part.original.iter().cloned());
            write.absorb(&part.write, &part.ambiguous.write);
            read.absorb(&part.read, &part.ambiguous.read);
            command.absorb(&part.command, &part.ambiguous.command);
        }
        let mut result = Self::from_builders(write, read, command);
        result.original = original;
        result.project_source = project_source;
        result
    }

    /// Close the three per-direction builders into an index, keeping each
    /// direction's abstentions alongside its map.
    fn from_builders(write: RoleMapBuilder, read: RoleMapBuilder, command: RoleMapBuilder) -> Self {
        let (write, write_ambiguous) = write.finish();
        let (read, read_ambiguous) = read.finish();
        let (command, command_ambiguous) = command.finish();
        Self {
            write,
            read,
            command,
            original: Vec::new(),
            project_source: Vec::new(),
            ambiguous: RoleAmbiguity {
                write: write_ambiguous,
                read: read_ambiguous,
                command: command_ambiguous,
            },
        }
    }

    /// `true` when no command carries an inferred by-reference argument.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.write.is_empty()
            && self.read.is_empty()
            && self.command.is_empty()
            && self
                .project_source
                .iter()
                .all(|provider| !provider.has_roles())
            && self.original.iter().all(|roles| {
                roles.write.is_empty() && roles.read.is_empty() && roles.command.is_empty()
            })
    }

    fn original_roles_at(
        &self,
        analysis: &AnalysisResult,
        source: &str,
        offset: u32,
    ) -> Option<OriginalProcArgRoles> {
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config?;
        if !analysis.matches_original_source_image(&image, config) {
            return None;
        }
        let invocation = crate::definition::invocation_reference_at(analysis, offset)?;
        if invocation.range.start() != offset {
            return None;
        }
        let input = invocation.original_name_input.as_ref()?;
        let lookup = invocation
            .original_lookup
            .as_ref()
            .filter(|lookup| lookup.name_input() == input)?;
        if lookup.site().offset != offset || lookup.site().source.source_image() != &image {
            return None;
        }
        if let Some(reference) = &invocation.resolved_command_reference {
            use tcl_compiler::command_binding::{BindingKind, SourceCommandReferenceBinding};
            let kind = match reference.binding() {
                SourceCommandReferenceBinding::Direct { kind, .. }
                | SourceCommandReferenceBinding::Imported { kind, .. } => kind,
            };
            // Alias prefixes need an independent effective argument mapping.
            if *kind != BindingKind::Proc {
                return None;
            }
            let definition = reference
                .linked_definition()
                .or_else(|| reference.definition())?;
            if let Some(proc_def) = analysis.proc_for_definition(definition, source) {
                let name = proc_def.source_name.as_ref()?;
                return Some(OriginalProcArgRoles::new(
                    name,
                    Some(&definition.allocation().site),
                    proc_def,
                ));
            }
            let mut selected = self
                .original
                .iter()
                .filter(|roles| roles.site.as_ref() == Some(&definition.allocation().site));
            let first = selected.next()?;
            return selected
                .all(|other| first.same_roles(other))
                .then(|| first.clone());
        }
        // Declaration assistance only: all original lookup alternatives and
        // all records at the selected byte publication must agree on roles.
        let selected =
            lookup.matching_publications(self.original.iter().map(|roles| (&roles.name, roles)))?;
        let first = *selected.first()?;
        selected
            .iter()
            .all(|other| first.same_roles(other))
            .then(|| first.clone())
    }
}

/// Accumulates one direction's `name → arg indices` map while dropping any name
/// that resolves to conflicting index sets (abstain-on-conflict), so the merged
/// result is independent of insertion order.
#[derive(Default)]
struct RoleMapBuilder {
    map: FxHashMap<String, Vec<u32>>,
    ambiguous: FxHashSet<String>,
}

impl RoleMapBuilder {
    fn insert(&mut self, keys: &[String], indices: &[u32]) {
        if indices.is_empty() {
            return;
        }
        for key in keys {
            if self.ambiguous.contains(key) {
                continue;
            }
            match self.map.get(key) {
                Some(existing) if existing.as_slice() != indices => {
                    self.map.remove(key);
                    self.ambiguous.insert(key.clone());
                }
                Some(_) => {}
                None => {
                    self.map.insert(key.clone(), indices.to_vec());
                }
            }
        }
    }

    /// Fold an already-built map and its abstention set in, so merging finished
    /// indexes reaches the same fixed point as inserting every underlying proc.
    fn absorb(&mut self, map: &FxHashMap<String, Vec<u32>>, ambiguous: &FxHashSet<String>) {
        for key in ambiguous {
            self.map.remove(key);
            self.ambiguous.insert(key.clone());
        }
        for (key, indices) in map {
            self.insert(std::slice::from_ref(key), indices);
        }
    }

    fn finish(self) -> (FxHashMap<String, Vec<u32>>, FxHashSet<String>) {
        (self.map, self.ambiguous)
    }
}

/// The 0-based argument indices of a proc's parameters that the analyser
/// inferred to *alias a caller variable written by the proc*
/// ([`ProcArgTrait::VarWrite`]) — so a literal name passed there names the
/// caller's variable.  [`ProcArgTrait::DynamicNameLocal`] (the param's *value*
/// names a callee-local variable) is excluded: a literal there is not the
/// caller's variable.
fn proc_var_write_indices(proc: &ProcDef) -> Vec<u32> {
    proc_indices_with_trait(proc, |traits| traits.contains(&ProcArgTrait::VarWrite))
}

/// The 0-based argument indices of a proc's parameters whose value the analyser
/// inferred to name a variable that is *read* — either a caller-frame
/// [`ProcArgTrait::VarRead`] `upvar` alias, or a
/// [`ProcArgTrait::DynamicNameLocal`] whose value names a callee-local variable
/// read (`set $p`, `set ${v}($k)`).  Both mean the literal at the call site is
/// a variable name, so both elevate to a read reference — the highlighter does
/// not need to distinguish caller-frame from callee-local.
fn proc_var_read_indices(proc: &ProcDef) -> Vec<u32> {
    proc_indices_with_trait(proc, |traits| {
        traits.contains(&ProcArgTrait::VarRead) || traits.contains(&ProcArgTrait::DynamicNameLocal)
    })
}

/// The 0-based argument indices of a proc's parameters whose value the analyser
/// inferred to name a *command* ([`ProcArgTrait::Command`] — a `$param` command
/// head or a `CommandPrefix` argument), so a literal at the call site highlights
/// as a command.
fn proc_command_indices(proc: &ProcDef) -> Vec<u32> {
    proc_indices_with_trait(proc, |traits| traits.contains(&ProcArgTrait::Command))
}

/// Shared body of [`proc_var_write_indices`] / [`proc_var_read_indices`]: the
/// positions of the parameters whose inferred trait set satisfies `keep`.
fn proc_indices_with_trait(
    proc: &ProcDef,
    keep: impl Fn(&std::collections::HashSet<ProcArgTrait>) -> bool,
) -> Vec<u32> {
    proc.params
        .iter()
        .enumerate()
        .filter(|(_, p)| proc.param_traits.get(&p.name).is_some_and(&keep))
        .filter_map(|(i, _)| u32::try_from(i).ok())
        .collect()
}

/// Every name a call site might use for `proc`: the bare name, the qualified
/// name, and the qualified name without its leading `::`.
fn proc_name_keys(proc: &ProcDef) -> Vec<String> {
    let mut keys = vec![proc.name.clone()];
    let stripped =
        tcl_syntax::naming::unroot_rooted_key(&proc.qualified_name).unwrap_or(&proc.qualified_name);
    if stripped != proc.name {
        keys.push(stripped.to_owned());
    }
    if proc.qualified_name != proc.name && proc.qualified_name != stripped {
        keys.push(proc.qualified_name.clone());
    }
    keys
}

/// Compute semantic tokens for the entire document.
#[must_use]
pub fn full(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> SemanticTokens {
    full_with_cu(source, dialect, registry, None)
}

/// First-paint semantic tokens from the written words and the registry alone.
///
/// No command is interpreted and no body is analysed, so the cost is one
/// pass over the text whatever the file's size. Command heads, subcommands,
/// options, bodies, expressions, parameters, variables and comments are
/// coloured as [`full`] colours them; what needs the analysis (user classes,
/// object dispatch, a rebound builtin, regex sources) arrives with the
/// analysed result.
#[must_use]
pub fn coarse(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> SemanticTokens {
    let shell = standalone_analyser(dialect, registry)
        .source_shell()
        .analyse(source, dialect.name);
    full_with_cu_and_analysis(source, dialect, registry, None, Some(&shell))
}

/// [`coarse`] filtered to the tokens that start inside `range`.
#[must_use]
pub fn coarse_range(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    let shell = standalone_analyser(dialect, registry)
        .source_shell()
        .analyse(source, dialect.name);
    range_with_cu_and_analysis(source, dialect, range, registry, None, Some(&shell))
}

/// Compute semantic tokens for an **APL** (iApp presentation) document.
///
/// APL is not Tcl — it is a declarative form-description grammar — so it does
/// not go through the Tcl segmenter at all.  Running the Tcl tokeniser over it
/// (which is what happens today for any document the server does not route
/// here) treats each braced block as one literal word and emits whole *lines*
/// as `String` tokens, actively mis-colouring the file.
///
/// The caller decides a document is APL with the server's `is_apl_source`
/// (language id `tcl-apl`, or a `*.apl` / `presentation` basename) — the
/// dialect string cannot be used for this, because `tcl-apl` and the Tcl
/// `tcl-iapp` *implementation* files both resolve to the `f5-iapps` dialect.
#[must_use]
pub fn apl_full(source: &str, registry: &CommandRegistry) -> SemanticTokens {
    encode_entries(&apl_entries(source, registry))
}

/// [`apl_full`] restricted to `range`, for viewport (`semanticTokens/range`)
/// requests.  Same half-open filter as the Tcl [`range`] path.
#[must_use]
pub fn apl_range(
    source: &str,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    encode_entries(&clip_to_range(apl_entries(source, registry), range))
}

/// Compute semantic tokens for a **BIG-IP config** document (`bigip.conf`,
/// `.scf`).
///
/// Like APL (see [`apl_full`]), BIG-IP config text is not Tcl: it is a
/// brace-delimited declarative config.  The Tcl tokeniser reads each stanza
/// body as one literal braced word and emits whole *lines* as `String` tokens
/// — 272 of `samples/bigip/bigip.conf`'s 302 tokens are exactly that, which
/// mis-colours the file rather than merely under-colouring it.
///
/// The caller decides a document is BIG-IP config with the server's
/// `is_bigip_conf_name` / the `f5-bigip` dialect.
#[must_use]
pub fn bigip_conf_full(source: &str, registry: &CommandRegistry) -> SemanticTokens {
    encode_entries(&bigip_conf_entries(source, registry))
}

/// [`bigip_conf_full`] restricted to `range`.
#[must_use]
pub fn bigip_conf_range(
    source: &str,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    encode_entries(&clip_to_range(bigip_conf_entries(source, registry), range))
}

/// Drop entries whose start falls outside `range` (half-open, per the LSP
/// `Range` semantics the Tcl [`range`] path uses).
fn clip_to_range(mut entries: Vec<Entry>, range: crate::definition::LspRange) -> Vec<Entry> {
    entries.retain(|(line, col, _, _, _)| {
        let pos = (*line, *col);
        let start = (range.start_line, range.start_character);
        let end = (range.end_line, range.end_character);
        pos >= start && pos < end
    });
    entries
}

/// Map the BIG-IP config lexer's tokens onto legend entries, and walk each
/// embedded iRule body as **Tcl**.
///
/// A `ltm rule /Common/x { … }` stanza's body is iRules code sitting inside a
/// config file.  Read as config it is nonsense — `when` / `if` / `switch`
/// become config *property keys* and `[HTTP::uri]` is not tokenised at all —
/// so the config lexer leaves those spans empty and they are re-walked here with
/// the iRules registry.  `registry` must therefore be the **iRules** one.
fn bigip_conf_entries(source: &str, registry: &CommandRegistry) -> Vec<Entry> {
    use tcl_bigip::conf_tokens::BigipTokenKind as B;

    let line_index = LineIndex::new(source);
    let mut entries: Vec<Entry> = Vec::new();

    // The Tcl regions are the bodies the shared rule parser recognises, ended
    // by the iRules brace grammar — not the config lexer's colour brackets,
    // which an escaped `\}` in the iRule would truncate.
    for rule in tcl_bigip::rule_extract::find_embedded_rules(source) {
        let (bstart, bend) = (rule.body_start_offset, rule.body_end_offset);
        let Some(body) = source.get(bstart..bend) else {
            continue;
        };
        // The body's tokens come back positioned relative to the body text, so
        // shift them onto the document: a token on the body's first line also
        // needs the column the body started at.
        let origin = line_index.position_at_utf16(u32::try_from(bstart).unwrap_or(0), source);
        for (line, col, len, kind, mods) in collect_entries(
            body,
            crate::profile_for_dialect("f5-irules"),
            registry,
            None,
            WorkspaceTokenFacts::default(),
        ) {
            let (line, col) = if line == 0 {
                (origin.line, origin.character.get() + col)
            } else {
                (origin.line + line, col)
            };
            entries.push((line, col, len, kind, mods));
        }
        // The same BIG-IP object-reference overlay a standalone `.irul` gets —
        // so `pool /Common/api_pool` inside a config's rule body reads as an
        // `object`, exactly as it would in its own file.  Spans come back
        // relative to the body, so shift them onto the document; the overlay
        // itself replaces the generic `string` the walk produced.
        for span in crate::irules_object_refs::object_ref_spans(body, registry) {
            let shifted = tcl_lexer::Span::new(
                span.start() + u32::try_from(bstart).unwrap_or(0),
                span.end() + u32::try_from(bstart).unwrap_or(0),
            );
            push_object_token(source, &line_index, shifted, &mut entries);
        }
    }
    for t in tcl_bigip::conf_tokens::tokenise_bigip_conf(source) {
        let kind = match t.kind {
            // BIG-IP-specific types.
            B::Partition => TokenKind::Partition,
            B::Pool => TokenKind::Pool,
            B::Monitor => TokenKind::Monitor,
            B::Profile => TokenKind::Profile,
            B::Vlan => TokenKind::Vlan,
            B::Interface => TokenKind::BigipInterface,
            B::IpAddress => TokenKind::IpAddress,
            B::Port => TokenKind::Port,
            B::RouteDomain => TokenKind::RouteDomain,
            B::Fqdn => TokenKind::Fqdn,
            B::Username => TokenKind::Username,
            B::Encrypted => TokenKind::Encrypted,
            B::Object => TokenKind::Object,
            // Shared primitives reuse the standard types.
            B::Comment => TokenKind::Comment,
            B::Keyword => TokenKind::Keyword,
            B::Property => TokenKind::OptionValue,
            B::Str => TokenKind::String,
            B::Escape => TokenKind::Escape,
            B::Number => TokenKind::Number,
        };
        let Some(text) = source.get(t.start as usize..t.end as usize) else {
            continue;
        };
        push_span_entries(
            source,
            &line_index,
            t.start as usize,
            text,
            kind,
            0,
            &mut entries,
        );
    }
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
    entries
}

/// Map the APL lexer's tokens onto legend entries, and walk each embedded Tcl
/// `[ … ]` region as Tcl.
///
/// APL embeds Tcl in bracket expressions, and the KCS feature doc is explicit
/// that they receive full Tcl highlighting.  The APL lexer leaves those spans
/// empty; they are re-walked here.
fn apl_entries(source: &str, registry: &CommandRegistry) -> Vec<Entry> {
    use tcl_bigip::apl::AplTokenKind as A;

    let line_index = LineIndex::new(source);
    let mut entries: Vec<Entry> = Vec::new();

    for (bstart, bend) in tcl_bigip::apl::embedded_tcl_regions(source) {
        let (bstart, bend) = (bstart as usize, bend as usize);
        let Some(body) = source.get(bstart..bend) else {
            continue;
        };
        let origin = line_index.position_at_utf16(u32::try_from(bstart).unwrap_or(0), source);
        for (line, col, len, kind, mods) in collect_entries(
            body,
            crate::profile_for_dialect(IAPPS_DIALECT),
            registry,
            None,
            WorkspaceTokenFacts::default(),
        ) {
            let (line, col) = if line == 0 {
                (origin.line, origin.character.get() + col)
            } else {
                (origin.line + line, col)
            };
            entries.push((line, col, len, kind, mods));
        }
    }
    for t in tcl_bigip::apl::tokenise_apl(source) {
        let kind = match t.kind {
            // APL-specific types.
            A::SectionKw => TokenKind::AplSection,
            A::FieldType => TokenKind::AplFieldType,
            A::Attribute => TokenKind::AplAttribute,
            A::SectionName => TokenKind::AplSectionName,
            A::FieldName => TokenKind::AplFieldName,
            A::Define => TokenKind::AplDefine,
            A::DefineName => TokenKind::AplDefineName,
            A::Directive => TokenKind::AplDirective,
            A::Optional => TokenKind::AplOptional,
            A::Validator => TokenKind::AplValidator,
            // Shared primitives reuse the standard types, so a theme that
            // styles Tcl strings/comments styles APL's the same way.
            A::Comment => TokenKind::Comment,
            A::Str => TokenKind::String,
            A::Number => TokenKind::Number,
            A::Variable => TokenKind::Variable,
            A::Operator => TokenKind::Operator,
            A::Escape => TokenKind::Escape,
        };
        let Some(text) = source.get(t.start as usize..t.end as usize) else {
            continue;
        };
        push_span_entries(
            source,
            &line_index,
            t.start as usize,
            text,
            kind,
            0,
            &mut entries,
        );
    }
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
    entries
}

/// Compute semantic tokens with an optional [`CompilationUnit`] for the same
/// document.
///
/// When `cu` is `Some`, a `regexp`/`regsub` pattern supplied through a
/// provably-constant string variable (`set my_re ".*abc"; regexp $my_re $s`)
/// causes the *originating* `set` literal to be highlighted as a regex — see
/// [`tcl_compiler::regex_source`].  With `cu == None` the result is identical
/// to the pure-segmentation tokenisation (the feature is simply absent), so
/// callers without an analysis pay nothing.
#[must_use]
pub fn full_with_cu(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
) -> SemanticTokens {
    full_with_cu_and_analysis(source, dialect, registry, cu, None)
}

/// [`full_with_cu`] with an optional [`AnalysisResult`], enabling precise
/// `$obj method …` / `[dict get $objs $k] method …` highlighting against
/// *user-defined* classes (their methods and `oo::configurable` properties),
/// resolved through the analyser's class hierarchy.  `None` falls back to the
/// registry-only object-method path.
#[must_use]
pub fn full_with_cu_and_analysis(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    analysis: Option<&AnalysisResult>,
) -> SemanticTokens {
    let proc_roles = analysis.map(VarNameArgRoles::from_analysis);
    let named_instances = analysis.map(named_instances_from_analysis);
    full_with_cu_and_facts(
        source,
        dialect,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes: analysis.map(AnalysisResult::class_hierarchy),
            proc_roles: proc_roles.as_ref(),
            named_instances: named_instances.as_ref(),
            project_classes: None,
            analysis,
        },
    )
}

/// [`full_with_cu`] with an optional [`ClassHierarchy`] — the workspace-merged
/// project class index, so a `$obj method …` dispatch resolves against a class
/// defined in *another* file.  The salsa / server path uses this; the local
/// single-file path goes through [`full_with_cu_and_analysis`].
#[must_use]
pub fn full_with_cu_and_classes(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
) -> SemanticTokens {
    full_with_cu_and_classes_and_roles(source, dialect, registry, cu, classes, None)
}

/// [`full_with_cu_and_classes`] with the workspace-merged inferred variable-name
/// argument roles ([`VarNameArgRoles`]), so a `myproc arr(key) …` call whose
/// `myproc` parameter aliases a caller variable highlights its array-element
/// target like `set arr(key) …`.  The project path
/// passes a cross-file index; `None` restricts the retag to the static registry
/// roles (plus source-derived stub roles).
#[must_use]
pub fn full_with_cu_and_classes_and_roles(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
    proc_roles: Option<&VarNameArgRoles>,
) -> SemanticTokens {
    full_with_cu_and_facts(
        source,
        dialect,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes,
            proc_roles,
            named_instances: None,
            project_classes: None,
            analysis: None,
        },
    )
}

/// [`full_with_cu_and_classes_and_roles`] with the workspace-merged named
/// bareword instance-command index bundled in, so `CLASS
/// create NAME` resolves its class exactly like the single-file
/// [`full_with_cu_and_analysis`] path does — the project token-aggregation
/// path (`semantic_tokens_project`) is the one caller with a project-wide
/// [`NamedInstanceMap`] to hand; `WorkspaceTokenFacts::default()` (every
/// other caller) skips the merge.
#[must_use]
pub fn full_with_cu_and_facts(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> SemanticTokens {
    let entries = collect_entries(source, dialect, registry, cu, facts);
    encode_entries(&entries)
}

/// Compute semantic tokens for `range` within the document.
/// Tokens whose start position falls outside the range are
/// dropped.  Delta encoding starts from the first surviving
/// token rather than the document origin, matching the LSP
/// spec for `semanticTokens/range`.
#[must_use]
pub fn range(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    range_with_cu(source, dialect, range, registry, None)
}

/// [`range`] with an optional [`CompilationUnit`] enabling regex-source
/// highlighting (see [`full_with_cu`]).
#[must_use]
pub fn range_with_cu(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
) -> SemanticTokens {
    range_with_cu_and_analysis(source, dialect, range, registry, cu, None)
}

/// [`range_with_cu`] with an optional [`AnalysisResult`] for user-class
/// object-method highlighting (see [`full_with_cu_and_analysis`]).
#[must_use]
pub fn range_with_cu_and_analysis(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    analysis: Option<&AnalysisResult>,
) -> SemanticTokens {
    let proc_roles = analysis.map(VarNameArgRoles::from_analysis);
    let named_instances = analysis.map(named_instances_from_analysis);
    range_with_cu_and_facts(
        source,
        dialect,
        range,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes: analysis.map(AnalysisResult::class_hierarchy),
            proc_roles: proc_roles.as_ref(),
            named_instances: named_instances.as_ref(),
            project_classes: None,
            analysis,
        },
    )
}

/// [`range_with_cu`] with an optional workspace-merged [`ClassHierarchy`] (see
/// [`full_with_cu_and_classes`]).
#[must_use]
pub fn range_with_cu_and_classes(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
) -> SemanticTokens {
    range_with_cu_and_classes_and_roles(source, dialect, range, registry, cu, classes, None)
}

/// [`range_with_cu_and_classes`] with the workspace-merged inferred
/// variable-name argument roles (see [`full_with_cu_and_classes_and_roles`]).
#[must_use]
pub fn range_with_cu_and_classes_and_roles(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
    proc_roles: Option<&VarNameArgRoles>,
) -> SemanticTokens {
    range_with_cu_and_facts(
        source,
        dialect,
        range,
        registry,
        cu,
        WorkspaceTokenFacts {
            classes,
            proc_roles,
            named_instances: None,
            project_classes: None,
            analysis: None,
        },
    )
}

/// [`range_with_cu_and_classes_and_roles`] with the workspace-merged named
/// bareword instance-command index bundled in (see [`full_with_cu_and_facts`]).
#[must_use]
pub fn range_with_cu_and_facts(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> SemanticTokens {
    let mut entries = collect_entries(source, dialect, registry, cu, facts);
    entries.retain(|(line, col, _, _, _)| {
        // Half-open interval per LSP `Range` semantics: start is
        // inclusive, end is exclusive.
        let pos = (*line, *col);
        let start = (range.start_line, range.start_character);
        let end = (range.end_line, range.end_character);
        pos >= start && pos < end
    });
    encode_entries(&entries)
}

/// One collected token: `(line, col, length, kind, modifiers)` with
/// absolute line/column and a token-modifier bitmask (see
/// [`legend_token_modifiers`]).
type Entry = (u32, u32, u32, TokenKind, u32);

/// Return the source offsets whose commands are admitted by the shared
/// iRules top-level declaration boundary.
///
/// Event handlers come from the registry/syntax owner directly.  Procedures
/// use the same top-level segmented command stream and registry-owned
/// declaration shape, because the syntax owner intentionally only exposes the
/// `when`-specific boundary record.  Recursive walks must consult these facts
/// rather than infer declaration placement from their own recursion depth:
/// command identity mutations and the iRules lexer grammar are document facts,
/// not properties of this token walk.
fn irules_top_level_declaration_heads(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    identities: &tcl_compiler::realm::CommandBindingRealm,
) -> FxHashSet<u32> {
    let mut heads: FxHashSet<u32> =
        tcl_registry::events::top_level_when_handler_candidates_with_registry_and_head_resolver(
            source, registry, identities,
        )
        .into_iter()
        .map(|handler| handler.span.start())
        .collect();

    // Read under the document's own registry and lexer configuration only.
    // The heads are consulted solely for a command the document's registry
    // marks `IS_EVENT_HANDLER` (see `original.rs`), so a process-wide iRules
    // store could only ever colour a document whose registry already has its
    // own event handler — where it would donate another registry's schema.
    // A plain Tcl buffer has no such command, and no `when` declaration role.
    for seg in segment_commands_with_offset_and_config(source, 0, config) {
        let Some(head_token) = seg.argv.first() else {
            continue;
        };
        let Some(head) = seg.texts.first() else {
            continue;
        };
        let resolved = tcl_registry::events::CommandHeadResolver::resolve(
            identities,
            head,
            head_token.span.start(),
        );
        let Some(closed) = tcl_registry::events::closed_braced_argument_words(
            source,
            seg.arg_tokens(),
            seg.arg_single_token(),
        ) else {
            continue;
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let Some(arguments) = tcl_registry::events::IrulesDeclarationArguments::new(
            &args,
            seg.arg_tokens(),
            seg.arg_single_token(),
            &closed,
        ) else {
            continue;
        };
        if matches!(
            registry.irules_top_level_declaration_shape(resolved.as_ref(), arguments),
            Some(tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. })
        ) {
            heads.insert(head_token.span.start());
        }
    }
    heads
}

/// True when `s` looks like an iRules event name (`^[A-Z][A-Z0-9_]+$`).
fn is_event_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2
        && bytes[0].is_ascii_uppercase()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_')
}

/// How a specific argument token should be classified, overriding the
/// default lexer-kind classification.
#[derive(Debug, Clone)]
enum ArgOverride {
    /// Classify the whole token as this kind (e.g. an event name).
    Kind(TokenKind),
    /// Sub-tokenise the token as a regex pattern (groups / classes /
    /// quantifiers / …); falls back to a single `regexp` token when the
    /// pattern has no metacharacters.
    RegexPattern,
    /// Sub-tokenise the token as a `format`/`scan` conversion string
    /// (`%[pos$][flags][width][.prec][len]type`); falls back to the
    /// default classification when it has no `%` specifiers.
    SprintfFormat,
    /// Sub-tokenise the token as a `clock format`/`scan` field string
    /// (`%Y` / `%Ey` / …); falls back to the default classification when
    /// it has no `%` specifiers.
    ClockFormat,
    /// Sub-tokenise the token as a `binary format`/`scan` field string
    /// (`a3` / `Su` / `c*` / …); falls back to the default classification
    /// when no specifier is recognised.
    BinaryFormat,
    /// Sub-tokenise the token as a `regsub` replacement spec (`\1`-`\9`
    /// → number, `\&` → operator); falls back to the default
    /// classification when it has no backreferences.
    RegsubReplace,
    /// Recurse into a braced command-body argument (`ArgRole::Body`),
    /// re-segmenting its inner script so nested commands / vars / strings
    /// are tokenised rather than emitted as one opaque `string`.
    BodyScript,
    /// Recurse into a braced expression argument (`ArgRole::Expr`),
    /// tokenising it via the expression sub-lexer (variables / numbers /
    /// operators / functions / nested `[cmd]` substitutions).
    ExprScript,
    /// A recognised `-option` switch → `Decorator`.
    Decorator,
    /// A variable name a command declares / writes (`ArgRole::VarWrite`) →
    /// `Variable` + `declaration` modifier.
    VarDecl,
    /// A variable name a command reads by reference (`ArgRole::VarRead` —
    /// `info exists arr(key)`, `array names arr`, `dict with $d`) → `Variable`
    /// with **no** `declaration` modifier: it references an existing variable
    /// rather than declaring one.
    VarRef,
    /// A command name passed as an argument (`ArgRole::CommandPrefix`, or a proc
    /// parameter the analyser inferred to be a `Command`) → `Function`: the
    /// literal names a command the callee invokes.
    CommandRef,
    /// The `{params body ?ns?}` lambda literal argument of a command
    /// carrying `ArgRole::LambdaLiteral` (`apply` today) — its second list
    /// element (the body) is re-segmented as a script.  Reached either as
    /// the call's own argument or, indirectly, through the `[list apply
    /// {…} $x]` deferred-command idiom — see
    /// the shared original produced-prefix owner.
    LambdaLiteral,
    /// A known subcommand word (arg index 1) → `Keyword` + `defaultLibrary`.
    SubcommandKeyword,
    /// The name argument of a `proc` definition → `Function` + `definition`.
    ProcNameDef,
    /// The braced clause-list argument of a `switch … { pat body … }` or an
    /// Expect `expect { ?-flags? pat body … }`: pattern elements are classified
    /// (as regexes when the shape says so) and body elements recursed as
    /// scripts.  Without this the whole `{ pat body … }` list would be walked as
    /// one literal word, leaving every body opaque and unhighlighted.  The
    /// [`CaseListSpec`] is registry data, so the walker names no command; the
    /// `bool` is whether *this call* put the list in regex mode
    /// (`switch -regexp`).
    CaseList(Box<tcl_registry::CaseListSpec>, bool),
    /// A structural keyword word at an argument position (`if`'s
    /// `then`/`elseif`/`else`, `try`'s `on`/`trap`/`finally`), carried
    /// by `ArgRole::Keyword` → highlighted as `Keyword` rather than a
    /// string.
    KeywordArg,
    /// The variable-spec word of a `foreach` / `lmap` / `dict for` loop — a
    /// single bareword (`foreach item …`) or a braced list of names
    /// (`foreach {k v} …`).  Each name is a variable the loop assigns on every
    /// iteration, so it is emitted as `Variable` + `declaration`.
    LoopVarList,
    /// A procedure parameter list (`proc p {a b {c 5} args} …`).  Each
    /// parameter name is emitted as `Parameter` + `declaration`; a `{name
    /// default}` pair emits the name as a parameter and classifies its default.
    ParamList,
    /// The declared name of a definition-body member (`method foo …`,
    /// `typemethod`, `property`), carried by `ArgRole::Name` → emitted as
    /// `Method` + `definition` rather than falling through to a plain string.
    MemberName,
    /// The class name at a *declaring* definer (`oo::class create Shape`,
    /// `snit::type Name`, `itcl::class Name`) → `Class` + `definition`.
    ClassNameDef,
    /// The class name at a *referencing* definer (`oo::define Shape`) → `Class`.
    ClassNameRef,
}

/// The inner content (delimiters stripped via `content_offset`) of a
/// braced/quoted literal token, plus its absolute byte start, or `None`
/// for a non-literal token / out-of-bounds span.  Shared by the
/// sub-language scanners.
///
/// Applies the same clamp-trim as [`push_token`]: the lexer extends a quoted
/// `Esc` fragment's span by one byte over the `$` / `[` that introduces the
/// *following* substitution (keeping `token_text` empty), so a fragment like
/// the `"$` of `"$x"` reports content `$`.  That introducer byte belongs to
/// the next `Var` / `Cmd` token; leaving it in would make a sub-language
/// scanner (regex, …) mis-read it (a `$` as an anchor) and overlap the
/// substitution token.  Trim it back to the leading delimiter here so every
/// consumer sees substitution-free literal content.
fn subspec_content(source: &str, tok: Token) -> Option<(usize, &str)> {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return None;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let mut cend = (tok.span.end() as usize).min(source.len());
    if tok.kind == TokenType::Esc
        && (tok.span.end() - tok.span.start()) == u32::from(tok.content_offset) + 1
        && source
            .as_bytes()
            .get(tok.span.end() as usize - 1)
            .is_some_and(|&b| b == b'$' || b == b'[')
    {
        cend = cstart.min(cend);
    }
    // An **empty** delimited word (`{}`, `""`) is the one shape whose
    // `span.end()` lands *past* its closing delimiter rather than at it, so the
    // closer would otherwise be handed back as the word's content.  For a body
    // argument that content is then re-segmented as a script, and the stray `}`
    // is classified as a command head — `proc p {args} {}` would emit
    // `'}':function`.  Recognised exactly (span length == content_offset + 1, last
    // byte is the matching closer), so a non-empty word ending in an *escaped*
    // quote (`"a\""`) is untouched.
    if tok.content_offset > 0
        && (tok.span.end() - tok.span.start()) == u32::from(tok.content_offset) + 1
        && closing_delimiter(source, tok.span.start())
            .is_some_and(|c| source.as_bytes().get(cend - 1) == Some(&c))
    {
        cend = cstart.min(cend);
    }
    source.get(cstart..cend).map(|inner| (cstart, inner))
}

#[derive(Clone, Copy)]
struct TokenPositionContext<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
}

/// Emit the literal run `inner[run..end]` (absolute start `cstart + run`)
/// as `kind`, when non-empty.  The inter-construct filler for the
/// sub-language scanners.
fn flush_run(
    pos: TokenPositionContext<'_>,
    cstart: usize,
    inner: &str,
    run: usize,
    end: usize,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if end > run {
        push_subtoken(
            pos.source,
            pos.line_index,
            cstart + run,
            &inner[run..end],
            kind,
            entries,
        );
    }
}

/// Sub-tokenise a `regsub` replacement spec: `\&` → `Operator`,
/// `\0`-`\9` → `Number`, literal runs → `String`.  Returns `false` when
/// there are no backreferences.  A direct backslash scan (no regex).
fn push_regsub_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos = TokenPositionContext { source, line_index };
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let next = bytes.get(i + 1).copied();
        if bytes[i] == b'\\' && next.is_some_and(|b| b.is_ascii_digit() || b == b'&') {
            flush_run(pos, cstart, inner, run, i, TokenKind::String, entries);
            // `\&` → operator (whole match); `\0`-`\9` → number (capture).
            let kind = if next == Some(b'&') {
                TokenKind::Operator
            } else {
                TokenKind::Number
            };
            push_subtoken(
                source,
                line_index,
                cstart + i,
                &inner[i..i + 2],
                kind,
                entries,
            );
            emitted = true;
            i += 2;
            run = i;
        } else {
            i += 1;
        }
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// Tag each literal (`Str`/`Esc`) fragment of the word spanning `word_span`
/// with `ov`, leaving `Var`/`Cmd` substitution fragments untouched (they fall
/// through to the default classifier).  A single-fragment literal word (a
/// braced `{a+b}` pattern, a plain `"abc"`) is tagged as one piece — the
/// common case — while a word interleaving literals and substitutions gets
/// each literal run tagged independently.
fn mark_literal_fragments(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    word_span: tcl_lexer::Span,
    ov: &ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    for t in &seg.all_tokens {
        if t.span.start() >= word_span.start()
            && t.span.end() <= word_span.end()
            && matches!(t.kind, TokenType::Str | TokenType::Esc)
        {
            overrides
                .entry(t.span.start())
                .or_insert_with(|| ov.clone());
        }
    }
}

/// Retag the literal fragments of any word the compiler flagged as a regex
/// source (`set pat {…}` that later feeds `regexp`/`regsub`) so they highlight
/// as regex.  Keyed on the def-site word start; the compiler-supplied span is
/// authoritative for the fragment scan (the segmenter's token span can clamp a
/// closing delimiter).
fn mark_regex_source_words(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    regex_sources: &FxHashMap<u32, tcl_lexer::Span>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if regex_sources.is_empty() {
        return;
    }
    for word in &seg.argv {
        if let Some(&full_span) = regex_sources.get(&word.span.start()) {
            mark_literal_fragments(seg, full_span, &ArgOverride::RegexPattern, overrides);
        }
    }
}

/// Whether an option value's role is re-coloured by another semantic-token
/// pass (`insert_role_overrides` for `Body`/`Expr`, `insert_var_decl_overrides`
/// for `VarWrite`, the retained format projection for a conversion string). Such
/// values must not be claimed as `OptionValue` by the option pass, which would
/// block the more specific role token.
fn role_claimed_by_token_pass(role: Option<tcl_registry::ArgRole>) -> bool {
    use tcl_registry::ArgRole;
    matches!(
        role,
        Some(
            ArgRole::Body
                | ArgRole::Expr
                | ArgRole::VarWrite
                | ArgRole::FormatString
                | ArgRole::ScanFormat
        )
    )
}

/// Whether a command's *head word* is a runtime-computed (non-static) command
/// name rather than a statically-resolvable one: a `$var` / `[cmd]`
/// substitution head, or a multi-fragment word (`chartV$node`, `${prefix}cmd`).
///
/// The command name of such a call is only known at runtime — an object handle
/// dispatched through a variable (`$chart method …`), a `[Class new]` /
/// `[dict get …]` constructor-or-lookup result, a computed command
/// name — so it must not be classified as a resolved command-head token, nor
/// consulted against the registry's declared option tables.  A plain
/// single-token bareword head (`puts`, a user proc) is *not* computed and stays
/// on the registry-precise path.
fn head_is_computed(seg: &tcl_compiler::segmenter::SegmentedCommand) -> bool {
    seg.argv
        .first()
        .is_some_and(|t| matches!(t.kind, TokenType::Var | TokenType::Cmd))
        || !seg.single_token_word.first().copied().unwrap_or(true)
}

/// Object-handle → class-name map for the current document, keyed by the
/// handle text a `$var method` dispatch presents (minus the leading `$`) —
/// a scalar (`chart`) or array element (`arr(key)`).  Built once per document
/// from the [`CompilationUnit`] by
/// [`tcl_compiler::object_types::object_handle_classes`].
type ObjectClassMap = std::collections::HashMap<String, std::collections::HashSet<String>>;

/// Named-instance display reports with separately retained compatibility mode.
/// Conditional original source rows never enter legacy label classification.
/// No report label supplies receiver identity, dispatch or Native authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NamedInstanceMap {
    rows: std::collections::HashMap<String, NamedInstanceDisplayRow>,
    ambiguous: std::collections::HashSet<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct NamedInstanceDisplayRow {
    class: String,
    compatibility: bool,
}
impl NamedInstanceMap {
    /// Preserve the existing analysis-gated compatibility projection. Genuine
    /// conditional source-card reports are added independently below.
    #[must_use]
    pub fn from_analysis(analysis: &AnalysisResult) -> Self {
        let mut result = Self::default();
        for (name, class) in &analysis.instance_classes {
            if analysis.created_instance_commands.contains(name.as_str()) {
                result.insert_report(name.clone(), class.clone(), true);
            }
        }
        result
    }

    /// Add display-only labels from genuine final source instance cards.
    /// This never enables a compatibility row or changes source selection.
    pub fn extend_conditional_source_cards(
        &mut self,
        cards: &tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards,
    ) {
        for (name, class) in cards.named_instance_display_reports() {
            self.insert_report(name, class.qualified_name.clone(), false);
        }
    }

    /// Merge report rows without upcasting a conditional row to compatibility.
    /// Independent compatible rows retain their previous mode; conflicts abstain.
    pub fn merge_reports(&mut self, other: &Self) {
        for name in &other.ambiguous {
            self.rows.remove(name);
            self.ambiguous.insert(name.clone());
        }
        for (name, row) in &other.rows {
            self.insert_report(name.clone(), row.class.clone(), row.compatibility);
        }
    }

    /// Whether the display report contains any represented named instance.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Display label lookup only; this cannot supply a receiver/source owner.
    #[must_use]
    pub fn get(&self, label: &str) -> Option<&str> {
        self.rows.get(label).map(|row| row.class.as_str())
    }

    /// Readonly report labels for presentation. Conditional mode is not a
    /// class/command resolver and no raw-map/Deref conversion is provided.
    pub fn report_rows(&self) -> impl Iterator<Item = (&str, &str)> {
        self.rows
            .iter()
            .map(|(name, row)| (name.as_str(), row.class.as_str()))
    }

    fn compatibility_rows(&self) -> impl Iterator<Item = (&String, &String)> {
        self.rows
            .iter()
            .filter(|(_, row)| row.compatibility)
            .map(|(name, row)| (name, &row.class))
    }

    fn insert_report(&mut self, name: String, class: String, compatibility: bool) {
        if self.ambiguous.contains(&name) {
            return;
        }
        match self.rows.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if entry.get().class != class {
                    entry.remove();
                    self.ambiguous.insert(name);
                } else if compatibility {
                    // Only an independently retained pre-existing compatible
                    // row can enable the legacy projection.
                    entry.get_mut().compatibility = true;
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(NamedInstanceDisplayRow {
                    class,
                    compatibility,
                });
            }
        }
    }
}

/// The optional workspace-merged facts a semantic-tokens request can enrich
/// its object-dispatch resolution with — [`ClassHierarchy`] (cross-file
/// classes), [`VarNameArgRoles`] (cross-file proc parameter roles), and
/// [`NamedInstanceMap`] (cross-file `CLASS create NAME` bindings).
/// Bundled into one `Copy` struct so the `range_*` full-arity entry
/// point stays within budget instead of growing a ninth positional
/// parameter.
#[derive(Clone, Copy, Default)]
pub struct WorkspaceTokenFacts<'a> {
    /// The workspace-merged class hierarchy, or the local single-file one.
    pub classes: Option<&'a ClassHierarchy>,
    /// Original project class/factory/instance display cards. Present means
    /// user-class roles require this typed owner, including after refusal.
    pub project_classes: Option<
        &'a [tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards],
    >,
    /// The workspace-merged (or local) inferred proc parameter roles.
    pub proc_roles: Option<&'a VarNameArgRoles>,
    /// The workspace-merged (or local) `CLASS create NAME` bareword
    /// instance-command index.
    pub named_instances: Option<&'a NamedInstanceMap>,
    /// Local analysis, when available. Package-require floors are document
    /// facts used to resolve lifecycle-gated registry methods.
    pub analysis: Option<&'a AnalysisResult>,
}

/// Build a [`NamedInstanceMap`] from `analysis` — `None` when `analysis`
/// created no named instances, so the merge in [`collect_entries`] is a
/// no-op lookup-free skip for the overwhelming majority of documents.
fn named_instances_from_analysis(analysis: &AnalysisResult) -> NamedInstanceMap {
    NamedInstanceMap::from_analysis(analysis)
}

/// Precise `$obj method …` highlighting via the registry's object-class model.
///
/// When the command head is an object handle whose class is known — a `$var`
/// bound by `set var [Class new]` (tracked by [`tcl_compiler::object_types`]),
/// or a direct `[Class new] method …` dispatch — and the class declares
/// `method`, the method word is highlighted as a function and the method's
/// declared options / option values are coloured exactly like a built-in's
/// (`Decorator` for the switch, `EnumMember` for a closed-set value, else
/// `OptionValue`).
///
/// Recognised method roles precede generic source options: a method's
/// options are claimed precisely first, and anything it leaves (an option not
/// in the spec, an un-provenanced receiver) is picked up by the generic
/// shape-based fallback.  A `$var` / `[cmd]` option value keeps its own
/// highlight; only literal (`Esc`/`Str`) values are recoloured.
fn insert_object_method_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    classes: Option<&ClassHierarchy>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if classes.is_some()
        && let Some(providers) = ctx.project_classes
        && receiver_source::insert_project_method(ctx, seg, providers, overrides)
    {
        return;
    }
    // Typed project mode cannot recover a refused source class from maps.
    let classes = ctx.project_classes.is_none().then_some(classes).flatten();
    let registry = ctx.registry;
    let object_classes = ctx.object_classes;
    let document_floor = ctx
        .analysis
        .map(|analysis| crate::document_floor::DocumentFloor::new(analysis, ctx.dialect));
    let dialect = Some(ctx.context.authoring_query());
    let (Some(head_tok), Some(head_text), Some(method)) =
        (seg.argv.first(), seg.texts.first(), seg.texts.get(1))
    else {
        return;
    };
    // Candidate receiver classes implied by the head's shape: a `$var` object
    // handle, a direct `[Class new] …` constructor, a `[dict get $coll $k]`
    // / `[lindex $coll $i]` retrieval resolved through the retained original
    // collection occurrences, or a bareword instance-command name bound by a
    // positional create call (`ttk::treeview .t` / a registry naming factory;
    // `object_classes` is name-keyed regardless of whether the name came from a
    // `set` LHS or a bareword factory, so the same map already carries these —
    // see `object_types::harvest_unit`'s `Statement::Call` arm).
    let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(ctx.full_source),
        ctx.config,
        seg,
    );
    let variable = original_words
        .word_exprs
        .first()
        .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
        .and_then(|(spelling, _)| object_handle_name(spelling, Some(ctx.config)));
    let mut candidates: Vec<String> = if let Some(name) = variable {
        object_classes
            .get(name)
            .map_or_default(|classes| classes.iter().cloned().collect())
    } else {
        match head_tok.kind {
            // The genuine whole receiver word, child source binding and selected
            // family determine conditional candidates. Missing canonical class
            // receipts remain typed unavailable inside the shared receiver path.
            TokenType::Cmd => match receiver_source::candidates(ctx, seg) {
                Ok(receiver_source::ReceiverSourceCandidates::ClassNames(classes)) => classes,
                Ok(receiver_source::ReceiverSourceCandidates::LogicalClass(class)) => {
                    class.insert_method(ctx, seg, classes, overrides);
                    return;
                }
                Ok(receiver_source::ReceiverSourceCandidates::CollectionElement) => {
                    receiver_source::insert_collection_method(ctx, seg, overrides);
                    return;
                }
                Err(_) => Vec::new(),
            },
            TokenType::Esc => object_classes
                .get(head_text.as_str())
                .map_or_default(|s| s.iter().cloned().collect()),
            _ => Vec::new(),
        }
    };
    if candidates.is_empty() {
        return;
    }
    // The candidate sets come from `HashSet` iteration, whose order varies per
    // map instance (random hash seed). When a receiver has several candidate
    // classes that resolve differently, an order-dependent pick would make the
    // incrementally-edited buffer and a fresh open disagree on the token — so
    // sort to a stable order before selecting (see the `edit_tracking_stress`
    // incremental-vs-fresh parity tests).
    candidates.sort_unstable();
    candidates.dedup();
    // 1. Registry-modelled class — precise, declared method options.
    if let Some(method_sub) = candidates.iter().find_map(|cls| {
        let package_version = registry
            .get(cls)
            .and_then(|spec| document_floor.and_then(|floor| floor.for_spec(spec)));
        registry.instance_method_at(cls, method, package_version, dialect)
    }) {
        mark_method_word(seg, overrides);
        insert_registry_method_options(seg, method_sub, overrides);
        return;
    }
    // 2. User-defined class — resolve the method through the class hierarchy
    //    (workspace-wide when a project index is supplied, so a class defined in
    //    another file resolves too); for a configurable receiver, colour the
    //    generated accessor's property options.
    if let Some(hierarchy) = classes
        && let Some(cls) = candidates
            .iter()
            .find(|c| user_class_provides_method(hierarchy, registry, c, method))
    {
        mark_method_word(seg, overrides);
        insert_user_configure_options(seg, hierarchy, registry, cls, method, overrides);
    }
}

/// Highlight a dispatched object method's name word (`seg.argv[1]`) as a
/// [`TokenKind::Method`].
///
/// The *call site* of a method (`$obj add …`, `my Cleanup`, `[Class new] m …`)
/// is the same entity as its declaration, so it takes the same type — a method
/// is not a free procedure.  This is the one place a dispatched
/// method name is typed, so declaration and call site cannot drift apart.
fn mark_method_word(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if let Some(mtok) = seg.argv.get(1) {
        overrides
            .entry(mtok.span.start())
            .or_insert(ArgOverride::Kind(TokenKind::Method));
    }
}

/// Apply a registry object-method's declared options to the words after the
/// method (`skip(2)`): the switch → [`ArgOverride::Decorator`], a closed-set
/// value → [`TokenKind::EnumMember`], else [`TokenKind::OptionValue`].
fn insert_registry_method_options(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    method_sub: &tcl_registry::SubCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    for (i, text) in seg.texts.iter().enumerate().skip(2) {
        // registry-axis-ok: irreducible — generic across every object
        // method's option set; only `switch` and `regexp` have
        // `OptionEffectKind::EndsOptions` populated on a "--" row today, so
        // reading `method_sub`'s own row here would stop recognising "--"
        // for virtually every other method (`formatting/keywords.rs`'s
        // `scan_options` has the same gap, for the same reason); until never
        if text == "--" {
            // End-of-options marker — colour it, then stop (Tcl convention).
            if let Some(tok) = seg.argv.get(i) {
                overrides
                    .entry(tok.span.start())
                    .or_insert(ArgOverride::Decorator);
            }
            break;
        }
        if !text.starts_with('-') {
            continue;
        }
        let Some(opt) = method_sub.options.iter().find(|o| o.matches(text)) else {
            continue;
        };
        if let Some(tok) = seg.argv.get(i) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
        // Colour the option's value word(s).  A value whose role is claimed by
        // a later token pass is left alone (parity with the command path).
        if opt.takes_value() && !role_claimed_by_token_pass(opt.value_role()) {
            let values = opt.value_values();
            for vi in opt.value_indices(&seg.texts, i) {
                let (Some(val_tok), Some(val_text)) = (seg.argv.get(vi), seg.texts.get(vi)) else {
                    continue;
                };
                if !matches!(val_tok.kind, TokenType::Esc | TokenType::Str) {
                    continue;
                }
                let kind =
                    if !values.is_empty() && values.iter().any(|v| v.value == val_text.as_str()) {
                        TokenKind::EnumMember
                    } else {
                        TokenKind::OptionValue
                    };
                overrides
                    .entry(val_tok.span.start())
                    .or_insert(ArgOverride::Kind(kind));
            }
        }
    }
}

/// Whether a *user-defined* class provides `method` for an instance dispatch:
/// the class hierarchy's MRO resolves it (a declared method on the class or an
/// ancestor), it is an `TclOO` builtin every instance answers (`destroy`), or
/// it is one the class system generates from the class's declared properties
/// — decided by the same registry-driven
/// [`ClassHierarchy::property_accessor_methods`] the analyser's W308
/// existence check uses, so colouring and diagnostics cannot disagree about
/// whether a method exists.  `hierarchy` is the local file's hierarchy or a
/// workspace-merged project index.
fn user_class_provides_method(
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
    method: &str,
) -> bool {
    if hierarchy.method_target(class, method).is_some() {
        return true;
    }
    // registry-axis-ok: irreducible — `oo::object`'s own `SUBCOMMANDS` doc
    // comment spells out why this is `destroy` alone: it is "the [only]
    // exported method" reachable through an instance's public name (`new`
    // and `create` are class-level, not instance methods, despite being
    // registered on the same command); no trait yet distinguishes
    // instance-exported from class-level registry subcommands generically,
    // so asking that query would take a new registry concept, not a
    // mechanical read of an existing one; until never
    if method == "destroy" {
        return true;
    }
    hierarchy.is_property_accessor(Some(registry), class, method)
}

/// Colour the `-property` options of a generated property-accessor dispatch
/// (`oo::configurable`'s `configure`) on a user class: a word `-<name>` whose
/// `name` is a property declared on the class or an ancestor becomes a
/// [`ArgOverride::Decorator`], and a following literal value an
/// [`TokenKind::OptionValue`].  A non-property `-word` is left to the generic
/// option fallback.  No-op for any method the class system does not generate
/// from properties.
fn insert_user_configure_options(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
    method: &str,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if !hierarchy.is_property_accessor(Some(registry), class, method) {
        return;
    }
    // Property names across the whole MRO (`-node`, `-name`, inherited …).
    let props: std::collections::HashSet<String> = hierarchy
        .mro_or_self(class)
        .iter()
        .filter_map(|c| hierarchy.classes.get(c))
        .flat_map(|cd| cd.properties.keys().cloned())
        .collect();
    if props.is_empty() {
        return;
    }
    for (i, text) in seg.texts.iter().enumerate().skip(2) {
        // registry-axis-ok: irreducible — `props` (above) is derived
        // entirely from the class's own declared properties, not from any
        // registered `OptionSpec` row — a generated property accessor has no
        // command-level "--" fact in the registry to read at all; until never
        if text == "--" {
            if let Some(tok) = seg.argv.get(i) {
                overrides
                    .entry(tok.span.start())
                    .or_insert(ArgOverride::Decorator);
            }
            break;
        }
        let Some(prop) = text.strip_prefix('-') else {
            continue;
        };
        if !props.contains(prop) {
            continue;
        }
        if let Some(tok) = seg.argv.get(i) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
        // The immediately-following literal word is this property's value.
        let next_word = seg.texts.get(i + 1).map(String::as_str);
        // registry-axis-ok: irreducible — same reason as this loop's other
        // "--" check above; until never
        let next_word_is_a_value = next_word.is_some_and(|w| !w.starts_with('-') && w != "--");
        if let Some(val_tok) = seg.argv.get(i + 1)
            && matches!(val_tok.kind, TokenType::Esc | TokenType::Str)
            && next_word_is_a_value
        {
            overrides
                .entry(val_tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::OptionValue));
        }
    }
}

/// Project a complete original variable spelling to its metadata-map name.
/// Supplied configuration validates the whole reference before decoration is
/// removed; the caller separately requires a sole substitution in its word.
/// An absent configuration is the explicit standalone decoration convention.
fn object_handle_name(head_text: &str, config: Option<tcl_lexer::LexerConfig>) -> Option<&str> {
    if !head_text.starts_with('$') {
        return None;
    }
    if let Some(config) = config {
        let reference =
            tcl_lexer::word_parts::whole_var_ref(head_text.as_bytes(), config).ok()??;
        if reference.next != head_text.len() {
            return None;
        }
        return Some(tcl_syntax::naming::var_reference_for_style(
            head_text,
            config.braced_var,
        ));
    }
    // Explicit standalone decoration metadata retains its prior convention.
    Some(tcl_syntax::naming::var_reference(head_text))
}

/// Resolve a class name *as written* at a definer head to a qualified key in
/// `hierarchy` through retained root source receipts. Missing or ambiguous
/// byte geometry withdraws the class identity.
fn resolve_class_in_hierarchy(hierarchy: &ClassHierarchy, name: &str) -> Option<String> {
    tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(name, &hierarchy.classes)
}

/// Resolve a self-call inside a class body against the enclosing class's MRO:
/// colour the method a callable, and — for `configure` / `cget` on an
/// `oo::configurable` class — its `-property` options.  The self-receiver is
/// `my` (`TclOO`), `[self]`/`[self object]` (`TclOO`), `$self`
/// (snit), or `$this` (itcl) — each of which dispatches on the enclosing
/// object.  No-op outside a class body, without a hierarchy, or for any
/// other head.
fn insert_self_method_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let (Some(hierarchy), Some(class_name), Some(head)) =
        (ctx.classes, ctx.enclosing_class, seg.argv.first())
    else {
        return;
    };
    let selected_self = original.and_then(|words| {
        words.with_source_schema(ctx.generation, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
        })
    }) == Some(true);
    let method_index = if selected_self {
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            original.and_then(|words| words.origins.as_slice().get(1))
        else {
            return;
        };
        *written
    } else {
        let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(ctx.full_source),
            ctx.config,
            seg,
        );
        let implicit_self = original_words
            .word_exprs
            .first()
            .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
            .and_then(|(spelling, _)| object_handle_name(spelling, Some(ctx.config)))
            .is_some_and(|name| {
                ctx.receiver_grammar
                    .is_some_and(|grammar| grammar.is_implicit_receiver_variable(name))
            });
        if !implicit_self && !original_self_accessor(ctx, *head) {
            return;
        }
        1
    };
    let Some(method) = seg.texts.get(method_index) else {
        return;
    };
    let Some(class) = resolve_class_in_hierarchy(hierarchy, class_name) else {
        return;
    };
    if !user_class_provides_method(hierarchy, ctx.registry, &class, method) {
        return;
    }
    if let Some(word) = seg.argv.get(method_index) {
        overrides.insert(word.span.start(), ArgOverride::Kind(TokenKind::Method));
    }
    // This presentation helper addresses direct written selector/options only.
    // Captures never borrow its positional argument coordinates.
    if method_index == 1
        && original.is_none_or(|words| {
            words
                .origins
                .as_slice()
                .iter()
                .enumerate()
                .all(|(ordinal, origin)| {
                    *origin
                        == tcl_compiler::registry_invocation::InvocationWordOrigin::Written(ordinal)
                })
        })
    {
        insert_user_configure_options(seg, hierarchy, ctx.registry, &class, method, overrides);
    }
}

/// A single genuine receiver substitution under the current original schema.
/// This is conditional source syntax, not the result of executing the child.
fn original_self_accessor(ctx: ScriptCtx<'_>, head: Token) -> bool {
    let Some(analysis) = ctx.analysis else {
        return false;
    };
    let regions =
        crate::executable_regions::command_substitution_regions(ctx.full_source, ctx.config, head);
    let [(start, end)] = regions.as_slice() else {
        return false;
    };
    let Ok(offset) = u32::try_from(*start) else {
        return false;
    };
    let commands =
        segment_commands_with_offset_and_config(&ctx.full_source[*start..*end], offset, ctx.config);
    let [command] = commands.as_slice() else {
        return false;
    };
    let Some(words) = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        ctx.full_source,
        analysis,
        command,
    ) else {
        return false;
    };
    words.with_source_schema(ctx.generation, |schema| {
        let spec = schema.authored_source_descriptors().command;
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match schema.words.arguments().exact_argv_len() {
            Some(0) => spec.arity.min == 0,
            Some(1) => schema
                .words
                .arguments()
                .get(0)
                .and_then(tcl_registry::InvocationWord::literal)
                .is_some_and(|word| spec.self_receiver_words.contains(&word)),
            _ => false,
        }
    }) == Some(true)
}

/// Paint literal name fragments only under the caller's retained variable role.
/// Original word geometry supplies display extents, independently of runtime
/// names, variable places, cells or successful argument evaluation.
fn retag_original_variable_argument(
    command: &tcl_compiler::segmenter::SegmentedCommand,
    original: &tcl_lexer::NativeWord,
    kind: ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_syntax::word_rules::OriginalVariableNameWordForm;
    if !original
        .tokens()
        .iter()
        .all(|token| command.all_tokens.contains(token))
    {
        return;
    }
    let Some(geometry) = tcl_syntax::word_rules::original_variable_name_word_geometry(original)
    else {
        return;
    };
    if matches!(geometry.form(), OriginalVariableNameWordForm::Dynamic) {
        return;
    }
    if original.group().kind == tcl_lexer::WordKind::Quoted
        || original.tokens().iter().any(|token| token.in_quote)
    {
        if matches!(geometry.form(), OriginalVariableNameWordForm::Static { .. })
            && let [token] = original.tokens()
            && token.kind == TokenType::Esc
        {
            overrides.entry(token.span.start()).or_insert(kind);
        }
        return;
    }
    for token in geometry.original_word().tokens() {
        let literal = token.kind == TokenType::Esc
            && geometry
                .literal_spans()
                .iter()
                .any(|span| span.start() <= token.span.start() && token.span.end() <= span.end());
        let braced = token.kind == TokenType::Str
            && matches!(geometry.form(), OriginalVariableNameWordForm::Static { .. });
        if literal || braced {
            overrides
                .entry(token.span.start())
                .or_insert_with(|| kind.clone());
        }
    }
}

/// Sub-tokenise a `binary format`/`scan` field string into its
/// specifiers: digit runs → `BinaryCount`, specifier letters →
/// `BinarySpec`, a `u` suffix after any specifier (Tcl 8.5+)
/// or a trailing `*` → `BinaryFlag`.  Whitespace and unrecognised
/// characters are skipped.  Returns `false` when nothing was emitted.
fn push_binary_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    dialect: &'static tcl_dialect::DialectProfile,
    entries: &mut Vec<Entry>,
) -> bool {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return false;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(source.len());
    let Some(inner) = source.get(cstart..cend) else {
        return false;
    };
    let profile = dialect;
    let allow_mod = tcl_cmd_core::binary::signedness_available(profile);
    let mut emitted = false;
    for spec in tcl_cmd_core::binary::specifiers(inner.as_bytes(), allow_mod) {
        push_subtoken(
            source,
            line_index,
            cstart + spec.letter_start,
            &inner[spec.letter_start..=spec.letter_start],
            TokenKind::BinarySpec,
            entries,
        );
        emitted = true;
        let count_start = spec
            .letter_start
            .saturating_add(1)
            .saturating_add(usize::from(spec.modifier.is_some()));
        if count_start > spec.letter_start + 1 {
            push_subtoken(
                source,
                line_index,
                cstart + spec.letter_start + 1,
                &inner[spec.letter_start + 1..count_start],
                TokenKind::BinaryFlag,
                entries,
            );
            emitted = true;
        }
        if count_start < spec.end {
            push_subtoken(
                source,
                line_index,
                cstart + count_start,
                &inner[count_start..spec.end],
                if spec.star {
                    TokenKind::BinaryFlag
                } else {
                    TokenKind::BinaryCount
                },
                entries,
            );
            emitted = true;
        }
    }
    emitted
}

/// Sub-tokenise a `clock format`/`scan` field string into its `%`
/// specifiers (`ClockPercent` + optional `ClockModifier` + `ClockSpec`),
/// literal runs classified as `string`.  Returns `false` when there are
/// no specifiers.
fn push_clock_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let pos = TokenPositionContext { source, line_index };
    let mut emitted = false;
    let mut run = 0usize;
    for spec in tcl_cmd_core::clock::specifiers(inner) {
        flush_run(
            pos,
            cstart,
            inner,
            run,
            spec.start,
            TokenKind::String,
            entries,
        );
        push_subtoken(
            source,
            line_index,
            cstart + spec.start,
            "%",
            TokenKind::ClockPercent,
            entries,
        );
        if spec.modifier.is_some() {
            push_subtoken(
                source,
                line_index,
                cstart + spec.start + 1,
                &inner[spec.start + 1..spec.start + 2],
                TokenKind::ClockModifier,
                entries,
            );
        }
        let letter_start = spec.end - 1;
        push_subtoken(
            source,
            line_index,
            cstart + letter_start,
            &inner[letter_start..spec.end],
            TokenKind::ClockSpec,
            entries,
        );
        emitted = true;
        run = spec.end;
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// Sub-tokenise a `format`/`scan` conversion string into its `%`
/// specifier components (`FormatPercent` / `FormatFlag` / `FormatWidth`
/// / `FormatSpec`), with literal runs classified as `string`.  Returns
/// `false` (emitting nothing) when there are no `%` specifiers.
fn push_sprintf_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    dialect: &'static tcl_dialect::DialectProfile,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos_ctx = TokenPositionContext { source, line_index };
    let profile = dialect;
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(cuts) = parse_sprintf_cuts(bytes, i, profile)
        {
            flush_run(pos_ctx, cstart, inner, run, i, TokenKind::String, entries);
            let mut pos = i;
            for (end, kind) in cuts {
                emit_part(pos_ctx, cstart, inner, &mut pos, end, kind, entries);
            }
            emitted = true;
            i = pos;
            run = i;
            continue;
        }
        i += 1;
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos_ctx,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// Parse one `%`-specifier at `b[start]` into its component
/// `(end, kind)` cuts (monotonic ends, consumed in order by
/// [`emit_part`]), or `None` when it isn't a valid conversion (no type
/// letter — the `%` is then a literal).
fn parse_sprintf_cuts(
    b: &[u8],
    start: usize,
    profile: &tcl_dialect::DialectProfile,
) -> Option<Vec<(usize, TokenKind)>> {
    let mut owner_end = start + 1;
    let owner_spec = tcl_syntax::format::parse_spec(b, &mut owner_end)?;
    // `%%` is a literal percent in Tcl format strings, not a value-bearing
    // conversion. The shared parser represents it as verb `%` for renderers.
    if owner_spec.verb == b'%' {
        return None;
    }
    // `%%` is a literal percent in Tcl format strings, not a value-bearing
    // conversion.  The shared parser represents it as verb `%` so renderers
    // can handle it uniformly; semantic tokens must keep it as string text.
    if !tcl_cmd_core::format::is_verb(owner_spec.verb)
        || !tcl_cmd_core::format::is_available(&owner_spec, profile)
    {
        return None;
    }
    let n = b.len();
    let mut cuts: Vec<(usize, TokenKind)> = Vec::new();
    let mut j = start + 1;
    cuts.push((j, TokenKind::FormatPercent)); // `%`

    // Positional `<digits>$` (or `<digits>\$`).
    let pos_start = j;
    while j < n && b[j].is_ascii_digit() {
        j += 1;
    }
    if j > pos_start {
        let mut k = j;
        if b.get(k) == Some(&b'\\') {
            k += 1;
        }
        if b.get(k) == Some(&b'$') {
            cuts.push((j, TokenKind::FormatWidth)); // position digits
            cuts.push((k + 1, TokenKind::FormatPercent)); // `\`?`$`
            j = k + 1;
        } else {
            j = pos_start; // not positional — the digits are the width
        }
    }

    // Flags `[-+ 0#]*`.
    let flags_start = j;
    while j < n && matches!(b[j], b'-' | b'+' | b' ' | b'0' | b'#') {
        j += 1;
    }
    if j > flags_start {
        cuts.push((j, TokenKind::FormatFlag));
    }

    // Width `*` | digits.
    let width_start = j;
    if b.get(j) == Some(&b'*') {
        j += 1;
    } else {
        while j < n && b[j].is_ascii_digit() {
            j += 1;
        }
    }
    if j > width_start {
        let kind = digit_or_flag(b[width_start]);
        cuts.push((j, kind));
    }

    // Precision `.` then `*` | digits.  The separator is matched as a
    // literal `.` — the actual sprintf precision separator — not any
    // character, so a malformed `%5,3d` stays a plain string rather than
    // being mis-split into width-5 / precision-3 (highlighting only).
    if b.get(j) == Some(&b'.') {
        let value_start = j + 1;
        let mut k = value_start;
        if b.get(k) == Some(&b'*') {
            k += 1;
        } else {
            while k < n && b[k].is_ascii_digit() {
                k += 1;
            }
        }
        cuts.push((value_start, TokenKind::FormatFlag)); // the `.`
        if k > value_start {
            cuts.push((k, digit_or_flag(b[value_start])));
        }
        j = k;
    }

    // Length modifier `[hlLzq]`.
    if j < n && matches!(b[j], b'h' | b'l' | b'L' | b'z' | b'q') {
        j += 1;
        cuts.push((j, TokenKind::FormatFlag));
    }

    // Conversion type — required.
    if owner_end > start && b.get(owner_end - 1) == Some(&owner_spec.verb) {
        cuts.push((owner_end, TokenKind::FormatSpec));
        Some(cuts)
    } else {
        None
    }
}

/// `FormatWidth` for a digit, `FormatFlag` for `*` (variable width/prec).
fn digit_or_flag(first: u8) -> TokenKind {
    if first.is_ascii_digit() {
        TokenKind::FormatWidth
    } else {
        TokenKind::FormatFlag
    }
}

/// Emit `inner[*pos..end]` (absolute offset `cstart + *pos`) as `kind`
/// and advance `*pos`, when non-empty.  The sub-token cursor helper for
/// [`push_sprintf_subtokens`].
fn emit_part(
    pos_ctx: TokenPositionContext<'_>,
    cstart: usize,
    inner: &str,
    pos: &mut usize,
    end: usize,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if end > *pos {
        push_subtoken(
            pos_ctx.source,
            pos_ctx.line_index,
            cstart + *pos,
            &inner[*pos..end],
            kind,
            entries,
        );
        *pos = end;
    }
}

/// Sub-tokenise a regex pattern token into ARE components (groups,
/// character classes, quantifiers, anchors, escapes, backreferences,
/// alternation), with the literal runs between them classified as
/// `regexp`.  Returns `false` (emitting nothing) when the token isn't a
/// braced/quoted literal or contains no metacharacters — the caller then
/// falls back to a single `regexp` token.
fn push_regex_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos_ctx = TokenPositionContext { source, line_index };
    let mut matched_any = false;
    let mut pos = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(end) = scan_are_token(bytes, i) {
            flush_run(pos_ctx, cstart, inner, pos, i, TokenKind::Regexp, entries);
            let kind = classify_regex_component(&inner[i..end]);
            push_subtoken(
                source,
                line_index,
                cstart + i,
                &inner[i..end],
                kind,
                entries,
            );
            matched_any = true;
            i = end;
            pos = i;
        } else {
            i += 1;
        }
    }
    if !matched_any {
        return false;
    }
    if pos < inner.len() {
        push_subtoken(
            source,
            line_index,
            cstart + pos,
            &inner[pos..],
            TokenKind::Regexp,
            entries,
        );
    }
    true
}

/// Recognise one ARE metacharacter construct starting at `b[i]`,
/// returning its exclusive end, or `None` when `b[i]` is a literal
/// character.
fn scan_are_token(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    match b[i] {
        b'(' => {
            if b.get(i + 1) != Some(&b'?') {
                return Some(i + 1); // group open
            }
            // non-capturing / lookaround open: `(?:` `(?=` `(?!` `(?>`
            if let Some(b':' | b'=' | b'!' | b'>') = b.get(i + 2) {
                return Some(i + 3);
            }
            // embedded flags `(?imsx-imsx)`
            let mut j = i + 2;
            while j < len
                && matches!(
                    b[j],
                    b'i' | b'm' | b'n' | b's' | b'x' | b'w' | b'p' | b'q' | b'-'
                )
            {
                j += 1;
            }
            // Closed flag group → the whole `(?…)`; else just `(`.
            if b.get(j) == Some(&b')') {
                Some(j + 1)
            } else {
                Some(i + 1)
            }
        }
        b')' | b'|' | b'^' | b'$' | b'.' => Some(i + 1),
        b'*' | b'+' | b'?' => Some(if b.get(i + 1) == Some(&b'?') {
            i + 2
        } else {
            i + 1
        }),
        b'[' => scan_are_class(b, i),
        b'{' => scan_are_brace_quant(b, i),
        b'\\' if i + 1 < len => scan_are_escape(b, i),
        _ => None,
    }
}

/// Scan a bracket expression `[…]` starting at `b[i] == '['`.
///
/// `[` optional `^` optional leading `]`, then members up to the closing `]`.
/// A member is a `\`-escape (ARE recognises backslash escapes inside brackets,
/// e.g. `[\d]`), or a POSIX / collating / equivalence **sub-bracket**
/// (`[:alpha:]`, `[.ch.]`, `[=a=]`) whose internal `]` does **not** close the
/// outer bracket — so `[[:alpha:]]` scans as one char class, matching the ARE
/// engine (and C Tcl), not `[[:alpha:]` + a dangling `]`.
fn scan_are_class(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let mut j = i + 1;
    if b.get(j) == Some(&b'^') {
        j += 1;
    }
    // A `]` immediately after `[` / `[^` is a literal member, not the close.
    if b.get(j) == Some(&b']') {
        j += 1;
    }
    while j < len && b[j] != b']' {
        if b[j] == b'[' && matches!(b.get(j + 1), Some(b':' | b'.' | b'=')) {
            // Sub-bracket `[X … X]` (X ∈ `:.=`): skip to the matching `X]`.
            let delim = b[j + 1];
            let mut k = j + 2;
            while k + 1 < len && !(b[k] == delim && b[k + 1] == b']') {
                k += 1;
            }
            if k + 1 < len {
                j = k + 2; // past the closing `X]`
            } else {
                return None; // unterminated sub-bracket → not a token
            }
        } else if b[j] == b'\\' && j + 1 < len {
            j += 2;
        } else {
            j += 1;
        }
    }
    (j < len).then_some(j + 1) // unterminated class → not a token
}

/// Scan a brace quantifier `{n}` / `{n,}` / `{n,m}` at `b[i] == '{'`.
fn scan_are_brace_quant(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let mut j = i + 1;
    let digits = j;
    while j < len && b[j].is_ascii_digit() {
        j += 1;
    }
    if j == digits {
        return None;
    }
    if b.get(j) == Some(&b',') {
        j += 1;
        while j < len && b[j].is_ascii_digit() {
            j += 1;
        }
    }
    (b.get(j) == Some(&b'}')).then_some(j + 1)
}

/// Scan a backslash escape at `b[i] == '\\'` (caller guarantees `i + 1`
/// is in bounds): a two-char class/anchor/backref/escaped-metachar, or a
/// `\xHH` / `\uHHHH` / `\UHHHHHHHH` hex escape.
fn scan_are_escape(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let esc = b[i + 1];
    match esc {
        // class shortcuts / anchors / backref / escaped metachar /
        // escape sequence — all two characters.
        b'A'
        | b'b'
        | b'B'
        | b'd'
        | b'D'
        | b'm'
        | b'M'
        | b's'
        | b'S'
        | b'w'
        | b'W'
        | b'y'
        | b'Y'
        | b'z'
        | b'Z'
        | b'0'..=b'9'
        | b'a'
        | b'e'
        | b'f'
        | b'n'
        | b'r'
        | b't'
        | b'v'
        | b'.'
        | b'*'
        | b'+'
        | b'?'
        | b'('
        | b')'
        | b'{'
        | b'}'
        | b'['
        | b']'
        | b'|'
        | b'^'
        | b'$'
        | b'\\' => Some(i + 2),
        // `\xHH` (1-2 hex), `\uHHHH` (1-4), `\UHHHHHHHH` (1-8).
        b'x' | b'u' | b'U' => {
            let max = match esc {
                b'x' => 2,
                b'u' => 4,
                _ => 8,
            };
            let mut j = i + 2;
            while j < len && j < i + 2 + max && b[j].is_ascii_hexdigit() {
                j += 1;
            }
            // Requires at least one hex digit, else not a token.
            (j > i + 2).then_some(j)
        }
        _ => None, // `\` before an unrecognised char → literal
    }
}

/// Classify a single ARE metacharacter run.
fn classify_regex_component(matched: &str) -> TokenKind {
    let bytes = matched.as_bytes();
    if matched.starts_with('[') {
        return TokenKind::RegexpCharClass;
    }
    if matched.starts_with('\\') && bytes.len() >= 2 {
        let ch = bytes[1];
        return if ch.is_ascii_digit() {
            TokenKind::RegexpBackref
        } else if matches!(
            ch,
            b'a' | b'e' | b'f' | b'n' | b'r' | b't' | b'v' | b'x' | b'u' | b'U'
        ) {
            TokenKind::RegexpEscape
        } else if matches!(ch, b'd' | b'D' | b's' | b'S' | b'w' | b'W') {
            TokenKind::RegexpCharClass
        } else if matches!(
            ch,
            b'b' | b'B' | b'm' | b'M' | b'y' | b'Y' | b'A' | b'z' | b'Z'
        ) {
            TokenKind::RegexpAnchor
        } else {
            TokenKind::RegexpEscape
        };
    }
    // registry-axis-ok: irreducible — ARE (`regexp`/`regsub` pattern-string)
    // metacharacters, not Tcl command syntax; `|`/`^`/`$` coincide with
    // `::tcl::mathop` operator spellings the same way `eq`/`ne`/`-` do
    // elsewhere in the tree (see the irreducible waivers on those); until
    // never
    match matched {
        "^" | "$" => TokenKind::RegexpAnchor,
        "|" => TokenKind::RegexpAlternation,
        "." => TokenKind::RegexpCharClass,
        // A group's *closer* is as much a group delimiter as its opener —
        // falling through to the quantifier catch-all below would paint every
        // `)` in the quantifier colour.
        ")" => TokenKind::RegexpGroup,
        _ if matched.starts_with('(') => TokenKind::RegexpGroup,
        _ => TokenKind::RegexpQuantifier,
    }
}

/// Push one regex sub-token at absolute byte offset `abs_off` covering
/// `text`.  Skips empty runs; a multi-line run is split into one entry per
/// covered line (see [`push_span_entries`]).
fn push_subtoken(
    source: &str,
    line_index: &LineIndex,
    abs_off: usize,
    text: &str,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    push_span_entries(source, line_index, abs_off, text, kind, 0, entries);
}

/// Emit token [`Entry`] values for `text` at absolute byte offset `abs_off`.
///
/// The LSP semantic-tokens encoding cannot represent a single token spanning
/// a newline (each token carries only a length, not an end position), so a
/// multi-line token is split into one entry per covered line, each covering
/// that line's slice of the token.  This keeps multi-line literals — braced
/// (`{…}`) or quoted (`"…"`) strings that span lines — highlighted
/// rather than dropped.  Empty per-line slices (blank lines, the trailing
/// slice after a final newline) are skipped, and the newline / `\r` bytes
/// themselves are never covered.
fn push_span_entries(
    source: &str,
    line_index: &LineIndex,
    abs_off: usize,
    text: &str,
    kind: TokenKind,
    modifiers: u32,
    entries: &mut Vec<Entry>,
) {
    if text.is_empty() {
        return;
    }
    if !text.contains('\n') {
        let pos = line_index.position_at_utf16(u32::try_from(abs_off).unwrap_or(0), source);
        entries.push((
            pos.line,
            pos.character.get(),
            utf16_len(text),
            kind,
            modifiers,
        ));
        return;
    }
    let mut off = 0usize;
    for line in text.split_inclusive('\n') {
        let seg = line.strip_suffix('\n').unwrap_or(line);
        let seg = seg.strip_suffix('\r').unwrap_or(seg);
        if !seg.is_empty() {
            let pos =
                line_index.position_at_utf16(u32::try_from(abs_off + off).unwrap_or(0), source);
            entries.push((
                pos.line,
                pos.character.get(),
                utf16_len(seg),
                kind,
                modifiers,
            ));
        }
        off += line.len();
    }
}

/// Maximum body / expr / command-substitution recursion depth — guards
/// against pathological nesting.
const MAX_TOKEN_RECURSION: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(32);

/// Emit the command-head token, splitting a namespace-qualified head
/// (`oo::class`, `::set`) into a `namespace` token for the leading
/// `…::` prefix plus a command token for the final segment.  A bare head
/// is emitted whole, carrying `defaultLibrary` when it resolves to a
/// registry built-in.
/// Sub-tokenise the braced case list of `switch … { pat body … }`.
///
/// The inner script is re-segmented into commands; the words are flattened
/// across all command lines and paired (even index → pattern, odd index →
/// body), since a Tcl `switch` case list is one flat list whose line breaks
/// are insignificant whitespace.  Body words are recursed as scripts.
/// Pattern words (except the literal `default`) are sub-tokenised as regexes
/// when `regexp` is set (`-regexp` mode), otherwise classified as ordinary
/// literals.
/// Immutable context threaded through the recursive script-tokenisation
/// walk.  Bundling these read-only borrows keeps each recursive helper to a
/// small, focused signature (the mutable `entries` sink and the `depth`
/// guard stay explicit parameters).
/// Per-walk facts every [`ScriptCtx`] of one token walk shares.
struct WalkFacts<'a> {
    /// The source shell this walk reads. Nothing was interpreted, so the
    /// context's `analysis` is `None` and every command reads its written
    /// words.
    shell: Option<&'a AnalysisResult>,
    /// Written command tails the analysed document itself rebinds (a
    /// procedure, an alias, either end of a `rename`). A head the analysis has
    /// no proof for keeps its descriptor's colours unless it is one of these.
    rebound_tails: FxHashSet<Vec<u8>>,
    /// The script regions this walk entered, with their nesting depth. Their
    /// commands are coloured as script, so their comments are comments.
    entered_scripts: std::cell::RefCell<Vec<(Span, u32)>>,
    /// The analysed formals of every procedure declaration, keyed by the
    /// extent of the first token of its parameter word. Built on first use.
    procedure_formals: std::cell::OnceCell<
        FxHashMap<
            (u32, u32),
            tcl_compiler::signature_scan::formal_parameters::SignatureSourceFormalParameters,
        >,
    >,
}

impl<'a> WalkFacts<'a> {
    fn new(shell: Option<&'a AnalysisResult>, analysis: Option<&AnalysisResult>) -> Self {
        let rebound_tails = analysis.map_or_else(FxHashSet::default, |analysis| {
            analysis
                .all_procs
                .keys()
                .chain(analysis.command_aliases.keys())
                .chain(analysis.renamed_commands.keys())
                .chain(analysis.renamed_commands.values())
                .map(|name| tcl_syntax::naming::written_command_tail(name.as_bytes()).to_vec())
                .collect()
        });
        Self {
            shell,
            rebound_tails,
            entered_scripts: std::cell::RefCell::new(Vec::new()),
            procedure_formals: std::cell::OnceCell::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct ScriptCtx<'a> {
    full_source: &'a str,
    dialect: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
    context: &'a tcl_registry::model::ResolvedContext,
    generation: &'a tcl_registry::model::ContextRegistry,
    operand: Option<&'a crate::original_invocation::OriginalOperandSource>,
    original_words: Option<&'a crate::original_invocation::OriginalRegistryWords>,
    declared_words: Option<&'a tcl_compiler::command_binding::OriginalDeclaredCommandWords>,
    /// The enclosing definition body was opened by a command with no retained
    /// invocation, so its member words are read from the grammar alone.
    written_members: bool,
    /// Facts fixed for the whole walk.
    walk: &'a WalkFacts<'a>,
    lexical: bool,
    /// The dialect's numeric-literal grammar, resolved once from `dialect` so
    /// the per-word `Number` classification does not re-walk the profile
    /// catalogue for every token.  Decides which radix prefixes exist, whether a
    /// bare leading zero is octal, and whether `_` digit separators are
    /// allowed — see [`tcl_syntax::number::is_whole_number`].
    numbers: NumberSyntax,
    registry: &'a CommandRegistry,
    line_index: &'a LineIndex,
    /// The enclosing definition-body grammar, or `None` outside any
    /// definition body.  When `Some`, this script is a class/type definition
    /// body (an `oo::class create … { … }`, `snit::type … { … }`, or bare
    /// `oo::define … { … }` block) and the grammar's member sub-keywords
    /// (`method`, `typemethod`, `constructor`, `variable`, …) carry the script
    /// bodies / parameter lists / variable declarations to recurse and
    /// highlight — see [`crate::oo_body`].  Outside one, a same-named user proc
    /// is never treated as a member.
    oo_grammar: Option<&'static DefinitionBodyGrammar>,
    /// Selected source member receiver vocabulary, without an entered frame.
    receiver_grammar: Option<&'static DefinitionBodyGrammar>,
    /// Member vocabulary retains its actual source schema and whole body.
    original_definition_parent:
        Option<&'a tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
    /// Complete original argv owner for this current definition-source region.
    original_definition_members:
        Option<&'a tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberRegion>,
    /// The enclosing scoped command environment, or `None` outside any scoped
    /// body.  When `Some`, this script runs in a context (a `report::defstyle`
    /// style script) that exposes a curated command set (`top`, `data`,
    /// `columns`, …) — the heads highlight as library commands and their
    /// ensemble operations (`top set`) as subcommand keywords, resolved from
    /// registry data (see [`tcl_registry::scoped`]).  Persists into nested
    /// bodies and command substitutions inside the scoped body.
    scoped_env: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    /// Def-site literal value words to highlight as regex (regex-source
    /// tracking), keyed by word start.  Empty when disabled.
    regex_sources: &'a FxHashMap<u32, Span>,
    /// Object-handle → class-name provenance for the whole document, so a
    /// `$var method …` dispatch can resolve the method's options through the
    /// registry's object-class model.  Empty when no
    /// [`CompilationUnit`] is available or the document creates no tracked
    /// object handles.
    object_classes: &'a ObjectClassMap,
    /// Class hierarchy, when available — the MRO + `ClassDef`s (methods +
    /// `oo::configurable` properties) used to resolve a dispatched method
    /// against a *user* class, not just a registry-modelled one.  This is the
    /// current file's hierarchy, or a workspace-merged project index so a class
    /// defined in another file resolves too.  `None` for
    /// the pure-segmentation path.
    classes: Option<&'a ClassHierarchy>,
    project_classes: Option<
        &'a [tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards],
    >,
    /// The local analysis when this request has one, used only for document
    /// package floors on lifecycle-gated registry methods.
    analysis: Option<&'a AnalysisResult>,
    /// Original collection insertion/extraction occurrences of the same
    /// compilation unit, keyed by source occurrence rather than variable text.
    /// `None` without a [`CompilationUnit`] or a source-receiver consumer.
    original_collections: Option<&'a tcl_compiler::object_types::OriginalObjectCollectionSources>,
    original_roles: &'a original::OriginalTokenRoles,
    proc_roles: Option<&'a VarNameArgRoles>,
    /// The class whose definition body we are currently inside (as written at
    /// the `oo::class create NAME` / `oo::define NAME` head), sliced from the
    /// source so it lives as long as the walk.  Lets a `my method …` self-call
    /// in a method body resolve against the enclosing class's MRO — the single
    /// most common `TclOO` dispatch form.  `None` outside any class body.
    enclosing_class: Option<&'a str>,
    /// Source offsets admitted by the shared iRules declaration-boundary
    /// owner. Empty outside the iRules overlay.
    irules_top_level_declaration_heads: &'a FxHashSet<u32>,
}

/// Emit one clause-list *pattern* element.
///
/// A keyword pattern (`default`; Expect's `timeout` / `eof` / `full_buffer`)
/// matches no text — it is a keyword, never a regex and never a string.
/// Otherwise a regex-mode pattern is sub-tokenised as a regex, and an
/// exact/glob one is classified as an ordinary literal.
fn push_case_pattern(
    ctx: ScriptCtx<'_>,
    pat_tok: Token,
    text: &str,
    spec: &tcl_registry::CaseListSpec,
    regexp: bool,
    entries: &mut Vec<Entry>,
) {
    let line_index = ctx.line_index;
    let full_source = ctx.full_source;
    if spec.keyword_patterns.contains(&text) {
        push_token(
            line_index,
            full_source,
            pat_tok,
            TokenKind::Keyword,
            0,
            entries,
        );
    } else if regexp {
        if !push_regex_subtokens(line_index, full_source, pat_tok, entries) {
            push_token(
                line_index,
                full_source,
                pat_tok,
                TokenKind::Regexp,
                0,
                entries,
            );
        }
    } else if let Some(kind) = classify_arg_token(pat_tok, full_source, ctx.numbers) {
        push_token(line_index, full_source, pat_tok, kind, 0, entries);
    }
}

fn collect_case_list(
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
    depth: u32,
    spec: &tcl_registry::CaseListSpec,
    regexp: bool,
    written: bool,
) {
    if !ctx.lexical
        && !written
        && !ctx
            .operand
            .and_then(|operand| operand.word.as_ref())
            .is_some_and(|word| {
                word.group().kind == tcl_lexer::WordKind::Braced
                    && word
                        .tokens()
                        .first()
                        .is_some_and(|first| first.span == tok.span)
            })
    {
        classify_and_push_if(true, ctx, tok, entries);
        return;
    }
    if MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        return;
    };

    // The clause split is `tcl-syntax`'s, shared with the iRules
    // object-reference walker: if the two disagreed about where a clause body
    // is, they would disagree about what the code says.  It also handles
    // Expect's clause-leading flags (`-re`, `-timeout 5`), which strict
    // pattern/body alternation would let shift every following element by one.
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: spec.clause_flags,
        clause_value_flags: spec.clause_value_flags,
    };

    // Rebuild each element as a `Token` following the lexer's inner-end +
    // `content_offset` convention (`span.end()` sits at the closing `}`/`"`;
    // `content_offset` strips the opener) so the downstream helpers work
    // unchanged.
    let as_token = |e: tcl_syntax::case_list::Element| {
        let range = e.token_range();
        let (kind, content_offset) = if e.braced {
            (TokenType::Str, 1u8)
        } else if inner.as_bytes().get(e.start) == Some(&b'"') {
            (TokenType::Esc, 1u8)
        } else {
            (TokenType::Esc, 0u8)
        };
        Token::with_content_offset(
            kind,
            tcl_lexer::Span::new(
                u32::try_from(cstart + range.start).unwrap_or(0),
                u32::try_from(cstart + range.end).unwrap_or(0),
            ),
            content_offset,
        )
    };

    for clause in
        tcl_syntax::case_list::split_case_list_with_syntax(inner, &shape, ctx.config.list_parse)
    {
        let mut clause_regexp = regexp;
        for f in &clause.flags {
            let text = inner.get(f.start..f.end).unwrap_or_default();
            // The descriptor permits unique prefixes (`-re` for `-regexp`),
            // so compare the resolved canonical flag rather than the spelling
            // that appeared in the clause list.
            let flag = shape.resolve_flag(text);
            if spec.clause_regex_flag == flag {
                clause_regexp = true;
            }
            let ftok = as_token(*f);
            // A flag word is a decorator; its *value* word (`-timeout 5`) takes
            // its own literal classification.
            let kind = if flag.is_some() {
                Some(TokenKind::Decorator)
            } else {
                classify_arg_token(ftok, full_source, ctx.numbers)
            };
            if let Some(kind) = kind {
                push_token(line_index, full_source, ftok, kind, 0, entries);
            }
        }

        if let Some(p) = clause.pattern {
            let Some(text) = inner.get(p.content_range()) else {
                continue;
            };
            push_case_pattern(ctx, as_token(p), text, spec, clause_regexp, entries);
        }

        // Body element — recurse as a script.
        if let Some(b) = clause.body {
            let btok = as_token(b);
            if let Some((bstart, body)) = subspec_content(full_source, btok) {
                collect_script(
                    ctx,
                    body,
                    u32::try_from(bstart).unwrap_or(0),
                    entries,
                    depth + 1,
                    false,
                );
            } else if let Some(kind) = classify_arg_token(btok, full_source, ctx.numbers) {
                push_token(line_index, full_source, btok, kind, 0, entries);
            }
        }
    }
}

/// Recurse the body of an `ArgRole::LambdaLiteral` `{params body ?ns?}`
/// lambda literal (`apply`'s shape, reached directly or through the shared original
/// produced-prefix source receipt).
///
/// The braced lambda is a Tcl list; its second element is the body script
/// and is re-segmented so its commands / vars / strings tokenise like any
/// other body.  The first element (the argument list) and an optional third
/// (the namespace) are emitted with their default classification, so no part
/// of the lambda is dropped.  Mirrors C Tcl's `apply` lambda shape.
///
/// The body recurses in a *fresh* context (`oo_grammar` / `enclosing_class` /
/// `scoped_env` all cleared), never the enclosing command's: `apply`'s body
/// runs in a new call frame in the global namespace by default (or the
/// lambda's own optional third element, never inherited), so `my foo` inside
/// a bare `apply {{} {my foo}}` called from a method body is not actually a
/// call to that method at runtime — `my` isn't defined in `::`. Leaving the
/// enclosing class/grammar/scoped-env active here would resolve such a call
/// anyway, painting it as live when it would error (mirrors `folding.rs`'s
/// `None` reset for the same recursion, and the same fresh-frame reasoning the
/// interprocedural and param-trait passes apply).
fn collect_lambda_literal(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>, depth: u32) {
    original::collect_lambda(ctx, tok, entries, depth);
}

/// Emit the name(s) of a `foreach` / `lmap` / `dict for` variable spec as
/// variable declarations.  A single bareword is one name; a braced/quoted list
/// is flattened via the list grammar and each element name emitted separately.
/// A non-name element (a `$`-computed word, an array element) keeps a plain
/// `string` classification so nothing is dropped and it does not masquerade as
/// a variable.
fn collect_loop_var_list(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    if !original::collect_loop_variables(ctx, tok, entries) {
        classify_and_push_if(true, ctx, tok, entries);
    }
}

/// Emit a procedure parameter list's names as variable declarations.  Each
/// top-level list element is either a bareword parameter name or a `{name
/// ?default...?}` pair; the name is a `Variable` declaration and any default
/// words are classified (number / string).  A non-name element is left to the
/// default classifier.
fn collect_param_list(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    if !original::collect_parameter_fields(ctx, tok, entries) {
        classify_and_push_if(true, ctx, tok, entries);
    }
}

/// Emit the command-head token for a *static* head word — a resolvable command
/// name, painted as a single function / keyword / namespace token.
///
/// A *computed* head — `$obj method …`, `[dict get …] method …`, `[Class new]
/// method …`, a multi-fragment `chartV$node` — is not a command name we can
/// resolve, so it must not be painted as one; the caller gates on
/// [`head_is_computed`] and lets those tokens fall through to the ordinary
/// argument path, where a `[…]` recurses into its inner script and a `$var`
/// reads as a variable — an accurate picture of the runtime dispatch rather
/// than a misleading command highlight.
fn emit_static_command_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    entries: &mut Vec<Entry>,
) {
    let Some(&head_tok) = seg.argv.first() else {
        return;
    };
    let (kind, mods) = if ctx.original_roles.head_names_class(ctx, seg) {
        (TokenKind::Class, 0)
    } else if original::definition_member_head(ctx, seg) {
        (TokenKind::Keyword, 0)
    } else {
        unrecorded_head_kind(ctx, seg)
    };
    push_token(
        ctx.line_index,
        ctx.full_source,
        head_tok,
        kind,
        mods,
        entries,
    );
}

/// Segment `text` (anchored at absolute byte `base_offset` within
/// `full_source`) into commands and push a semantic-token [`Entry`] for each
/// token, recursing into braced bodies (`ArgRole::Body`), braced expressions
/// (`ArgRole::Expr`), and `[…]` command substitutions.  Token spans are
/// already absolute (the segmenter shifts them by `base_offset`), so positions
/// and text are resolved against `full_source` + `line_index`.
///
/// `deferred_role` is `true` only when this call's *entire* `text` is the
/// content of a `[…]` substitution whose own enclosing argument slot (in the
/// command containing it) carries `Body` / `LambdaLiteral` / `CommandPrefix`
/// — i.e. a position whose value is later invoked or sourced, not merely
/// computed. It gates the original produced-prefix projection and is otherwise `false`: every other recursion (the top-level
/// script, a body, a lambda body, a case-list clause, an expression) processes
/// source that is *itself* executed code, not a value that might or might not
/// be invoked later, so list-quoted detection inside it is decided fresh at the
/// next `[…]` hop rather than inherited.
fn collect_script(
    ctx: ScriptCtx<'_>,
    text: &str,
    base_offset: u32,
    entries: &mut Vec<Entry>,
    depth: u32,
    deferred_role: bool,
) {
    if MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    if let Ok(length) = u32::try_from(text.len()) {
        ctx.walk.entered_scripts.borrow_mut().push((
            Span::new(base_offset, base_offset.saturating_add(length)),
            depth,
        ));
    }
    let full_source = ctx.full_source;
    let definition_members = ctx.original_definition_parent.and_then(|parent| {
        let end = base_offset.checked_add(u32::try_from(text.len()).ok()?)?;
        parent.definition_member_region(ctx.generation, Span::new(base_offset, end))
    });
    let ctx = ScriptCtx {
        original_definition_members: definition_members.as_ref(),
        ..ctx
    };
    for seg in
        segment_commands_with_offset_and_config(text, base_offset, ctx.config.at_depth(depth))
    {
        // A cooperative stop point only: a cancelled host read unwinds here.
        tcl_compiler::analysis_control::unwind_point(
            tcl_compiler::analysis_control::AnalysisCheckpoint::Command,
        );
        if seg.argv.is_empty() {
            continue;
        }
        // Classify the command-head token.  A head that resolves to a registry
        // built-in carries the `defaultLibrary` modifier.
        let head_tok = seg.argv[0];
        let head_text = &seg.texts[0];
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        // Source descriptors retain their independently selected domain. Logical
        // advice uses the same original word/context owner as other source roles.
        let original = ctx.analysis.and_then(|analysis| {
            crate::original_invocation::source_registry_words(full_source, analysis, &seg)
        });
        let member_head = original::definition_member_head_word(ctx, &seg)
            .and_then(|word| Some((word, word.content_span().ok()?)));
        // A complete original member word can prove static spelling even when
        // its physical escape tokens form several lexical fragments.
        let computed_head = member_head.is_none()
            && head_is_computed(&seg)
            && original
                .as_ref()
                .is_none_or(|words| original::original_logical_head(ctx, words).is_none());
        let static_head_extent = member_head.map_or(head_tok.span, |(word, _)| word.word_span());
        if !computed_head {
            emit_selected_command_head(ctx, &seg, original.as_ref(), member_head, entries);
        }

        // The command's argument words (head excluded), borrowed once as
        // `&[&str]` and shared by every registry-driven pass below — the
        // override builder and the OO-body context check both need it, and
        // the registry API takes `&[&str]`, so building it here keeps the
        // hot path to a single bridging allocation per command.
        let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();

        let member_bodies = original::definition_member_bodies(ctx, &seg);
        let original_bodies = original
            .as_ref()
            .map_or_else(Vec::new, |words| words.source_script_bodies(ctx.generation));
        let mut overrides =
            command_role_overrides(ctx, &seg, original.as_ref(), member_bodies.as_deref());
        let member_parameter_operands = original::definition_member_parameter_operands(ctx, &seg);
        let declared = ctx.analysis.and_then(|analysis| {
            tcl_compiler::registry_invocation::source_structure::source_declared_command_words(
                full_source,
                analysis,
                &seg,
            )
        });
        if let Some(declared) = &declared {
            overrides.extend(original::declared_overrides(&seg, declared));
        }
        let procedure_role_extents = insert_original_proc_role_overrides(ctx, &seg, &mut overrides);
        // A deferred operand may retain the exact future source words of a
        // selected original list builder. The shared receipt owns both parent
        // and builder schemas; inert data has no deferred parent purpose.
        let produced = deferred_produced_command_words(ctx, &seg, deferred_role);
        if let Some(produced) = &produced {
            merge_list_quoted_command_overrides(&seg, ctx, produced, &mut overrides);
        }
        // `my method …` inside a class body resolves against the enclosing
        // class's MRO (the most common `TclOO` dispatch form).
        if !ctx.lexical || !ctx.original_roles.has_head(head_tok.span.start()) {
            insert_self_method_overrides(ctx, &seg, original.as_ref(), &mut overrides);
        }
        ctx.original_roles
            .insert_overrides(ctx, &seg, &mut overrides);
        // Regex-source tracking: retag a `set` value word that feeds a regexp
        // pattern as a (substitution-aware) regex.
        mark_regex_source_words(&seg, ctx.regex_sources, &mut overrides);

        let body_ctx = command_body_context(ctx, &seg, original.as_ref(), head_text, &arg_texts);

        let deferred_role_starts =
            command_deferred_role_starts(original.as_ref(), declared.as_ref());

        let mut expanded_roles = original
            .as_ref()
            .map_or_else(Vec::new, original::expanded_role_extents);
        expanded_roles.extend(procedure_role_extents);
        collect_command_tokens(
            ctx,
            body_ctx,
            &seg,
            &CommandTokenSources {
                original: &original,
                produced: &produced,
                declared: &declared,
                member_bodies: &member_bodies,
                original_bodies: &original_bodies,
                member_parameter_operands: &member_parameter_operands,
                overrides: &overrides,
                deferred_role_starts: &deferred_role_starts,
                expanded_roles: &expanded_roles,
                computed_head,
                static_head_extent,
            },
            entries,
            depth,
        );
    }
}

struct CommandTokenSources<'a> {
    original: &'a Option<crate::original_invocation::OriginalRegistryWords>,
    produced: &'a Option<crate::original_invocation::OriginalRegistryWords>,
    declared: &'a Option<tcl_compiler::command_binding::OriginalDeclaredCommandWords>,
    member_bodies: &'a Option<
        Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
    >,
    original_bodies: &'a [tcl_compiler::registry_invocation::OriginalSourceScriptBody],
    member_parameter_operands: &'a [crate::original_invocation::OriginalOperandSource],
    overrides: &'a FxHashMap<u32, ArgOverride>,
    deferred_role_starts: &'a FxHashSet<u32>,
    expanded_roles: &'a [(Span, TokenKind, u32)],
    computed_head: bool,
    static_head_extent: Span,
}

fn emit_selected_command_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    member_head: Option<(&tcl_lexer::NativeWord, Span)>,
    entries: &mut Vec<Entry>,
) {
    let full_source = ctx.full_source;
    let head_tok = seg.argv[0];
    if member_head.is_none() && document_member(ctx, seg).is_some() {
        push_token(
            ctx.line_index,
            full_source,
            head_tok,
            TokenKind::Keyword,
            0,
            entries,
        );
    } else if ctx.lexical && original.is_none() && member_head.is_none() {
        emit_static_command_head(ctx, seg, entries);
    } else if let Some((_, span)) = member_head {
        push_token(
            ctx.line_index,
            full_source,
            Token { span, ..head_tok },
            TokenKind::Keyword,
            0,
            entries,
        );
    } else if ctx.original_roles.head_names_class(ctx, seg) {
        push_token(
            ctx.line_index,
            full_source,
            head_tok,
            TokenKind::Class,
            0,
            entries,
        );
    } else if let Some(words) = original {
        original::registry_head(ctx, seg, words, entries);
    } else {
        push_written_head(ctx, seg, entries);
    }
}

/// A written bareword head with no selected invocation. Its qualifier reads
/// as a namespace and its tail as the command, split where the naming owner
/// ends the qualifier, as a selected head is shown.
fn push_written_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    entries: &mut Vec<Entry>,
) {
    let head = seg.argv[0];
    let text = &seg.texts[0];
    let (kind, mods) = unrecorded_head_kind(ctx, seg);
    let tail = tcl_syntax::naming::written_command_tail(text.as_bytes());
    let split = text
        .len()
        .checked_sub(tail.len())
        .filter(|boundary| *boundary != 0 && !tail.is_empty())
        .filter(|_| {
            // A backslash sequence can end the name early (a NUL ends a C
            // command name), so only a plain spelling is split.
            head.kind == TokenType::Esc
                && seg.single_token_word.first() == Some(&true)
                && !text.contains('\\')
                && ctx.full_source.get(head.span.as_range()) == Some(text.as_str())
        })
        .and_then(|boundary| u32::try_from(boundary).ok())
        .and_then(|boundary| head.span.start().checked_add(boundary));
    let Some(split) = split else {
        push_token(ctx.line_index, ctx.full_source, head, kind, mods, entries);
        return;
    };
    let known = unrecorded_command_spec(ctx, seg).is_some();
    push_token(
        ctx.line_index,
        ctx.full_source,
        Token {
            span: Span::new(head.span.start(), split),
            ..head
        },
        TokenKind::Namespace,
        if known { MOD_DEFAULT_LIBRARY } else { 0 },
        entries,
    );
    push_token(
        ctx.line_index,
        ctx.full_source,
        Token {
            span: Span::new(split, head.span.end()),
            ..head
        },
        kind,
        mods,
        entries,
    );
}

/// A written member of the document's own grammar (a `.sslictcl` or `SpecTcl`
/// document), at the root or inside a member that opens a member grammar,
/// where no selected definition-member inventory covers the command.
fn document_member(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<&'static tcl_registry::definer::MemberSpec> {
    if ctx.original_definition_members.is_some()
        || seg.single_token_word.first() != Some(&true)
        || seg.argv.first()?.kind != TokenType::Esc
    {
        return None;
    }
    let grammar = ctx.oo_grammar?;
    if !ctx.written_members && ctx.registry.document_grammar()?.family != grammar.family {
        return None;
    }
    grammar.source_member_in(
        seg.texts.first()?.as_bytes(),
        Some(ctx.context.authoring_query()),
    )
}

/// Head colour of a command with no selected invocation: its descriptor's
/// kind when the spelling still names the document's own command.
fn unrecorded_head_kind(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> (TokenKind, u32) {
    let Some(traits) = unrecorded_command_spec(ctx, seg).map(|spec| spec.traits) else {
        return (TokenKind::Function, 0);
    };
    if traits.contains(tcl_registry::Traits::LANGUAGE_KEYWORD) {
        (TokenKind::Keyword, 0)
    } else if traits.contains(tcl_registry::Traits::OPERATOR_COMMAND) {
        (TokenKind::Operator, 0)
    } else {
        (TokenKind::Function, MOD_DEFAULT_LIBRARY)
    }
}

/// The document's own command for a written bareword head the analysis kept
/// no invocation for (a body it did not enter). Display only: the spelling
/// selects the descriptor, so it grants no binding.
fn unrecorded_command_spec(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<&'static tcl_registry::CommandSpec> {
    use tcl_compiler::realm::RealmBindingFact;
    let head = seg.argv.first()?;
    (head.kind == TokenType::Esc && seg.single_token_word.first() == Some(&true)).then_some(())?;
    let text = seg.texts.first()?;
    let Some(analysis) = ctx.analysis else {
        // A source shell knows no rebinding: the spelling names the command.
        ctx.walk.shell?;
        return ctx.generation.resolve_command(text);
    };
    let realm = analysis.retained_command_realm()?;
    let name = match realm.binding_at(text, head.span.start()) {
        RealmBindingFact::Unchanged => text.as_str(),
        RealmBindingFact::Command(name) => name,
        RealmBindingFact::Rebound => {
            // A site the interpretation dispatched and could not prove stays
            // unknown. One it never reached has no fact at all: the spelling
            // still names the document's own command for display, unless the
            // document itself rebinds that name.
            let tail = tcl_syntax::naming::written_command_tail(text.as_bytes());
            if realm.source_bindings_ref().reached_site(head.span.start())
                || ctx.walk.rebound_tails.contains(tail)
            {
                return None;
            }
            text.as_str()
        }
    };
    ctx.generation.resolve_command(name)
}

/// Registry roles of an unrecorded command, from its written spellings.
fn insert_unrecorded_command_roles(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if let Some(spec) = unrecorded_command_spec(ctx, seg) {
        original::insert_written_overrides(ctx, seg, spec, overrides);
    }
}

fn command_role_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    member_bodies: Option<
        &[tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody],
    >,
) -> FxHashMap<u32, ArgOverride> {
    let mut overrides = original.map_or_else(FxHashMap::default, |words| {
        original::registry_overrides(ctx, seg, words)
    });
    for body in member_bodies.unwrap_or_default() {
        if let [token] = body.original_container().tokens() {
            overrides.insert(token.span.start(), ArgOverride::BodyScript);
        }
    }
    if ctx.lexical {
        insert_lexical_user_overrides(ctx, seg, &mut overrides);
    } else if let Some(providers) = ctx.project_classes {
        // Native lexical/report overlays remain guarded by has_head. This
        // independent source-display branch requires the selected original
        // class/instance and preserves stronger role overrides on insertion.
        receiver_source::insert_project_method(ctx, seg, providers, &mut overrides);
    } else {
        insert_object_method_overrides(ctx, seg, ctx.classes, &mut overrides);
    }
    original::insert_definition_member_overrides(ctx, seg, &mut overrides);
    if let Some(member) = document_member(ctx, seg) {
        use tcl_registry::ArgRole;
        for &(ordinal, role) in member.arg_roles {
            let word = usize::from(ordinal) + 1;
            let Some(token) = seg.argv.get(word) else {
                continue;
            };
            let single = seg.single_token_word.get(word) == Some(&true);
            let kind = match role {
                ArgRole::Body if token.kind == TokenType::Str => ArgOverride::BodyScript,
                ArgRole::ParamList if single => ArgOverride::ParamList,
                ArgRole::Name if single => ArgOverride::MemberName,
                _ => continue,
            };
            overrides.entry(token.span.start()).or_insert(kind);
        }
    } else if original.is_none() {
        insert_generic_option_overrides(seg, &mut overrides);
        insert_unrecorded_command_roles(ctx, seg, &mut overrides);
    }
    overrides
}

/// Generic `-option` / option-value highlighting for a command with a
/// *computed* head not resolved by the registry — a `$obj method …` object
/// dispatch, a `[Class new] method …`, or a multi-fragment `chartV$node …`
/// head.
///
/// A *registered* command's declared option set is authoritative — `puts -foo`
/// stays a string because `puts` declares no `-foo`
/// ([`insert_option_and_subcommand_overrides`]).  A plain bareword head is a
/// (possibly user-defined) command name and is left to the registry too, so
/// `mycmd -foo` stays a string.  Only a computed head — where the real option
/// set lives on a method / ensemble the registry does not model — is treated as
/// the overwhelmingly-common `-switch value` shape: those pairs are coloured
/// like any built-in's.
///
/// A "clean option" is a single-token [`TokenType::Esc`] word for which
/// [`is_generic_option_word`] holds — `-<letter>…`, excluding substitution
/// forms, negative numbers (including the `-inf` / `-nan` special-float
/// literals), a bare `-`, and `--`.  The single-token check is what excludes
/// `-$var` / `-{$var}` / `-[cmd]`, which keep their variable / command
/// highlight.
///
/// Tcl's `--` end-of-options marker is honoured: `--` itself is coloured as an
/// option marker, and scanning stops there — every following word is a
/// positional operand, even if it reads like `-foo` (`$obj cfg -- -literal`
/// leaves `-literal` a plain string).
///
/// The word immediately following an option is recoloured [`TokenKind::OptionValue`]
/// when it is a literal (`Esc`/`Str`) that is not itself an option and not
/// `--` — a `$var` / `[cmd]` value keeps its own highlight, and a following
/// option stays an option.  Arity is unknown for an undeclared command, so at
/// most the single adjacent value word is claimed; a boolean option followed by
/// another option therefore claims no value.
fn insert_generic_option_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // Only a *computed* head — a `$var` / `[cmd]` substitution or a
    // multi-fragment word (`chartV$node`) — is treated as a runtime dispatch
    // (object handle, ensemble) whose `-switch value` pairs are options.  A
    // plain single-token bareword head is a (possibly user-defined) command
    // name; deferring to the registry there keeps a bareword call conservative
    // — `mycmd -foo` stays a string, a user command's `test … -body …` is not
    // mistaken for tcltest's, and an OO-body member (`property … -get {…}`)
    // keeps its recursed bodies — exactly as `puts -foo` stays a string.
    if !head_is_computed(seg) {
        return;
    }
    // The single-token literal (`Esc`) text of word `i`, or `None` for a
    // substitution / braced / multi-fragment word.
    let literal_word = |i: usize| -> Option<&str> {
        (seg.single_token_word.get(i).copied().unwrap_or(false)
            && seg
                .argv
                .get(i)
                .is_some_and(|t| matches!(t.kind, TokenType::Esc)))
        .then(|| seg.texts.get(i).map(String::as_str))
        .flatten()
    };
    let mut i = 1;
    while i < seg.texts.len() {
        let Some(text) = literal_word(i) else {
            i += 1;
            continue;
        };
        // `--` ends option processing (Tcl convention). Colour the marker, then
        // stop — nothing after it is an option.
        // registry-axis-ok: irreducible — this function only runs for a
        // computed head with no selected invocation, so there is no
        // per-command `EndsOptions` row here to read; until never
        if text == "--" {
            if let Some(tok) = seg.argv.get(i) {
                overrides
                    .entry(tok.span.start())
                    .or_insert(ArgOverride::Decorator);
            }
            break;
        }
        if !is_generic_option_word(text) {
            i += 1;
            continue;
        }
        if let Some(tok) = seg.argv.get(i) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
        // The immediately-following literal word is this option's value when it
        // is not itself an option and not the `--` marker.  A `$var` / `[cmd]`
        // value falls through the `Esc | Str` check and keeps its own highlight.
        let vi = i + 1;
        if let Some(val_tok) = seg.argv.get(vi)
            && matches!(val_tok.kind, TokenType::Esc | TokenType::Str)
            // registry-axis-ok: irreducible — same reason as this function's
            // other "--" check above: no `CommandSpec` exists for this head;
            // until never
            && literal_word(vi).is_none_or(|w| w != "--" && !is_generic_option_word(w))
        {
            overrides
                .entry(val_tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::OptionValue));
        }
        i += 1;
    }
}

/// Whether a literal word reads as a generic `-option` switch on an unknown
/// command head: a leading `-` then an ASCII letter, excluding the negative
/// special-float literals Tcl's parser accepts as numbers (`-inf`, `-Inf`,
/// `-infinity`, `-nan`, …).  A bare `-`, `--`, and ordinary negative numbers
/// (`-5`, `-1.6`) are already excluded by the "letter after the dash" rule.
fn is_generic_option_word(text: &str) -> bool {
    let Some(rest) = text.strip_prefix('-') else {
        return false;
    };
    if !rest.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) {
        return false;
    }
    // `-inf` / `-infinity` / `-nan` are negative floating-point values, not
    // options — Tcl's `expr` and numeric commands parse them as numbers.
    !(rest.eq_ignore_ascii_case("inf")
        || rest.eq_ignore_ascii_case("infinity")
        || rest.eq_ignore_ascii_case("nan"))
}

fn deferred_produced_command_words(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    deferred_role: bool,
) -> Option<crate::original_invocation::OriginalRegistryWords> {
    let full_source = ctx.full_source;
    deferred_role
        .then(|| {
            let analysis = ctx.analysis?;
            crate::original_invocation::source_produced_command_prefix_words(
                full_source,
                analysis,
                seg,
            )
        })
        .flatten()
}

fn command_body_context<'a>(
    ctx: ScriptCtx<'a>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    head_text: &str,
    arg_texts: &[&str],
) -> ScriptCtx<'a> {
    let full_source = ctx.full_source;
    // Receiver advice keeps its independent grammar. Member vocabulary
    // travels in the original body receipts below.
    let outer_grammar = original.and_then(|words| {
        words
            .with_source_schema(ctx.generation, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
    });
    // A command with no retained invocation opens the definition grammar its
    // written words select, so the member words inside read as members.
    let written_grammar = (!ctx.lexical && original.is_none())
        .then(|| {
            let spec = unrecorded_command_spec(ctx, seg)?;
            ctx.registry
                .resolve_invocation(spec.name, arg_texts, None)?
                .authored_source_definition_body_grammar()
        })
        .flatten();
    let written_members = written_grammar.is_some() || (ctx.written_members && original.is_none());
    let next_oo = if ctx.lexical || written_members {
        next_original_definition_grammar(
            ctx,
            head_text,
            arg_texts,
            outer_grammar.or(written_grammar),
        )
    } else {
        // A document grammar's member opens its own member grammar on every
        // profile; a class-system body keeps its selected source owner.
        ctx.oo_grammar.and_then(|current| {
            ctx.registry
                .authored_document_member_grammar(current, head_text)
        })
    };
    // The class whose body the recursion enters: a `oo::class create NAME`
    // (and the property-/instantiation-metaclasses) names it at argv[2], an
    // `oo::define NAME { … }` at argv[1].  Slice it from the source so it
    // outlives the walk; otherwise inherit the enclosing class (so a
    // `method …` body keeps its class).  Lets `my method …` in the body
    // resolve against that class.
    let next_class = if ctx.lexical {
        original
            .and_then(|words| original::definition_class_name(ctx, seg, words))
            .or(ctx.enclosing_class)
    } else {
        declared_class_for_body(ctx, seg).or(ctx.enclosing_class)
    };
    // The scoped command environment the recursion into THIS command's body
    // should carry: a command whose spec declares a `body_scope` switches it
    // on (`report::defstyle`'s style script); otherwise it persists so the
    // scoped commands stay resolvable inside nested control-flow bodies and
    // `[…]` substitutions within the style script.
    let next_scoped = ctx
        .analysis
        .and_then(|analysis| {
            analysis.original_scoped_body_in_source(
                &tcl_lexer::SourceImage::document(full_source),
                ctx.config,
                seg.span.start(),
            )
        })
        .map(tcl_compiler::registry_invocation::OriginalSourceScopedBody::environment)
        .or(ctx.scoped_env);
    ScriptCtx {
        oo_grammar: next_oo,
        written_members: written_members && next_oo.is_some(),
        enclosing_class: next_class,
        scoped_env: next_scoped,
        ..ctx
    }
}

fn command_deferred_role_starts(
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    declared: Option<&tcl_compiler::command_binding::OriginalDeclaredCommandWords>,
) -> FxHashSet<u32> {
    let mut deferred_role_starts = deferred_role_arg_starts(original);
    if let Some(declared) = declared {
        deferred_role_starts.extend(
            declared
                .supplied_argument_roles()
                .unwrap_or_default()
                .into_iter()
                .filter(|(_, role)| {
                    matches!(
                        role,
                        tcl_registry::ArgRole::Body
                            | tcl_registry::ArgRole::CommandPrefix
                            | tcl_registry::ArgRole::LambdaLiteral
                    )
                })
                .filter_map(|(ordinal, _)| {
                    declared
                        .argument_word(ordinal)?
                        .tokens()
                        .first()
                        .map(|token| token.span.start())
                }),
        );
    }

    deferred_role_starts
}

fn collect_command_tokens(
    ctx: ScriptCtx<'_>,
    body_ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    sources: &CommandTokenSources<'_>,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    let CommandTokenSources {
        original,
        produced,
        declared,
        member_bodies,
        original_bodies,
        member_parameter_operands,
        overrides,
        deferred_role_starts,
        expanded_roles,
        computed_head,
        static_head_extent,
    } = *sources;
    for tok in &seg.all_tokens {
        if original::emit_expanded_role_extents(ctx, *tok, expanded_roles, entries) {
            continue;
        }
        // Skip every token that falls inside a *static* head word — not
        // just the exact head token.  Such a head is one word that
        // `emit_static_command_head` already emitted as a single command token;
        // its sub-fragments (`ns::`, `cmd` for a `ns::cmd` head) also
        // appear in `all_tokens`, and emitting those would overlap the head
        // token (invalid — LSP clients reject overlapping semantic tokens).
        //
        // A *computed* head is deliberately NOT emitted by
        // `emit_static_command_head` (see above), so its tokens must flow through
        // the argument path here: the `[…]` head token recurses into its
        // inner script and the `$var` head token reads as a variable.
        if !computed_head
            && tok.span.start() >= static_head_extent.start()
            && tok.span.end() <= static_head_extent.end()
        {
            continue;
        }
        let token_ctx = ScriptCtx {
            operand: produced
                .as_ref()
                .or(original.as_ref())
                .and_then(|words| {
                    words
                        .operands
                        .iter()
                        .filter_map(Option::as_ref)
                        .find(|operand| operand.span == tok.span)
                })
                .or_else(|| {
                    member_parameter_operands
                        .iter()
                        .find(|operand| operand.span == tok.span)
                })
                .or_else(|| ctx.original_roles.operand_at(tok.span)),
            original_words: produced.as_ref().or(original.as_ref()),
            declared_words: declared.as_ref(),
            ..ctx
        };
        let definition_parent = if let Some(member_body) =
            member_bodies.iter().flatten().find(|body| {
                body.original_container()
                    .tokens()
                    .iter()
                    .any(|part| part.span == tok.span)
            }) {
            member_body.definition_parent().cloned()
        } else if let Some(body) = original_bodies.iter().find(|body| {
            body.original_container()
                .tokens()
                .iter()
                .any(|part| part.span == tok.span)
        }) {
            body.definition_parent_for(ctx.generation, ctx.original_definition_parent)
        } else {
            None
        };
        let token_body_ctx = ScriptCtx {
            receiver_grammar: matches!(
                overrides.get(&tok.span.start()),
                Some(ArgOverride::BodyScript)
            )
            .then(|| original::receiver_grammar_for_body(ctx, seg, original.as_ref(), *tok))
            .flatten(),
            original_definition_parent: definition_parent.as_ref(),
            original_definition_members: None,
            ..body_ctx
        };
        emit_arg_token(
            token_ctx,
            token_body_ctx,
            *tok,
            overrides.get(&tok.span.start()),
            deferred_role_starts.contains(&tok.span.start()),
            entries,
            depth,
        );
    }
}

/// User-procedure traits and entered OO source grammar are independent of
/// builtin source descriptors. They cannot recover a refused builtin head.
fn insert_lexical_user_overrides(
    ctx: ScriptCtx<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    insert_object_method_overrides(
        ctx,
        command,
        if ctx.original_roles.has_head(command.argv[0].span.start()) {
            None
        } else {
            ctx.classes
        },
        overrides,
    );
}

/// The analysed class one of this command's words is a body of. A method body
/// the analysis never entered still belongs to its class for display, so a
/// self-dispatch inside it reads that class's declared methods. This names no
/// receiver and grants no edit.
fn declared_class_for_body<'a>(
    ctx: ScriptCtx<'a>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
) -> Option<&'a str> {
    ctx.analysis?
        .class_body_spans
        .iter()
        .find(|(_, span)| {
            seg.argv
                .iter()
                .skip(1)
                .any(|token| token.span.start().abs_diff(span.start()) <= 1)
        })
        .map(|(name, _)| name.as_str())
}

/// Retain only the actual selected outer source grammar and the independent
/// enclosing member vocabulary. Member bodies leave definition context;
/// the descriptor's wrapper-block form keeps the enclosing vocabulary.
fn next_original_definition_grammar(
    ctx: ScriptCtx<'_>,
    head: &str,
    arguments: &[&str],
    outer: Option<&'static DefinitionBodyGrammar>,
) -> Option<&'static DefinitionBodyGrammar> {
    if let Some(grammar) = ctx
        .oo_grammar
        .and_then(|current| ctx.registry.authored_document_member_grammar(current, head))
    {
        return Some(grammar);
    }
    if outer.is_some() {
        return outer;
    }
    if let Some(grammar) = ctx.oo_grammar
        && let Some(member) = grammar.member(head)
    {
        return (member.wrapper_block_body
            && arguments
                .first()
                .is_some_and(|inner| !grammar.is_member(inner)))
        .then_some(grammar);
    }
    ctx.oo_grammar
}

fn insert_original_proc_role_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) -> Vec<(tcl_lexer::Span, TokenKind, u32)> {
    let Some(analysis) = ctx.analysis else {
        return Vec::new();
    };
    if let Some(index) = ctx.proc_roles
        && !index.project_source.is_empty()
        && let Some(prototype) = tcl_compiler::registry_invocation::source_structure::original_project_procedure_arguments(
            ctx.full_source, analysis, seg, &index.project_source,
        )
        && analysis.resolved_input.as_ref().is_some_and(|input| {
            prototype.matches_source(&tcl_lexer::SourceImage::document(ctx.full_source), input)
        })
    {
        let metadata = prototype.declaration().metadata();
        return insert_original_procedure_argument_role_geometry(
            seg, prototype.bindings(), prototype.arguments(), prototype.operands(),
            [&proc_var_write_indices(metadata), &proc_var_read_indices(metadata), &proc_command_indices(metadata)],
            overrides,
        );
    }
    if ctx.lexical {
        let Some(prototype) = tcl_compiler::registry_invocation::source_structure::original_logical_procedure_arguments(ctx.full_source, analysis, seg) else {
            return Vec::new();
        };
        if !prototype.matches_source(
            &tcl_lexer::SourceImage::document(ctx.full_source),
            ctx.config,
        ) || !prototype.matches_registry(ctx.registry)
            || analysis.resolved_input.as_ref() != Some(prototype.resolved_input())
        {
            return Vec::new();
        }
        let metadata = prototype.declaration().metadata();
        return insert_original_procedure_argument_role_geometry(
            seg,
            prototype.bindings(),
            prototype.arguments(),
            prototype.operands(),
            [
                &proc_var_write_indices(metadata),
                &proc_var_read_indices(metadata),
                &proc_command_indices(metadata),
            ],
            overrides,
        );
    }
    let Some(prototype) =
        tcl_compiler::registry_invocation::source_structure::original_procedure_arguments(
            ctx.full_source,
            analysis,
            seg,
        )
    else {
        return Vec::new();
    };
    if !prototype.matches_source(
        &tcl_lexer::SourceImage::document(ctx.full_source),
        ctx.config,
    ) || !prototype.matches_registry(ctx.registry)
    {
        return Vec::new();
    }
    let declaration = prototype.declaration();
    let roles = ctx
        .proc_roles
        .and_then(|index| {
            index.original_roles_at(analysis, ctx.full_source, seg.argv.first()?.span.start())
        })
        .filter(|roles| {
            roles.site.as_ref() == Some(declaration.declaration_site())
                && roles.name == *declaration.name()
        })
        .unwrap_or_else(|| {
            OriginalProcArgRoles::new(
                declaration.name(),
                Some(declaration.declaration_site()),
                declaration.metadata(),
            )
        });
    insert_original_procedure_argument_role_geometry(
        seg,
        prototype.bindings(),
        prototype.arguments(),
        prototype.operands(),
        [&roles.write, &roles.read, &roles.command],
        overrides,
    )
}

fn insert_original_procedure_argument_role_geometry(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    bindings: &[tcl_syntax::formal_params::FormalByteArgumentBinding],
    arguments: &[tcl_compiler::registry_invocation::EffectiveInvocationWord],
    operands: &[Option<
        tcl_compiler::registry_invocation::source_structure::OriginalOperandSource,
    >],
    roles: [&[u32]; 3],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) -> Vec<(Span, TokenKind, u32)> {
    use tcl_syntax::formal_params::FormalByteArgumentBinding;
    let mut extents = Vec::new();
    for (parameters, kind, modifier, override_kind) in [
        (
            roles[0],
            TokenKind::Variable,
            MOD_DECLARATION,
            ArgOverride::VarDecl,
        ),
        (roles[1], TokenKind::Variable, 0, ArgOverride::VarRef),
        (roles[2], TokenKind::Function, 0, ArgOverride::CommandRef),
    ] {
        for binding in bindings {
            let (parameter, argument) = match binding {
                FormalByteArgumentBinding::Value {
                    parameter,
                    argument,
                }
                | FormalByteArgumentBinding::CallerLink {
                    parameter,
                    argument,
                    ..
                } => (*parameter, *argument),
                FormalByteArgumentBinding::Default { .. }
                | FormalByteArgumentBinding::Rest { .. } => continue,
            };
            if !u32::try_from(parameter).is_ok_and(|index| parameters.contains(&index)) {
                continue;
            }
            let Some(operand) = operands.get(argument).and_then(Option::as_ref) else {
                continue;
            };
            if let Some(word) = operand.word() {
                if !seg.argv.iter().any(|token| token.span == operand.span()) {
                    continue;
                }
                if matches!(override_kind, ArgOverride::VarDecl | ArgOverride::VarRef) {
                    retag_original_variable_argument(seg, word, override_kind.clone(), overrides);
                } else if tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some()
                    && word
                        .tokens()
                        .iter()
                        .all(|token| seg.all_tokens.contains(token))
                {
                    for token in word
                        .tokens()
                        .iter()
                        .filter(|token| matches!(token.kind, TokenType::Esc | TokenType::Str))
                    {
                        overrides
                            .entry(token.span.start())
                            .or_insert_with(|| override_kind.clone());
                    }
                }
            } else if let Some(input) = operand.input()
                && arguments
                    .get(argument)
                    .and_then(|value| value.literal_bytes())
                    == Some(input.bytes())
            {
                extents.push((operand.span(), kind, modifier));
            }
        }
    }
    extents
}

/// Project the shared original produced prefix onto its own written operands.
/// Captured values have no current anchor; the builder head stays its own call.
fn merge_list_quoted_command_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    ctx: ScriptCtx<'_>,
    produced: &crate::original_invocation::OriginalRegistryWords,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let overlay = original::registry_overrides(ctx, seg, produced);
    for (span_start, override_kind) in overlay {
        overrides.entry(span_start).or_insert(override_kind);
    }
    if let Some(head) = produced.head_source().and_then(|head| head.word())
        && let Some(token) = head.tokens().first()
    {
        overrides
            .entry(token.span.start())
            .or_insert(ArgOverride::CommandRef);
    }
}

/// This command's own argument slots whose registry role means "the value
/// here is later invoked/sourced as a command" — `Body` / `LambdaLiteral` /
/// `CommandPrefix` — as the set of their representative tokens' start
/// offsets. A `[…]` substitution occupying one of these slots recurses with
/// `deferred_role = true` (see [`collect_script`]) so list-quoted-lambda
/// source projection only fires for a
/// genuinely deferred invocation (`package ifneeded … [list apply {…}
/// $dir]`), never for inert data (`set x [list apply {…} value]`).
fn deferred_role_arg_starts(
    words: Option<&crate::original_invocation::OriginalRegistryWords>,
) -> FxHashSet<u32> {
    words
        .into_iter()
        .flat_map(|words| {
            words
                .roles
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .filter(|(_, role)| {
                    matches!(
                        role,
                        tcl_registry::ArgRole::Body
                            | tcl_registry::ArgRole::CommandPrefix
                            | tcl_registry::ArgRole::LambdaLiteral
                    )
                })
                .filter_map(|(ordinal, _)| {
                    words
                        .operands
                        .get(*ordinal)?
                        .as_ref()
                        .map(|operand| operand.span.start())
                })
        })
        .collect()
}

/// Emit semantic-token entries for a single non-head argument token,
/// dispatching on its [`ArgOverride`] (or falling back to default
/// classification) and recursing into braced bodies / expressions /
/// command substitutions.  Extracted from [`collect_script`] to keep that
/// function's body small.
/// When `cond` holds, classify `tok` with [`classify_arg_token`] and, if it
/// yields a kind, push a plain token.  Used as the fallback for the
/// sub-tokenising format overrides (sprintf / clock / binary / regsub) when
/// the specialised sub-lexer declined to emit anything.
fn classify_and_push_if(cond: bool, ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    if cond && let Some(kind) = classify_arg_token(tok, ctx.full_source, ctx.numbers) {
        push_token(ctx.line_index, ctx.full_source, tok, kind, 0, entries);
    }
}

/// The fixed `(kind, modifier)` for overrides that emit their token verbatim,
/// or `None` for overrides that need custom handling (recursion / sub-tokens).
fn verbatim_token_kind(ov: &ArgOverride) -> Option<(TokenKind, u32)> {
    match ov {
        ArgOverride::Kind(kind) => Some((*kind, 0)),
        ArgOverride::Decorator => Some((TokenKind::Decorator, 0)),
        ArgOverride::VarDecl => Some((TokenKind::Variable, MOD_DECLARATION)),
        ArgOverride::VarRef => Some((TokenKind::Variable, 0)),
        ArgOverride::CommandRef => Some((TokenKind::Function, 0)),
        ArgOverride::SubcommandKeyword => Some((TokenKind::Keyword, MOD_DEFAULT_LIBRARY)),
        ArgOverride::ProcNameDef => Some((TokenKind::Function, MOD_DEFINITION)),
        ArgOverride::ClassNameDef => Some((TokenKind::Class, MOD_DEFINITION)),
        ArgOverride::ClassNameRef => Some((TokenKind::Class, 0)),
        _ => None,
    }
}

fn emit_arg_token(
    ctx: ScriptCtx<'_>,
    body_ctx: ScriptCtx<'_>,
    tok: Token,
    override_kind: Option<&ArgOverride>,
    deferred_role: bool,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let tok = &tok;
    // Substitutions retain the genuine source vocabulary parent; each region
    // issuer checks its own complete argv.
    let plain_ctx = ScriptCtx {
        oo_grammar: None,
        original_definition_members: None,
        declared_words: None,
        ..ctx
    };
    // Overrides that emit their token verbatim collapse to one path.
    if let Some((kind, modifier)) = override_kind.and_then(verbatim_token_kind) {
        push_token(line_index, full_source, *tok, kind, modifier, entries);
        return;
    }
    match override_kind {
        Some(ArgOverride::RegexPattern) => {
            if !push_regex_subtokens(line_index, full_source, *tok, entries) {
                push_token(line_index, full_source, *tok, TokenKind::Regexp, 0, entries);
            }
        }
        Some(ArgOverride::SprintfFormat) => {
            let emitted =
                push_sprintf_subtokens(line_index, full_source, *tok, ctx.dialect, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::ClockFormat) => {
            let emitted = push_clock_subtokens(line_index, full_source, *tok, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::BinaryFormat) => {
            let emitted =
                push_binary_subtokens(line_index, full_source, *tok, ctx.dialect, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::RegsubReplace) => {
            let emitted = push_regsub_subtokens(line_index, full_source, *tok, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::LambdaLiteral) => {
            collect_lambda_literal(ctx, *tok, entries, depth + 1);
        }
        Some(ArgOverride::LoopVarList) => {
            collect_loop_var_list(ctx, *tok, entries);
        }
        Some(ArgOverride::ParamList) => {
            collect_param_list(ctx, *tok, entries);
        }
        Some(ArgOverride::MemberName) => {
            push_token(
                line_index,
                full_source,
                *tok,
                TokenKind::Method,
                MOD_DEFINITION,
                entries,
            );
        }
        Some(ArgOverride::BodyScript) => {
            if let Some((cstart, inner)) = subspec_content(full_source, *tok) {
                // Recurse with the OO-body context computed for this
                // command's bodies (`body_ctx`) so a method / constructor /
                // property-accessor body inside a class definition is walked
                // as ordinary code, while the class body itself stays in OO
                // context.
                collect_script(
                    body_ctx,
                    inner,
                    u32::try_from(cstart).unwrap_or(0),
                    entries,
                    depth + 1,
                    false,
                );
            } else if let Some(kind) = classify_arg_token(*tok, full_source, ctx.numbers) {
                push_token(line_index, full_source, *tok, kind, 0, entries);
            }
        }
        Some(ArgOverride::ExprScript) => {
            collect_expr(plain_ctx, *tok, entries, depth + 1);
        }
        Some(ArgOverride::CaseList(spec, regexp)) => {
            // The written-word check reads this argument's own operand; the
            // clause bodies keep the command's body context.
            let list_ctx = ScriptCtx {
                operand: ctx.operand,
                ..body_ctx
            };
            // A command with no retained invocation reads its written list.
            let written = ctx.original_words.is_none() && ctx.declared_words.is_none();
            collect_case_list(list_ctx, *tok, entries, depth + 1, spec, *regexp, written);
        }
        Some(ArgOverride::KeywordArg) => {
            push_keyword_arg(ctx, *tok, entries);
        }
        // Verbatim-token overrides are handled by the early return above.
        Some(
            ArgOverride::Kind(_)
            | ArgOverride::Decorator
            | ArgOverride::VarDecl
            | ArgOverride::VarRef
            | ArgOverride::CommandRef
            | ArgOverride::SubcommandKeyword
            | ArgOverride::ProcNameDef
            | ArgOverride::ClassNameDef
            | ArgOverride::ClassNameRef,
        ) => {}
        None => emit_default_arg_token(plain_ctx, *tok, entries, depth, deferred_role),
    }
}

/// Handle an argument token with no [`ArgOverride`]: recurse into a `[…]`
/// command substitution, or classify a plain word (splitting backslash
/// escapes out of string literals).  Extracted from [`emit_arg_token`].
///
/// `deferred_role` is `tok`'s own [`collect_script`]-computed deferred-role
/// flag (see there) — threaded through unchanged so the `[…]` recursion below
/// carries it into the substitution's content.
fn emit_default_arg_token(
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
    depth: u32,
    deferred_role: bool,
) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    if matches!(tok.kind, TokenType::Cmd) {
        // Command substitution `[…]` — recurse into the inner
        // script (delimiters stripped via `content_offset`).
        let cstart = tok.span.start() as usize + tok.content_offset as usize;
        let cend = (tok.span.end() as usize).min(full_source.len());
        if cend > cstart
            && let Some(inner) = full_source.get(cstart..cend)
        {
            collect_script(
                ctx,
                inner,
                u32::try_from(cstart).unwrap_or(0),
                entries,
                depth + 1,
                deferred_role,
            );
        }
    } else if let Some(kind) = classify_arg_token(tok, full_source, ctx.numbers) {
        // String / bareword args with backslash escapes split
        // into literal `String` runs + `Escape` sub-tokens.
        if kind == TokenKind::String
            && push_escape_subtokens(line_index, full_source, tok, ctx.config, entries)
        {
            // emitted as sub-tokens
        } else {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
    }
}

/// Tokenise a braced expression argument via the expression sub-lexer,
/// emitting variable / number / operator / function / string / boolean
/// sub-tokens (math functions carry `defaultLibrary`) and recursing into
/// nested `[cmd]` substitutions.
fn collect_expr(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>, depth: u32) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        if let Some(kind) = classify_arg_token(tok, full_source, ctx.numbers) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let (tokens, _) = tcl_lexer::tokenise_expr_checked_with_expression_grammar(
        inner,
        &ctx.config.grammar_over(ctx.dialect.grammar),
        ctx.dialect.expr_grammar_base,
        ctx.dialect.f5_core_expr_grammar(),
    );
    for et in tokens {
        use tcl_lexer::ExprTokenType as E;
        let abs_start = cstart + et.start as usize;
        match et.kind {
            E::Command => {
                let original = tcl_lexer::ExprToken {
                    kind: et.kind,
                    text: et.text.as_bytes().to_vec(),
                    start: et.start,
                    end: et.end,
                };
                let Some(term) =
                    tcl_lexer::expression_term(inner.as_bytes(), &original, ctx.config)
                else {
                    continue;
                };
                let Some(body) = inner.get(term.value.start as usize..term.value.end as usize)
                else {
                    continue;
                };
                let Some(offset) = u32::try_from(cstart)
                    .ok()
                    .and_then(|start| start.checked_add(term.value.start))
                else {
                    continue;
                };
                collect_script(ctx, body, offset, entries, depth + 1, false);
            }
            E::Function if !et.text.is_empty() && !et.text.contains('\n') => {
                let pos = line_index
                    .position_at_utf16(u32::try_from(abs_start).unwrap_or(0), full_source);
                let mods = if ctx.lexical {
                    u32::from(
                        ctx.context.source_math_function(&et.text)
                            == tcl_dialect::model::CapabilityAnswer::Yes,
                    ) * MOD_DEFAULT_LIBRARY
                } else {
                    ctx.analysis
                        .and_then(|analysis| {
                            crate::math_function_symbol::at(
                                full_source,
                                analysis,
                                u32::try_from(abs_start).ok()?,
                            )
                        })
                        .and_then(|function| {
                            function
                                .selected_registry_spec(ctx.registry)
                                .map(|_| MOD_DEFAULT_LIBRARY)
                        })
                        .unwrap_or(0)
                };
                entries.push((
                    pos.line,
                    pos.character.get(),
                    utf16_len(&et.text),
                    TokenKind::Function,
                    mods,
                ));
            }
            kind => {
                if let Some(token_kind) = expr_subtoken_kind(kind) {
                    push_subtoken(
                        full_source,
                        line_index,
                        abs_start,
                        &et.text,
                        token_kind,
                        entries,
                    );
                }
            }
        }
    }
}

/// The semantic-token kind an expression sub-token highlights as, for the kinds
/// that map to one directly.
///
/// `None` for the two [`collect_expr`] handles itself — `Command` recurses into
/// the nested script and `Function` needs the `defaultLibrary` modifier — and
/// for the kinds that carry no highlighting (whitespace, the never-emitted
/// `Eof`). Exhaustive on purpose: a new `ExprTokenType` variant fails to compile
/// here rather than silently going unstyled.
fn expr_subtoken_kind(kind: tcl_lexer::ExprTokenType) -> Option<TokenKind> {
    use tcl_lexer::ExprTokenType as E;
    Some(match kind {
        E::Number => TokenKind::Number,
        E::Variable => TokenKind::Variable,
        E::String => TokenKind::String,
        E::Bool => TokenKind::Keyword,
        // The grouping / ternary / argument-separator punctuation is as much an
        // operator as `+` or `&&`, and the expr lexer already tells them apart —
        // dropping them into a catch-all arm would leave
        // `expr {($a + $b) * $c}`'s parens unstyled and
        // `$a > 1 ? "y" : "n"`'s `?` and `:` unstyled.
        E::Operator | E::ParenOpen | E::ParenClose | E::Comma | E::TernaryQ | E::TernaryC => {
            TokenKind::Operator
        }
        // A TIP 582 `# …` comment inside the expression body highlights as a
        // comment, like a script-level one. Always single-line: the lexer stops
        // the token before the terminating newline, so this satisfies
        // `push_subtoken`'s one-line requirement by construction.
        E::Comment => TokenKind::Comment,
        E::Command | E::Function | E::Whitespace | E::Eof => return None,
    })
}

/// Walk the segmenter + comment scan and return raw
/// [`Entry`] tuples sorted by position.  Shared by `full` and `range`.
/// Augment the object-handle map with loop variables that iterate an object
/// collection — `dict for {k v} $coll {…}`, `dict map …`, `foreach v $coll {…}`,
/// `lmap …` — so a `$v method …` dispatch in the loop body resolves like a
/// `[dict get $coll $k] method …` retrieval.
///
/// The shared original executable-source walker supplies potential regions;
/// the actual per-point schema owns roles and captured operands. Inert braced
/// data supplies no loop binding. No-op when no object collection is tracked.
fn augment_loop_var_handles(
    source: &str,
    analysis: &AnalysisResult,
    object_collections: &ObjectClassMap,
    object_classes: &mut ObjectClassMap,
) {
    if object_collections.is_empty() {
        return;
    }
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return;
    };
    let context = current.context();
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(
        &current.config().grammar_over(current.profile().grammar),
    );
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if let Some(words) =
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, command,
                )
            {
                bind_loop_vars_for_call(
                    (source, current.config()),
                    command,
                    &words,
                    &context,
                    rules,
                    object_collections,
                    object_classes,
                );
            }
            false
        },
    );
}

/// Add a loop variable → element-class set entry to the handle map (skips an
/// empty name).
fn bind_loop_var(
    handles: &mut ObjectClassMap,
    var: &str,
    classes: &std::collections::HashSet<String>,
) {
    if !var.is_empty() {
        handles
            .entry(var.to_owned())
            .or_default()
            .extend(classes.iter().cloned());
    }
}

/// Syntactic scan for object-handle bindings that the compiler CFG does not
/// surface, binding the handle variable to its class so a `$NAME method …`
/// dispatch in a snit method body resolves.  snit method bodies are **not**
/// lowered into the compiler CFG (only token-walked, like the `$self` path), so
/// a source scan is how these classes reach the handle map — the same technique
/// the loop-var scan uses.
///
/// Which calls bind a handle, and at which argument indices, is registry data
/// ([`tcl_registry::HandleBindingSpec`]) — the walker names no command, so
/// `::set` and a provable alias of it bind exactly like `set`.  Two layouts
/// exist:
///
/// - `install NAME using TYPE …`, snit's component installer, declared on the
///   snit definition-body grammar's
///   [`member_body_commands`](tcl_registry::definer::DefinitionBodyGrammar::member_body_commands)
///   because the word exists only inside a snit member body; and
/// - `set NAME [TYPE inst …]`, whose value word may be a *bare-word*
///   construction (`$type $name` creates an instance) — gated on `TYPE` being a
///   visible class of a family whose grammar declares
///   [`bare_word_construction`](tcl_registry::definer::DefinitionBodyGrammar::bare_word_construction),
///   and whose first argument is **not** a typemethod (`info` / `destroy` / a
///   declared `typemethod`), which would be a type-command call rather than a
///   construction.
///
/// Highlight-only and sound by abstention: the bare-constructor form only fires
/// when `TYPE` is visible in the hierarchy (local, or workspace-merged in
/// project mode), so an unknown type is never guessed at.
fn augment_snit_handles(
    source: &str,
    analysis: &AnalysisResult,
    classes: Option<&ClassHierarchy>,
    object_classes: &mut ObjectClassMap,
) {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return;
    };
    let context = current.context();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            bind_original_handle_construction(
                source,
                analysis,
                command,
                &context,
                classes,
                object_classes,
            );
            false
        },
    );
    let Some(walk) = crate::refactor::FrameWalk::new(source, analysis) else {
        return;
    };
    for (region, grammar) in selected_member_handle_regions(source, analysis, &context) {
        let Some(text) = source.get(region.as_range()) else {
            continue;
        };
        for root in walk.segment_nested(text, region.start()) {
            let mut commands = Vec::new();
            walk.nested_same_frame_commands(source, &root, &mut commands);
            commands.push(root);
            for command in commands {
                let Some(original) = walk.native_words(source, &command) else {
                    continue;
                };
                let values = original
                    .iter()
                    .map(tcl_syntax::word_rules::original_static_word_ascii_presentation)
                    .map(|value| value.and_then(|bytes| String::from_utf8(bytes).ok()))
                    .collect::<Vec<_>>();
                let Some(head) = values.first().and_then(Option::as_deref) else {
                    continue;
                };
                let arguments = original
                    .iter()
                    .zip(&values)
                    .skip(1)
                    .map(|(word, value)| {
                        if word.group().expand {
                            tcl_registry::InvocationWord::Expanded
                        } else if let Some(value) = value.as_deref() {
                            tcl_registry::InvocationWord::Literal(value)
                        } else {
                            tcl_registry::InvocationWord::Dynamic
                        }
                    })
                    .collect::<Vec<_>>();
                if let Some(binding) = grammar
                    .member_body_command(head)
                    .and_then(|command| command.binds_handle)
                    && let Some(bound) = binding
                        .resolve_words(tcl_registry::InvocationArguments::structured(&arguments))
                    && matches!(bound.class_source, tcl_registry::HandleClassSource::Word(_))
                {
                    bind_object_handle(&bound, classes, object_classes);
                }
            }
        }
    }
}

/// The class a declaration names in this analysis: its canonical record
/// under a pinned release, its retained logical record otherwise.
fn declared_source_class<'a>(
    declaration: &tcl_compiler::command_binding::OriginalSourceClassDeclaration,
    analysis: &'a AnalysisResult,
) -> Option<&'a tcl_compiler::analyser::ClassDef> {
    declaration
        .source_class(analysis)
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::metadata)
        .or_else(|| declaration.logical_source_class(analysis))
}

/// The actual selected factory and sealed member-script vectors own the
/// installer vocabulary. This is conditional source metadata only.
fn selected_member_handle_regions(
    source: &str,
    analysis: &AnalysisResult,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<(Span, &'static tcl_registry::definer::DefinitionBodyGrammar)> {
    use tcl_compiler::registry_invocation::source_structure;
    let mut pending = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            let declaration = source_structure::source_class_declaration_at(
                source,
                analysis,
                command.span.start(),
            )
            .or_else(|| {
                source_structure::source_configured_class_at(source, analysis, command.span.start())
                    .map(|target| target.class_declaration().clone())
            });
            if let Some(declaration) = declaration
                && declared_source_class(&declaration, analysis).is_some()
                && let Some(grammar) = declaration.grammar(context)
                && !grammar.member_body_commands.is_empty()
                && let Some(words) =
                    source_structure::source_registry_words(source, analysis, command)
            {
                for body in words.source_script_bodies(context) {
                    if let Some(parent) = body.definition_parent_for(context, None) {
                        pending.push((parent, body.content_span(), grammar, 0));
                    }
                }
            }
            false
        },
    );
    let mut regions = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    while let Some((parent, region, grammar, depth)) = pending.pop() {
        if MAX_TOKEN_RECURSION.exceeded(depth) || !visited.insert((region.start(), region.end())) {
            continue;
        }
        let Some(members) = parent.definition_member_region(context, region) else {
            continue;
        };
        for original in members.original_commands() {
            for body in members.script_bodies(original).into_iter().flatten() {
                if let Some(parent) = body.definition_parent() {
                    pending.push((parent.clone(), body.content_span(), grammar, depth + 1));
                } else {
                    regions.push((body.content_span(), grammar));
                }
            }
        }
    }
    regions
}

/// A bare-word construction the analysis kept no call for because the
/// instance name is computed (`set eng [Engine ${selfns}::e]`). The written
/// type word must name a class of the hierarchy whose definer's grammar
/// constructs on an unknown first word; a literal first word is left to the
/// analysis, which knows whether it is a type method.
fn bind_written_bare_construction(
    setter: &tcl_compiler::registry_invocation::OriginalSourceHandleConstruction,
    context: &tcl_registry::model::ContextRegistry,
    classes: Option<&ClassHierarchy>,
    handles: &mut ObjectClassMap,
) {
    let words = &setter.construction().words;
    let (Some(hierarchy), [head, name, ..]) = (classes, words.as_slice()) else {
        return;
    };
    if tcl_syntax::word_rules::original_static_word_ascii_presentation(name).is_some() {
        return;
    }
    let Some(class) = tcl_syntax::word_rules::original_static_word_ascii_presentation(head)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|written| resolve_class_in_hierarchy(hierarchy, &written))
        .and_then(|qualified| hierarchy.classes.get(&qualified))
    else {
        return;
    };
    let bare_word = class.class_command_fallback.constructs_named_instance()
        || context
            .commands()
            .get(&class.metaclass)
            .and_then(|definer| definer.definition_body)
            .is_some_and(|grammar| grammar.bare_word_construction);
    if !bare_word {
        return;
    }
    if let Ok(variable) = std::str::from_utf8(setter.variable_bytes())
        && !variable.is_empty()
    {
        handles
            .entry(variable.to_owned())
            .or_default()
            .insert(class.qualified_name.clone());
    }
}

/// A selected setter and current constructor graph retain independent source
/// producers; no displayed bracket text supplies a receiver or stored value.
fn bind_original_handle_construction(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
    classes: Option<&ClassHierarchy>,
    handles: &mut ObjectClassMap,
) {
    use tcl_compiler::registry_invocation::source_structure;
    let Some(setter) =
        source_structure::source_handle_construction_at(source, analysis, command.span.start())
    else {
        return;
    };
    let Some(head) = setter.construction().words.first() else {
        return;
    };
    let Some(call) =
        source_structure::source_constructor_call_at(source, analysis, head.span().start())
    else {
        bind_written_bare_construction(&setter, context, classes, handles);
        return;
    };
    let Some(class) = declared_source_class(call.class_declaration(), analysis) else {
        return;
    };
    let Some(grammar) = call.class_declaration().grammar(context) else {
        return;
    };
    if !grammar.bare_word_construction
        && !class
            .factory
            .as_ref()
            .is_some_and(|factory| factory.unknown_binds_instance)
    {
        return;
    }
    let arguments = call.arguments();
    let Some(selector) = arguments
        .first()
        .and_then(|word| word.literal_bytes())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return;
    };
    if grammar.manufacturer(selector).is_none()
        && (grammar.is_builtin_type_method(selector) || class.class_methods.contains_key(selector))
    {
        return;
    }
    let Ok(name) = std::str::from_utf8(setter.variable_bytes()) else {
        return;
    };
    if !name.is_empty() {
        handles
            .entry(name.to_owned())
            .or_default()
            .insert(class.qualified_name.clone());
    }
}

/// Bind one resolved [`BoundHandle`] into the handle map.
///
/// The two class sources are resolved differently and both abstain rather than
/// guess: a [`HandleClassSource::Word`] is literal type-name data selected
/// from the original grouped word, while a
/// [`HandleClassSource::ConstructionValue`] must parse as a command
/// substitution whose head is a visible class of a family that constructs by
/// bare word.
///
/// [`BoundHandle`]: tcl_registry::BoundHandle
/// [`HandleClassSource::Word`]: tcl_registry::HandleClassSource::Word
/// [`HandleClassSource::ConstructionValue`]: tcl_registry::HandleClassSource::ConstructionValue
fn bind_object_handle(
    bound: &tcl_registry::BoundHandle<'_>,
    classes: Option<&ClassHierarchy>,
    handles: &mut ObjectClassMap,
) {
    if bound.name.is_empty() {
        return;
    }
    match bound.class_source {
        tcl_registry::HandleClassSource::Word(_) => {
            let type_name = bound.class_word;
            if type_name.is_empty() {
                return;
            }
            let Some(hierarchy) = classes else { return };
            let Some(qualified) = resolve_class_in_hierarchy(hierarchy, type_name) else {
                return;
            };
            handles
                .entry(bound.name.to_owned())
                .or_default()
                .insert(qualified);
        }
        tcl_registry::HandleClassSource::ConstructionValue(_) => {}
    }
}

/// Bind every loop variable of one call that iterates a known object
/// collection.
///
/// Registry-driven, with no command spelling anywhere: the
/// [`ArgRole::LoopVarList`] indices come from the registry
/// (`foreach`/`lmap`'s repeated `(start 0, stride 2)` layout, `dict for` /
/// `dict map` / `array for`'s static role tables), and the iterated
/// collection is always the word after the variable list. `::`-qualified and
/// aliased spellings therefore classify exactly like bare ones.
///
/// The variable-list shape follows the collection's declared argument type:
/// a `Dict` collection binds a `{keyVar valueVar}` pair whose *value*
/// variable holds the element, and anything else binds every variable in the
/// group.
fn bind_loop_vars_for_call(
    written_source: (&str, tcl_lexer::LexerConfig),
    command: &tcl_compiler::segmenter::SegmentedCommand,
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    rules: tcl_syntax::word_rules::WordValueRules,
    collections: &ObjectClassMap,
    handles: &mut ObjectClassMap,
) {
    let (source, config) = written_source;
    if command.is_partial {
        return;
    }
    let Some(argument_roles) = words.roles() else {
        return;
    };
    let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        command,
    );
    for &(ordinal, role) in argument_roles {
        if role != tcl_registry::ArgRole::LoopVarList {
            continue;
        }
        let Some(value) = words
            .arguments()
            .get(ordinal)
            .and_then(|word| word.literal_bytes())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            continue;
        };
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            ordinal
                .checked_add(2)
                .and_then(|index| words.origins().get(index))
        else {
            continue;
        };
        if command.single_token_word.get(*written) != Some(&true) {
            continue;
        }
        let Some(classes) = original_words
            .word_exprs
            .get(*written)
            .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
            .and_then(|(spelling, _)| object_handle_name(spelling, Some(config)))
            .and_then(|name| collections.get(name))
        else {
            continue;
        };
        let Ok(names) = rules.split_list(value) else {
            continue;
        };
        let keyed = words.with_source_schema(context, |schema| {
            schema
                .facts()
                .argument_type_hint(ordinal + 1)
                .is_some_and(|hint| hint.expected == Some(tcl_registry::types::TclType::Dict))
        }) == Some(true);
        if keyed {
            if let Some(name) = names.get(1) {
                bind_loop_var(handles, name, classes);
            }
        } else {
            for name in &names {
                bind_loop_var(handles, name, classes);
            }
        }
    }
}

/// Decode a literal loop-variable-list word using Tcl's canonical list
/// grammar. Substitutions, expansions, compound words, incomplete commands,
/// and malformed list values all abstain.
#[cfg(test)]
fn static_loop_var_names(
    source: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    word_index: usize,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<Vec<String>> {
    if seg.is_partial || seg.single_token_word.get(word_index) != Some(&true) {
        return None;
    }
    let token = *seg.argv.get(word_index)?;
    if matches!(
        token.kind,
        TokenType::Var | TokenType::Cmd | TokenType::Expand
    ) {
        return None;
    }
    // The segmenter strips a braced/quoted word's delimiters. Validate the
    // original token too, so incomplete recovery text cannot masquerade as a
    // valid list value.
    let raw_start = token.span.start() as usize;
    let raw_end = token.span.end() as usize;
    let raw = source.get(raw_start..raw_end)?;
    if raw.starts_with(['{', '"']) {
        let closing = if raw.starts_with('{') { b'}' } else { b'"' };
        let raw = if source.as_bytes().get(raw_end) == Some(&closing) {
            source.get(raw_start..raw_end + 1)?
        } else {
            raw
        };
        tcl_syntax::list::find_element(raw, 0).ok()??;
    }
    let text = seg.texts.get(word_index)?;
    rules.split_list(text).ok().map(|elements| {
        elements
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect()
    })
}

fn collect_entries(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> Vec<Entry> {
    let WorkspaceTokenFacts {
        classes,
        proc_roles,
        named_instances: _,
        project_classes,
        analysis,
    } = facts;
    // Whether the caller supplied the document's own analysis, or this is the
    // explicit standalone source API (the fresh analysis below).
    let document_analysis = analysis.is_some();
    let fresh_analysis;
    let analysis = if let Some(analysis) = analysis {
        analysis
    } else {
        fresh_analysis = standalone_token_analysis(source, dialect, registry);
        &fresh_analysis
    };
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    let config = current.config();
    let registry = current.registry();
    let dialect = current.profile();
    let generation = current.context();
    // A source shell interpreted nothing, so every command reads its written
    // words; any analysis query would interpret the document after all.
    let retained = analysis;
    let shell = retained.is_source_shell().then_some(retained);
    let lexical = shell.is_none() && retained.allows_lexical_declaration_advice();
    let analysis = shell.is_none().then_some(retained);
    let mut entries: Vec<Entry> = Vec::new();
    let line_index = LineIndex::new(source);
    let profile = dialect;

    // Regex-source spans: the def-site literal words (`set my_re ".*"`) whose
    // variable flows into a `regexp`/`regsub` pattern, keyed by word start so
    // the walk can retag the matching argument.  Empty (no map lookups) when
    // there is no analysis or no such flow. The compiler decides the spans
    // under the unit's own profile, so every profile reads them.
    let regex_sources: FxHashMap<u32, Span> = cu.map_or_else(FxHashMap::default, |cu| {
        tcl_compiler::regex_source::regex_source_literal_spans(source, cu, registry, dialect)
            .into_iter()
            .map(|span| (span.start(), span))
            .collect()
    });

    // The document's command-identity facts: bare-name aliases for commands
    // imported from an exported namespace (`namespace import tcltest::*` →
    // `test` = `tcltest::test`), plus every statically proven `interp alias` /
    // `rename` / built-in-shadowing `proc`.  Empty (no lookups)
    // unless the document actually binds something.
    let Some(head_identities) = retained.retained_command_realm() else {
        return Vec::new();
    };

    // The iRules declaration overlay uses the shared top-level boundary facts
    // (including offset-resolved command identity) rather than this walk's
    // recursive depth as its placement predicate.
    let irules_top_level_declaration_heads = if lexical || dialect.is_irules() {
        irules_top_level_declaration_heads(source, registry, config, head_identities)
    } else {
        FxHashSet::default()
    };

    let (object_classes, _object_collections) =
        token_object_maps(source, registry, cu, retained, lexical, facts);

    // Walk every segmented command (recursing into braced bodies, braced
    // expressions, and `[…]` command substitutions) and classify each token.
    let original_collections = cu
        .filter(|_| lexical || project_classes.is_some())
        .and_then(|unit| {
            tcl_compiler::object_types::OriginalObjectCollectionSources::capture(
                source, analysis?, unit,
            )
        });
    let original_roles = original::OriginalTokenRoles::capture(source, analysis);
    let walk = WalkFacts::new(shell, analysis);
    let ctx = ScriptCtx {
        full_source: source,
        dialect,
        config,
        context: generation.context(),
        generation: &generation,
        operand: None,
        original_words: None,
        declared_words: None,
        written_members: false,
        walk: &walk,
        lexical,
        numbers: profile.grammar.numbers,
        registry,
        line_index: &line_index,
        // The root of a document is itself a definition body when the dialect
        // says so: a `.sslictcl` document's legal top-level words are exactly
        // its document grammar's members, painted from membership like every
        // level below. An ordinary Tcl document has no such grammar.
        oo_grammar: registry.document_grammar(),
        receiver_grammar: None,
        original_definition_parent: None,
        original_definition_members: None,
        scoped_env: None,
        regex_sources: &regex_sources,
        object_classes: &object_classes,
        classes,
        project_classes,
        analysis,
        original_collections: original_collections.as_ref(),
        original_roles: &original_roles,
        proc_roles,
        enclosing_class: None,
        irules_top_level_declaration_heads: &irules_top_level_declaration_heads,
    };
    collect_context_entries(ctx, cu, document_analysis, &mut entries);
    entries
}

fn standalone_token_analysis(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> AnalysisResult {
    standalone_analyser(dialect, registry)
        .structure_only()
        .analyse(source, dialect.name)
}

fn standalone_analyser(
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> tcl_compiler::analyser::Analyser {
    let context = tcl_registry::model::context_for_profile(dialect);
    let context =
        std::sync::Arc::new(context.with_command_store(registry.snapshot().shared_registry()));
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        dialect,
        dialect,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
    );
    tcl_compiler::analyser::Analyser::new().with_resolved_input(input)
}

fn token_object_maps(
    source: &str,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    retained: &AnalysisResult,
    lexical: bool,
    facts: WorkspaceTokenFacts<'_>,
) -> (ObjectClassMap, ObjectClassMap) {
    let WorkspaceTokenFacts {
        classes,
        named_instances,
        project_classes,
        ..
    } = facts;
    let analysis = Some(retained);
    // Source candidates retain the same complete analysis/Module inputs as
    // their shared producer. A release profile colours handles from the unit's
    // own map; the map is display data and proves no receiver.
    let mut object_classes: ObjectClassMap = cu
        .and_then(|cu| {
            if lexical {
                tcl_compiler::object_types::logical_source_object_handle_classes(
                    cu, registry, analysis?,
                )
            } else {
                Some(tcl_compiler::object_types::object_handle_classes(
                    cu, registry,
                ))
            }
        })
        .unwrap_or_default();

    // Logical object classification shares this compatibility handle map.
    // Native source roles never recover receiver identity from its labels.
    if project_classes.is_none()
        && let Some(named) = named_instances
    {
        for (name, class) in named.compatibility_rows() {
            object_classes
                .entry(name.clone())
                .or_default()
                .insert(class.clone());
        }
    }

    // Object-*collection* → element-class map (`dict set Pins $k [Pin new]` →
    // `Pins` is a `Dict` of `Pin`), so a `[dict get $Pins $k] method …`
    // retrieval dispatch resolves the element's method.  Empty
    // without a `CompilationUnit`.
    let object_collections: ObjectClassMap = cu.map_or_else(ObjectClassMap::default, |cu| {
        tcl_compiler::object_types::object_collection_classes(cu)
    });

    // A loop that iterates an object collection binds its value variable to an
    // element, so `$v method …` in the body resolves like `[dict get $coll $k]
    // method …`.  A *syntactic* scan of the source catches every loop shape —
    // including `return [dict map {k v} $coll {…}]` / `set x [dict map …]`,
    // where the loop is nested in a command substitution and the IR never
    // surfaces it as a loop — and feeds the value variable(s) into the handle
    // map (the SpiceGenTcl `allNodes` / `actOnParam` shape).
    augment_loop_var_handles(source, retained, &object_collections, &mut object_classes);

    // snit object-handle bindings the compiler CFG doesn't surface — `install
    // NAME using TYPE` components and `set NAME [Type inst]` bare constructors —
    // via a source scan, since snit method bodies (where these live) are not
    // lowered into the CFG `object_handle_classes` reads.
    augment_snit_handles(source, retained, classes, &mut object_classes);

    (object_classes, object_collections)
}

fn collect_context_entries(
    ctx: ScriptCtx<'_>,
    cu: Option<&CompilationUnit>,
    document_analysis: bool,
    entries: &mut Vec<Entry>,
) {
    let source = ctx.full_source;
    let analysis = ctx.analysis;
    let registry = ctx.registry;
    let profile = ctx.dialect;
    let lexical = ctx.lexical;
    let line_index = ctx.line_index;
    collect_script(ctx, source, 0, entries, 0, false);

    // The shared original source walker retains comment phase independently
    // of the semantic colours already emitted for other source roles.
    push_comment_tokens(source, line_index, analysis, entries);
    // A body the analysis did not enter is still coloured as script by the
    // walk above; its comments are read from the same regions.
    push_entered_comment_tokens(ctx, &ctx.walk.entered_scripts.borrow(), entries);

    // BIG-IP object references (iRules dialect): overlay `object` tokens
    // at recognised pool / data-group / virtual / … name positions.
    // Skipped when an entry already covers the position (e.g. a
    // single-line body's enclosing `string` token) so the token stream
    // never carries overlaps.  Multi-line bodies aren't tokenised by the
    // main walk, so refs inside them surface cleanly.
    if lexical && profile.is_irules() {
        let spans = match analysis {
            Some(analysis) if document_analysis => {
                irules_document_object_spans(source, analysis, cu, registry)
            }
            _ => crate::irules_object_refs::object_ref_spans(source, registry),
        };
        for span in spans {
            push_object_token(source, line_index, span, entries);
        }
    }

    // Sort by (line, column) so the delta encoding works.
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
}

/// The BIG-IP object-reference spans of an iRules document, read under the
/// document's own analysis: the source context is retained from that
/// analysis, its input and its unit — the caller's, or the one the analysis'
/// retained source request rebuilds — so no default iRules input is
/// recreated for it. An analysis that yields no current iRules source context
/// yields no spans (the standalone source API is not substituted).
fn irules_document_object_spans(
    source: &str,
    analysis: &AnalysisResult,
    cu: Option<&CompilationUnit>,
    registry: &CommandRegistry,
) -> Vec<Span> {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let rebuilt;
    let module = if let Some(cu) = cu {
        &cu.ir_module
    } else {
        let Some(unit) = analysis.original_optimiser_source_unit(source, registry, None) else {
            return Vec::new();
        };
        rebuilt = unit;
        &rebuilt.ir_module
    };
    let Some(original) =
        tcl_irules::OriginalIrulesSourceContext::from_analysis(source, analysis, module, &context)
    else {
        return Vec::new();
    };
    let mut spans: Vec<Span> =
        tcl_irules::extract_original_irules_source_object_references(source, &original, None)
            .into_iter()
            .map(|reference| reference.range)
            .collect();
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

/// Push a BIG-IP `object` token for `span`, unless an existing entry on
/// the same line already overlaps its column range (keeps the stream
/// overlap-free).
fn push_object_token(
    source: &str,
    line_index: &LineIndex,
    span: tcl_lexer::Span,
    entries: &mut Vec<Entry>,
) {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    if start.line != end.line {
        return;
    }
    let len = end.character.get().saturating_sub(start.character.get());
    if len == 0 {
        return;
    }
    // An object reference is more specific than the generic bareword
    // `string` classification the (now recursive) body walk produces — drop
    // an overlapping `string` entry and emit the object token instead.  A
    // more specific overlapping kind (keyword / function / variable / …)
    // wins and suppresses the object token.
    let mut other_overlap = false;
    entries.retain(|(l, c, ln, kind, _)| {
        let overlaps = *l == start.line
            && *c < start.character.get() + len
            && start.character.get() < *c + *ln;
        if overlaps {
            if *kind == TokenKind::String {
                return false;
            }
            other_overlap = true;
        }
        true
    });
    if !other_overlap {
        entries.push((start.line, start.character.get(), len, TokenKind::Object, 0));
    }
}

/// Sub-tokenise a string / bareword token's backslash escapes (`\n`, `\t`,
/// `\\`, `\x41`, `é`, `\101`, …): literal runs become `String`, each
/// escape becomes `Escape`.
/// Returns `false` (emitting nothing) when the token carries no backslash, so
/// the caller falls back to a single `String` token.  Multi-line tokens are
/// left to the caller.
fn push_escape_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    config: tcl_lexer::LexerConfig,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, text)) = subspec_content(source, tok) else {
        return false;
    };
    if !text.contains('\\') || text.contains('\n') {
        return false;
    }
    // The split is `tcl-lexer`'s, beside the evaluator that defines the escape
    // widths — every highlighter that colours a Tcl string needs the same rule,
    // and three private copies of it had already drifted apart.
    let segments = tcl_lexer::split_backslash_escapes_in(text, config.escapes);
    if !segments.iter().any(|s| s.is_escape) {
        return false;
    }
    for seg in segments {
        let kind = if seg.is_escape {
            TokenKind::Escape
        } else {
            TokenKind::String
        };
        push_subtoken(
            source,
            line_index,
            cstart + seg.start,
            &text[seg.start..seg.end],
            kind,
            entries,
        );
    }
    // `subspec_content` yields the word's *content*, so this path — unlike
    // `push_token` — must emit both delimiters itself: `"with \"esc\" inside"`
    // would otherwise leave its opening and closing `"` unstyled.  Entries are sorted by
    // position before encoding, so appending them out of order here is fine.
    let start = tok.span.start() as usize;
    push_subtoken(
        source,
        line_index,
        start,
        &source[start..cstart],
        TokenKind::String,
        entries,
    );
    let content_end = cstart + text.len();
    if closing_delimiter(source, tok.span.start())
        .is_some_and(|c| source.as_bytes().get(content_end) == Some(&c))
    {
        push_subtoken(
            source,
            line_index,
            content_end,
            &source[content_end..=content_end],
            TokenKind::String,
            entries,
        );
    }
    true
}

/// Classify a non-head token by its lexer-assigned kind.  `numbers` is the
/// document dialect's numeric-literal grammar, which decides whether a bareword
/// reads as a number (`0o17` is a number from 8.5, a bareword before it).
fn classify_arg_token(tok: Token, source: &str, numbers: NumberSyntax) -> Option<TokenKind> {
    let span = tok.span;
    let len = (span.end() - span.start()) as usize;
    if len == 0 {
        return None;
    }
    match tok.kind {
        TokenType::Var => Some(TokenKind::Variable),
        TokenType::Str => Some(TokenKind::String),
        TokenType::Esc => {
            // Quoted strings vs barewords vs numbers.  The
            // lexer sets `tok.in_quote = true` on every Esc /
            // Var / Cmd token emitted from inside `"..."`, so
            // multi-fragment quoted strings (e.g. `"a $b c"`)
            // get every literal fragment classified as String
            // — including the leading fragment whose span may
            // not include the opening `"`.  This matches the
            // lexer contract and avoids the prior byte-peek
            // heuristic that missed inner fragments.
            if tok.in_quote {
                return Some(TokenKind::String);
            }
            let start = span.start() as usize;
            let text = source
                .get(start..(start + len).min(source.len()))
                .unwrap_or("");
            if tcl_syntax::number::is_whole_number(text, numbers) {
                Some(TokenKind::Number)
            } else if tcl_syntax::naming::is_qualified(text.as_bytes()) && tok.content_offset == 0 {
                // Only a *bare* word can be a namespace reference.  A quoted or
                // braced word is a string literal even when its content happens
                // to contain `::` — painting `append cmd "::scan \$field"`'s
                // whole quoted word as a namespace would also lose the `\$`
                // escape inside it.
                Some(TokenKind::Namespace)
            } else {
                // Bareword argument words classify as String, so `puts
                // hello` emits the `hello` string token rather than
                // dropping it.
                Some(TokenKind::String)
            }
        }
        _ => None,
    }
}

/// Emit genuine original comment tokens without overlapping another colour.
fn push_comment_tokens(
    source: &str,
    line_index: &LineIndex,
    analysis: Option<&AnalysisResult>,
    entries: &mut Vec<Entry>,
) {
    let Some(comments) = analysis.and_then(|analysis| {
        tcl_compiler::analyser::utils::script_command_comment_facts_from_analysis(source, analysis)
    }) else {
        return;
    };
    push_comment_facts(source, line_index, comments, entries);
}

/// The lexer's comments in exactly the script regions the walk entered and
/// coloured as script. A source shell has no other source of comments.
fn push_entered_comment_tokens(
    ctx: ScriptCtx<'_>,
    entered: &[(Span, u32)],
    entries: &mut Vec<Entry>,
) {
    let image = tcl_lexer::SourceImage::document(ctx.full_source);
    let Some(mut comments) = tcl_compiler::analyser::utils::original_script_command_comment_facts(
        &image, entered, ctx.config,
    ) else {
        return;
    };
    comments.sort_by_key(|comment| comment.span.start());
    comments.dedup_by_key(|comment| comment.span.start());
    push_comment_facts(ctx.full_source, ctx.line_index, comments, entries);
}

fn push_comment_facts(
    source: &str,
    line_index: &LineIndex,
    comments: Vec<tcl_compiler::analyser::utils::ScriptCommentFact>,
    entries: &mut Vec<Entry>,
) {
    let mut covered: FxHashMap<u32, Vec<(u32, u32)>> = FxHashMap::default();
    for (line, column, length, _, _) in entries.iter() {
        covered.entry(*line).or_default().push((*column, *length));
    }
    for comment in comments {
        let pos = line_index.position_at_utf16(comment.span.start(), source);
        // This is only a rendering conflict check: the retained lexer token
        // already established command-position comment phase.
        let character = pos.character.get();
        let already_covered = covered.get(&pos.line).is_some_and(|spans| {
            spans
                .iter()
                .any(|(column, length)| *column <= character && character < *column + *length)
        });
        if !already_covered {
            push_span_entries(
                source,
                line_index,
                comment.span.start() as usize,
                &comment.text,
                TokenKind::Comment,
                0,
                entries,
            );
        }
    }
}

/// The closing delimiter a word opened at `start` expects, if it is delimited.
///
/// The lexer's span convention (documented on the `switch` case-list rebuild
/// above) is that a delimited word's `span.end()` sits **at** its closing `}` /
/// `"`, not past it — so an emitter that takes `start..end` verbatim covers
/// `opener + content` and silently drops the terminator.  Every delimited-word
/// emit path therefore has to ask for the closer back.
fn closing_delimiter(source: &str, start: u32) -> Option<u8> {
    let bytes = source.as_bytes();
    match bytes.get(start as usize)? {
        b'"' => Some(b'"'),
        b'{' => Some(b'}'),
        // `${name}` — a braced *variable*, whose opener is two bytes.
        b'$' if bytes.get(start as usize + 1) == Some(&b'{') => Some(b'}'),
        _ => None,
    }
}

/// Extend `end` over the word's closing delimiter when the lexer left it
/// uncovered (see [`closing_delimiter`]).
///
/// Deliberately keyed on the *byte at `end`* rather than on the token kind: a
/// span that already covers its terminator (an empty `""`, whose `end` lands
/// past the closing quote) has some other byte there and is left alone, so this
/// is idempotent and cannot double-count.
fn end_over_terminator(source: &str, start: u32, end: u32) -> u32 {
    match closing_delimiter(source, start) {
        Some(closer) if source.as_bytes().get(end as usize) == Some(&closer) => end + 1,
        _ => end,
    }
}

/// Push a single token into the entries list, computing
/// (line, column, length-in-chars, kind).
fn push_token(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    kind: TokenKind,
    modifiers: u32,
    entries: &mut Vec<Entry>,
) {
    let span = tok.span;
    let start = span.start();
    let mut end = span.end();
    // The lexer's empty-content clamp (tcl-lexer `parse_quoted`) extends a
    // quoted `Esc` fragment's span by one byte over the `$` / `[` that
    // introduces the *next* substitution token, so `token_text` stays empty
    // while `span.end` lands on the terminator.  That introducer byte
    // belongs to the following `Var` / `Cmd` token; emitting it here would
    // produce overlapping semantic tokens (e.g. `"$x"` → the opening
    // fragment `"$` overlapping the `$x` variable).  A clamped-empty ESC is
    // recognised by `span_len == content_offset + 1` with a `$` / `[` last
    // byte; trim it back to just its leading delimiter (the opening `"`, or
    // nothing when there is no delimiter, e.g. between adjacent `$a$b`).
    if tok.kind == TokenType::Esc
        && end - start == u32::from(tok.content_offset) + 1
        && let Some(&last) = source.as_bytes().get((end - 1) as usize)
        && (last == b'$' || last == b'[')
    {
        end = start + u32::from(tok.content_offset);
    } else {
        // Cover the word's closing `}` / `"`, which the lexer's span convention
        // leaves just past `span.end()`.  Not applied to the clamped
        // fragment above: that one was trimmed *back* precisely because its span
        // ran into the next token, and re-extending it would overlap.
        end = end_over_terminator(source, start, end);
    }
    if end <= start {
        return;
    }
    let text = source.get(start as usize..end as usize).unwrap_or("");
    // The LSP encoding wants per-line entries, so a multi-line token (a braced
    // or quoted string literal spanning lines) is split into one entry per
    // line rather than dropped — see [`push_span_entries`].
    push_span_entries(
        source,
        line_index,
        start as usize,
        text,
        kind,
        modifiers,
        entries,
    );
}

/// A current original operand owns its content delimiters independently of
/// keyword selection. Missing retained operands keep the token-only syntax view.
fn keyword_operand_content<'a>(
    source: &'a str,
    tok: Token,
    original: Option<&tcl_lexer::NativeWord>,
    config: tcl_lexer::LexerConfig,
) -> Option<(usize, &'a str)> {
    if let Some(word) = original {
        if word.image().bytes() != source.as_bytes()
            || word.config().nested() != config.nested()
            || word.group().expand
            || !word.tokens().contains(&tok)
        {
            return None;
        }
        let span = word.content_span().ok()?;
        return source
            .get(span.as_range())
            .map(|content| (span.start() as usize, content));
    }
    subspec_content(source, tok)
}

/// Emit the already selected structural keyword using its original content
/// extent. Literal closing bytes inside that extent remain data.
fn push_keyword_arg(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    let original = ctx.operand.and_then(|operand| operand.word.as_ref());
    if let Some((cstart, content)) =
        keyword_operand_content(ctx.full_source, tok, original, ctx.config)
    {
        if !content.is_empty() {
            push_subtoken(
                ctx.full_source,
                ctx.line_index,
                cstart,
                content,
                TokenKind::Keyword,
                entries,
            );
            return;
        }
    } else if original.is_some() {
        return;
    }
    push_token(
        ctx.line_index,
        ctx.full_source,
        tok,
        TokenKind::Keyword,
        0,
        entries,
    );
}

/// Encode entries into the LSP packed integer stream:
/// `[deltaLine, deltaCol, length, type, modifiers]` per token.
fn encode_entries(entries: &[Entry]) -> SemanticTokens {
    let mut data: Vec<u32> = Vec::with_capacity(entries.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    for (line, col, len, kind, modifiers) in entries {
        let delta_line = line.saturating_sub(prev_line);
        let delta_col = if delta_line == 0 {
            col.saturating_sub(prev_col)
        } else {
            *col
        };
        data.push(delta_line);
        data.push(delta_col);
        data.push(*len);
        data.push(*kind as u32);
        data.push(*modifiers);
        prev_line = *line;
        prev_col = *col;
    }
    SemanticTokens { data }
}

/// Number of packed integers per semantic token
/// (`[deltaLine, deltaCol, length, type, modifiers]`).
const TOKEN_STRIDE: usize = 5;

/// One minimal edit transforming a previous packed token stream
/// into a new one: starting at integer offset `start`, delete
/// `delete_count` integers and splice in `data`.
///
/// All three fields are token-aligned (multiples of
/// [`TOKEN_STRIDE`]) so the edit splits cleanly into whole
/// `SemanticToken`s, which is what the LSP `semanticTokens/full/
/// delta` wire shape requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenEdit {
    /// Integer offset into the previous `data` where the edit
    /// begins.
    pub start: u32,
    /// Number of integers to remove from the previous `data`.
    pub delete_count: u32,
    /// Replacement integers (the changed run of the new stream).
    pub data: Vec<u32>,
}

/// Compute the single minimal edit that turns `old` into `new`
/// by trimming the common leading and trailing tokens.
///
/// Operates at whole-token granularity: a token counts as common
/// only when its entire 5-integer group is identical, so the
/// returned offsets stay token-aligned.  Because the packed
/// encoding is *relative* (each token's delta is measured from
/// its predecessor), any change that shifts a token's position
/// perturbs its 5-tuple and pulls it into the replacement run —
/// so a prefix/suffix diff on the encoded array is correct
/// without re-deltifying the boundary.
///
/// Returns `None` when the streams are identical.
#[must_use]
pub fn diff(old: &[u32], new: &[u32]) -> Option<TokenEdit> {
    if old == new {
        return None;
    }
    let old_tokens = old.len() / TOKEN_STRIDE;
    let new_tokens = new.len() / TOKEN_STRIDE;
    let token = |buf: &[u32], i: usize| -> [u32; TOKEN_STRIDE] {
        let base = i * TOKEN_STRIDE;
        [
            buf[base],
            buf[base + 1],
            buf[base + 2],
            buf[base + 3],
            buf[base + 4],
        ]
    };
    let max_common = old_tokens.min(new_tokens);
    let mut prefix = 0;
    while prefix < max_common && token(old, prefix) == token(new, prefix) {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < max_common - prefix
        && token(old, old_tokens - 1 - suffix) == token(new, new_tokens - 1 - suffix)
    {
        suffix += 1;
    }
    let start = prefix * TOKEN_STRIDE;
    let delete_count = (old_tokens - prefix - suffix) * TOKEN_STRIDE;
    let data = new[start..(new_tokens - suffix) * TOKEN_STRIDE].to_vec();
    // Token streams are bounded well below `u32::MAX`; on the
    // theoretical overflow, return `None` so the caller falls back to
    // a full token set rather than emitting an invalid edit.
    let (Ok(start), Ok(delete_count)) = (u32::try_from(start), u32::try_from(delete_count)) else {
        return None;
    };
    Some(TokenEdit {
        start,
        delete_count,
        data,
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_proc_role_tests;

#[cfg(test)]
mod original_procedure_topology_colour_tests;

#[cfg(test)]
mod original_variable_argument_geometry_colour_tests;

#[cfg(test)]
mod original_realm_consumer_tests;

#[cfg(test)]
mod original_definition_vocabulary_tests;

#[cfg(test)]
mod selected_source_token_tests;

#[cfg(test)]
mod configured_semantic_receiver_tests;

#[cfg(test)]
mod original_comment_token_tests;

/// The iRules overlays (BIG-IP object references, `when`/procedure
/// declarations) over representative fixtures, read the way the server
/// supplies an iRules document: its own analysis, input and unit.
#[cfg(test)]
mod irules_overlay_tests;
