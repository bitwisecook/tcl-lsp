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
//! and running the Tcl tokenizer over them mis-colours the file rather than
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
use tcl_dialect::DialectSet;
use tcl_registry::CommandRegistry;
use tcl_registry::definer::{DefinerFamily, DefinitionBodyGrammar, MemberKind};

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
    /// Regex anchor: `^` `$` `\A` `\Z` `\b` `\B` `\m` `\M` `\y` `\Y`.
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
    /// `clock` locale modifier (`E` / `O`).
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
    /// in the legend, **not** `interface`: the retired Python legend reused the
    /// standard LSP `interface` type for this, which shadows its real meaning.
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
    /// an ordinary local (#898 §4).
    Parameter = 54,
    /// A `TclOO` / snit / itcl **method** — its declared name (`method foo {…}`)
    /// and its call sites (`my foo`, `$obj foo`).  The standard LSP type:
    /// `v1.11.4` typed these `function`, which was better than `v2.1.6`'s
    /// `string` but still conflated a method with a free procedure (#898 §2).
    Method = 55,
    /// A class name — `oo::class create Shape`, `oo::define Shape`.  Typed
    /// `string` by *both* v1.11.4 and v2.1.6 (#898 §2).
    Class = 56,
}

/// The iRules dialect key — the dialect a BIG-IP config's `ltm rule { … }`
/// bodies are written in.
const IRULES_DIALECT: &str = "f5-irules";

/// The iApps dialect key — the dialect an APL presentation's embedded `[ … ]`
/// Tcl is written in.
const IAPPS_DIALECT: &str = "f5-iapps";

/// `binary format`/`scan` specifier letters.
const BINARY_FORMAT_SPECIFIERS: &[u8] = b"aAbBhHcsSiInwWmrRfdxX@t";

/// Integer specifiers that accept a `u`/`s` signed/unsigned modifier
/// (Tcl 8.5+).
const BINARY_INT_SPECIFIERS: &[u8] = b"csSiIntwWmrR";

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

/// `TclOO` method-body helper commands (used inside a method body, not
/// definition-context members) with no `CommandSpec` **in the active
/// dialect** — the part of [`is_language_keyword_sub_keyword`]'s residue
/// specific to this crate (its clause-keyword half lives in the registry;
/// see that function's docs).
///
/// Both gained real, 9.0-gated registry specs with issue #923's
/// `ticklecharts` idx 51 (`tcl_registry::commands::tcl::oo_callback`), so
/// under a 9.0/9.1 profile the `LANGUAGE_KEYWORD` lookup already answers and
/// this list is inert. It still earns its place on 8.4-8.6, where the same
/// two words are only ever a hand-installed `proc ::oo::Helpers::callback`
/// (the "`TclOO` Tricks" wiki helper) or Tcllib `ooutil`'s `mymethod`: they
/// read as method-body keywords to a human either way, and the highlighter
/// has no package-load information to decide otherwise.
const METHOD_BODY_HELPER_SUB_KEYWORDS: &[&str] = &["callback", "mymethod"];

/// `true` for sub-keywords highlighted as `keyword` that are **not**
/// standalone commands, so they have no `CommandSpec` to carry the
/// `LANGUAGE_KEYWORD` trait, **and** are not definition-body members.
///
/// Definition-body member sub-keywords (`method`, `constructor`, `typemethod`,
/// `variable`, …) are deliberately **absent**: they are recognised
/// context-sensitively from the enclosing definer's `definition_body` grammar
/// (via [`crate::oo_body::is_member`] in [`emit_command_head`] for the script
/// form and [`insert_oo_define_keyword_overrides`] for the inline form), so a
/// same-named user proc outside a definition body is never mis-coloured and
/// `TclOO` and snit members behave identically. This residue only covers what
/// the grammar does not otherwise model: clause keywords of `if`/`try`/`switch`
/// ([`tcl_registry::traits::CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC`] —
/// shared with `xtask`'s `gen_tmlanguage_keywords` TextMate-grammar generator,
/// so the two never drift on which clause words are real keywords) and the
/// `TclOO` method-*body* helper commands
/// ([`METHOD_BODY_HELPER_SUB_KEYWORDS`], specific to this crate). The
/// standalone commands (`if`, `while`, `proc`, `when`, `oo::*`, …) come from
/// the registry's `LANGUAGE_KEYWORD` trait.
fn is_language_keyword_sub_keyword(name: &str) -> bool {
    tcl_registry::traits::CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC.contains(&name)
        || METHOD_BODY_HELPER_SUB_KEYWORDS.contains(&name)
}

/// Classify a command-head token name: a name is a `keyword`
/// when it carries the registry's `LANGUAGE_KEYWORD` trait or is one
/// of the non-command sub-keywords ([`is_language_keyword_sub_keyword`]); a
/// `::`-qualified name is a `namespace`; everything else is a
/// `function`.
///
/// The keyword / operator tests run against the head's *effective identity*
/// (`resolved`), so `interp alias {} myforeach {} foreach` makes `myforeach` a
/// keyword and a `rename foreach ""` stops the bare spelling being one — issue
/// #1185.  The `::`-qualified test stays on the written spelling: an imported
/// bare `test` resolves to `tcltest::test` without becoming a namespace token.
fn classify_command_head(head: CommandHead<'_>, registry: &CommandRegistry) -> TokenKind {
    let CommandHead {
        text: name,
        resolved,
        rebound,
        ..
    } = head;
    // A head whose registry binding was provably taken over is an ordinary
    // user command, whatever the built-in of the same spelling would have been.
    if rebound {
        return if name.contains("::") {
            TokenKind::Namespace
        } else {
            TokenKind::Function
        };
    }
    let is_keyword = registry.get(resolved).is_some_and(|s| {
        s.traits
            .contains(tcl_registry::prelude::Traits::LANGUAGE_KEYWORD)
    }) || is_language_keyword_sub_keyword(name);
    if is_keyword {
        TokenKind::Keyword
    } else if is_operator_command(resolved, registry) {
        // A bare operator used as a command head (`+ 3 4`, `tcl::mathop`
        // style).
        TokenKind::Operator
    } else if name.contains("::") {
        TokenKind::Namespace
    } else {
        TokenKind::Function
    }
}

/// `true` when `name` is one of the recognised `::tcl::mathop` operator
/// command heads (`+`, `in`, `eq`, `lt`, …) — the registry's
/// `Traits::OPERATOR_COMMAND` on `name`'s spec, already correctly and
/// exhaustively populated for every mathop-shaped operator by
/// `tcl_syntax::expr::operators` (issue #983's unification). Previously a
/// 10-symbol hand-typed list (`+ - * / > >= < <= == !=`) that missed every
/// word-form operator (`eq`/`ne`/`in`/`ni`/`lt`/`le`/`gt`/`ge`) and every
/// bitwise/shift symbol (`%`/`**`/`<<`/`>>`/`&`/`|`/`^`/`~`/`!`) entirely —
/// issue #986.
fn is_operator_command(name: &str, registry: &CommandRegistry) -> bool {
    registry.get(name).is_some_and(|spec| {
        spec.traits
            .contains(tcl_registry::prelude::Traits::OPERATOR_COMMAND)
    })
}

/// Extra "this argument names a written variable" positions for commands the
/// static [`CommandRegistry`] does not model — user procs whose parameters the
/// analyser inferred to alias a caller variable (`upvar $param`), and
/// `# tcl-lsp: stub … :var` / `:var_read` declarations.  Keyed by command /
/// proc name; each value is the 0-based argument indices (head excluded) that
/// name a variable — split by direction, since a *written* target highlights as
/// a `Variable` declaration and a *read* reference as a plain `Variable`.  Lets
/// the retag highlight `myset arr(key) …` / `myexists arr(key)` the same way it
/// highlights `set arr(key) …` / `info exists arr(key)` (issue #813 follow-up).
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
}

/// The per-direction abstention sets of a [`VarNameArgRoles`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RoleAmbiguity {
    write: FxHashSet<String>,
    read: FxHashSet<String>,
    command: FxHashSet<String>,
}

impl VarNameArgRoles {
    /// Infer the variable-name argument positions of every proc in `analysis`.
    #[must_use]
    pub fn from_analysis(analysis: &AnalysisResult) -> Self {
        Self::from_procs(analysis.all_procs.values())
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
        for proc in procs {
            let keys = proc_name_keys(proc);
            write.insert(&keys, &proc_var_write_indices(proc));
            read.insert(&keys, &proc_var_read_indices(proc));
            command.insert(&keys, &proc_command_indices(proc));
        }
        Self::from_builders(write, read, command)
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
        for part in parts {
            write.absorb(&part.write, &part.ambiguous.write);
            read.absorb(&part.read, &part.ambiguous.read);
            command.absorb(&part.command, &part.ambiguous.command);
        }
        Self::from_builders(write, read, command)
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
        self.write.is_empty() && self.read.is_empty() && self.command.is_empty()
    }

    /// Copy the write / read / command entries into the `out_*` maps without
    /// overwriting a name already present (source-derived stub roles that
    /// landed first win).
    fn extend_into(
        &self,
        out_write: &mut FxHashMap<String, Vec<u32>>,
        out_read: &mut FxHashMap<String, Vec<u32>>,
        out_command: &mut FxHashMap<String, Vec<u32>>,
    ) {
        for (name, indices) in &self.write {
            out_write
                .entry(name.clone())
                .or_insert_with(|| indices.clone());
        }
        for (name, indices) in &self.read {
            out_read
                .entry(name.clone())
                .or_insert_with(|| indices.clone());
        }
        for (name, indices) in &self.command {
            out_command
                .entry(name.clone())
                .or_insert_with(|| indices.clone());
        }
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
    let stripped = proc.qualified_name.trim_start_matches("::");
    if stripped != proc.name {
        keys.push(stripped.to_owned());
    }
    if proc.qualified_name != proc.name && proc.qualified_name != stripped {
        keys.push(proc.qualified_name.clone());
    }
    keys
}

/// Add `# tcl-lsp: stub` by-reference argument positions from the document
/// `source` to the `out_*` maps: `:var` (written) → `out_write`, `:var_read`
/// (read) → `out_read`, and `:command_prefix` (a command) → `out_command`.
/// Source-derived, so it applies on every token path (local and workspace)
/// without threading.  Cheap-gated: the line scan runs only when the source
/// mentions `stub`.
fn add_stub_var_roles(
    source: &str,
    out_write: &mut FxHashMap<String, Vec<u32>>,
    out_read: &mut FxHashMap<String, Vec<u32>>,
    out_command: &mut FxHashMap<String, Vec<u32>>,
) {
    if !source.contains("stub") {
        return;
    }
    let (stub_cmds, _exprs) = tcl_compiler::analyser::utils::scan_source_for_stubs(source);
    let overlay = tcl_compiler::analyser::types::build_stub_overlay(&stub_cmds);
    let indices_for = |sig: &tcl_registry::stub_overlay::StubSig, role: tcl_registry::ArgRole| {
        sig.args
            .iter()
            .enumerate()
            .filter(|(_, a)| a.role == role)
            .filter_map(|(i, _)| u32::try_from(i).ok())
            .collect::<Vec<u32>>()
    };
    for (name, sig) in overlay.iter() {
        for (role, out) in [
            (tcl_registry::ArgRole::VarWrite, &mut *out_write),
            (tcl_registry::ArgRole::VarRead, &mut *out_read),
            (tcl_registry::ArgRole::CommandPrefix, &mut *out_command),
        ] {
            let indices = indices_for(sig, role);
            if !indices.is_empty() {
                out.entry(name.to_owned()).or_insert(indices);
            }
        }
    }
}

/// Compute semantic tokens for the entire document.
#[must_use]
pub fn full(source: &str, dialect: &str, registry: &CommandRegistry) -> SemanticTokens {
    full_with_cu(source, dialect, registry, None)
}

/// Compute semantic tokens for an **APL** (iApp presentation) document.
///
/// APL is not Tcl — it is a declarative form-description grammar — so it does
/// not go through the Tcl segmenter at all.  Running the Tcl tokenizer over it
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
/// brace-delimited declarative config.  The Tcl tokenizer reads each stanza
/// body as one literal braced word and emits whole *lines* as `String` tokens
/// — 272 of `samples/bigip/bigip.conf`'s 302 tokens were exactly that, which
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
/// config file.  Read as config it produced nonsense — `when` / `if` / `switch`
/// became config *property keys* and `[HTTP::uri]` was not tokenised at all —
/// so the config lexer leaves those spans empty and they are re-walked here with
/// the iRules registry.  `registry` must therefore be the **iRules** one.
fn bigip_conf_entries(source: &str, registry: &CommandRegistry) -> Vec<Entry> {
    use tcl_bigip::conf_tokens::BigipTokenKind as B;

    let line_index = LineIndex::new(source);
    let mut entries: Vec<Entry> = Vec::new();

    for (bstart, bend) in tcl_bigip::conf_tokens::embedded_rule_bodies(source) {
        let (bstart, bend) = (bstart as usize, bend as usize);
        let Some(body) = source.get(bstart..bend) else {
            continue;
        };
        // The body's tokens come back positioned relative to the body text, so
        // shift them onto the document: a token on the body's first line also
        // needs the column the body started at.
        let origin = line_index.position_at_utf16(u32::try_from(bstart).unwrap_or(0), source);
        for (line, col, len, kind, mods) in
            collect_entries(body, IRULES_DIALECT, registry, None, None, None, None)
        {
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
        for (line, col, len, kind, mods) in
            collect_entries(body, IAPPS_DIALECT, registry, None, None, None, None)
        {
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
    dialect: &str,
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
    dialect: &str,
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
    dialect: &str,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
) -> SemanticTokens {
    full_with_cu_and_classes_and_roles(source, dialect, registry, cu, classes, None)
}

/// [`full_with_cu_and_classes`] with the workspace-merged inferred variable-name
/// argument roles ([`VarNameArgRoles`]), so a `myproc arr(key) …` call whose
/// `myproc` parameter aliases a caller variable highlights its array-element
/// target like `set arr(key) …` (issue #813 follow-up).  The project path
/// passes a cross-file index; `None` restricts the retag to the static registry
/// roles (plus source-derived stub roles).
#[must_use]
pub fn full_with_cu_and_classes_and_roles(
    source: &str,
    dialect: &str,
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
        },
    )
}

/// [`full_with_cu_and_classes_and_roles`] with the workspace-merged named
/// bareword instance-command index bundled in (issue #1312), so `CLASS
/// create NAME` resolves its class exactly like the single-file
/// [`full_with_cu_and_analysis`] path does — the project token-aggregation
/// path (`semantic_tokens_project`) is the one caller with a project-wide
/// [`NamedInstanceMap`] to hand; `WorkspaceTokenFacts::default()` (every
/// other caller) skips the merge.
#[must_use]
pub fn full_with_cu_and_facts(
    source: &str,
    dialect: &str,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> SemanticTokens {
    let entries = collect_entries(
        source,
        dialect,
        registry,
        cu,
        facts.classes,
        facts.proc_roles,
        facts.named_instances,
    );
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
    dialect: &str,
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
    dialect: &str,
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
    dialect: &str,
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
        },
    )
}

/// [`range_with_cu`] with an optional workspace-merged [`ClassHierarchy`] (see
/// [`full_with_cu_and_classes`]).
#[must_use]
pub fn range_with_cu_and_classes(
    source: &str,
    dialect: &str,
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
    dialect: &str,
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
        },
    )
}

/// [`range_with_cu_and_classes_and_roles`] with the workspace-merged named
/// bareword instance-command index bundled in (see [`full_with_cu_and_facts`]).
#[must_use]
pub fn range_with_cu_and_facts(
    source: &str,
    dialect: &str,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    facts: WorkspaceTokenFacts<'_>,
) -> SemanticTokens {
    let mut entries = collect_entries(
        source,
        dialect,
        registry,
        cu,
        facts.classes,
        facts.proc_roles,
        facts.named_instances,
    );
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

/// The process-wide iRules registry, for the dialect-independent
/// `when EVENT` overlay ([`special_arg_kinds`]) — resolved once so the
/// per-command fallback lookup skips `registry_for_dialect`'s cache mutex.
fn irules_registry() -> &'static CommandRegistry {
    static IRULES: std::sync::OnceLock<&'static CommandRegistry> = std::sync::OnceLock::new();
    IRULES.get_or_init(|| tcl_registry::registry_for_dialect("f5-irules"))
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
#[derive(Debug, Clone, Copy)]
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
    /// [`insert_lambda_literal_overrides`].
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
    CaseList(&'static tcl_registry::CaseListSpec, bool),
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
    // is classified as a command head — `proc p {args} {}` emitted `'}':function`
    // (#898 §7).  Recognised exactly (span length == content_offset + 1, last
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

/// Per-command argument-token classification overrides, keyed by the
/// representative token's start offset.  Two registry-driven cases:
///
/// * a `regexp` / `regsub` regex-pattern argument (the spec's
///   `pattern_type == Regex`, option-skipped first positional) →
///   [`ArgOverride::RegexPattern`] (sub-tokenised into ARE components);
/// * a `when EVENT` event-name argument → [`TokenKind::Event`].
///
/// `arg_texts` holds the command's argument words (`seg.texts[1..]`, head
/// excluded) borrowed as `&[&str]`.  The caller builds it once and shares it
/// with the registry-role and OO-body override passes, so the hot path makes
/// only a single bridging allocation per command.
#[allow(clippy::too_many_arguments)] // one override builder threading the whole per-command context
fn special_arg_kinds(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    oo_grammar: Option<&'static DefinitionBodyGrammar>,
    scoped_env: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    arg_texts: &[&str],
    object_classes: &ObjectClassMap,
    object_collections: &ObjectClassMap,
    classes: Option<&ClassHierarchy>,
    dialect: DialectSet,
    extra_var_write: &FxHashMap<String, Vec<u32>>,
    extra_var_read: &FxHashMap<String, Vec<u32>>,
    extra_command: &FxHashMap<String, Vec<u32>>,
    deferred_role: bool,
) -> FxHashMap<u32, ArgOverride> {
    let mut overrides = FxHashMap::default();

    // `when EVENT` — the literal event-name argument.  Event handlers come
    // from the registry's `IS_EVENT_HANDLER` trait; the event name is the
    // first argument, the same convention the completion provider's
    // event-name surface uses for the trait.  The overlay is deliberately
    // dialect-independent — iRules snippets are routinely opened in generic
    // Tcl buffers, and `when EVENT` was event-coloured there long before the
    // trait dispatch — so a head the document registry does not know is
    // resolved against the cached iRules registry before giving up.
    if registry
        .get(head)
        .or_else(|| irules_registry().get(head))
        .is_some_and(|s| s.traits.contains(tcl_registry::Traits::IS_EVENT_HANDLER))
        && let (Some(tok), Some(text)) = (seg.argv.get(1), seg.texts.get(1))
        && matches!(tok.kind, TokenType::Esc)
        && is_event_name(text)
    {
        overrides.insert(tok.span.start(), ArgOverride::Kind(TokenKind::Event));
    }

    insert_regex_overrides(seg, registry, head, arg_texts, &mut overrides);
    insert_format_overrides(seg, registry, head, arg_texts, &mut overrides);

    // `proc NAME …` — the name argument is a function definition.  Procedure
    // definers come from the registry's `DEFINES_PROCEDURE` trait; a spec
    // that also carries a `definition_body` grammar is a *class* definer
    // (`oo::class` & co.), whose name argument is claimed by
    // `insert_definer_class_name_override` instead.  The name position is
    // the spec's `ArgRole::Name` argument.
    if let Some(spec) = registry.get(head)
        && spec
            .traits
            .contains(tcl_registry::Traits::DEFINES_PROCEDURE)
        && spec.definition_body.is_none()
        && let Some(&name_idx) = registry
            .arg_indices_for_role(head, arg_texts, tcl_registry::ArgRole::Name)
            .first()
        && let Some(tok) = seg.argv.get(name_idx + 1)
    {
        overrides
            .entry(tok.span.start())
            .or_insert(ArgOverride::ProcNameDef);
    }

    insert_option_and_subcommand_overrides(seg, registry, head, dialect, &mut overrides);
    insert_object_method_overrides(
        seg,
        registry,
        object_classes,
        object_collections,
        classes,
        &mut overrides,
    );
    insert_generic_option_overrides(seg, registry, head, &mut overrides);
    // `insert_oo_define_keyword_overrides` must run before the generic enum
    // pass below: `oo::define`'s inline definition word (`method`,
    // `constructor`, …) is *also* one of `OO_DEFINE_SUBCOMMAND_VALUES`'
    // completion/hover entries, and `overrides` is a first-writer-wins map
    // (`.or_insert`) — the more specific inline-keyword classification must
    // claim that span first, or the generic closed-set-value pass claims it
    // as `EnumMember` instead (mirroring the existing `ArgRole::Keyword`
    // carve-out in `insert_enum_value_overrides`, issue #760, which this
    // dynamic `definition_body`-driven case isn't modelled by).
    insert_oo_define_keyword_overrides(seg, registry, dialect, &mut overrides);
    insert_enum_value_overrides(seg, registry, head, dialect, &mut overrides);
    insert_definer_class_name_override(seg, registry, &mut overrides);
    insert_lambda_literal_overrides(seg, registry, head, deferred_role, &mut overrides);
    insert_case_list_override(seg, registry, &mut overrides);
    insert_role_overrides(seg, registry, head, arg_texts, &mut overrides);
    insert_oo_body_overrides(seg, oo_grammar, arg_texts, dialect, &mut overrides);
    insert_scoped_subcommand_overrides(seg, scoped_env, head, &mut overrides);
    insert_multiname_var_overrides(seg, registry, head, arg_texts, oo_grammar, &mut overrides);
    insert_ref_var_overrides(seg, registry, head, &mut overrides);
    insert_loop_var_overrides(seg, registry, head, arg_texts, &mut overrides);
    insert_param_list_overrides(seg, registry, head, arg_texts, &mut overrides);
    insert_var_role_overrides(
        seg,
        registry,
        head,
        extra_var_write,
        extra_var_read,
        &mut overrides,
    );
    insert_command_role_overrides(seg, registry, head, extra_command, &mut overrides);

    overrides
}

/// Loop-variable specs → variable declarations.
///
/// Every position comes from the registry's [`ArgRole::LoopVarList`]
/// ([`ArgOverride::LoopVarList`]'s [`collect_loop_var_list`] then emits each
/// name — a bareword or the elements of a braced list — as a variable). That
/// covers both the fixed shape (`dict for {k v} …`, `dict map {k v} …`) and
/// the repeating one (`foreach v1 l1 ?v2 l2 …? body`, `lmap` likewise), whose
/// stride and excluded trailing body are declared as a
/// [`tcl_registry::RepeatedArgLayout`] on the spec rather than re-derived
/// here from the command's name (issue #1185) — so the explicitly global
/// `::foreach` is covered too, and a same-named user proc is not.
///
/// Highlighting only: the loop bodies already resolve these reads via the
/// analyser's scope tracking.
fn insert_loop_var_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    arg_texts: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // `i` indexes the argument words → `argv[i + 1]`.
    for i in registry.arg_indices_for_role(head, arg_texts, tcl_registry::ArgRole::LoopVarList) {
        if let Some(tok) = seg.argv.get(i + 1)
            && matches!(tok.kind, TokenType::Esc | TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::LoopVarList);
        }
    }
}

/// Procedure parameter lists → parameter declarations.  The registry's
/// [`ArgRole::ParamList`] marks the braced `{a b {c default}}` word of `proc`,
/// the iRules `proc`, and snit `method` / `typemethod`; it is tagged
/// [`ArgOverride::ParamList`] so [`collect_param_list`] emits each parameter
/// name as a `Parameter` declaration (and classifies any default value).
fn insert_param_list_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    arg_texts: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    for i in registry.arg_indices_for_role(head, arg_texts, tcl_registry::ArgRole::ParamList) {
        // A braced literal list (`{a b}`) or a **bare** single-name list — Tcl
        // accepts `proc unknown args {…}` / `proc auto_execok name {…}` without
        // braces, and Tcl's own `init.tcl` / `word.tcl` use it.  That form is an
        // unquoted `Esc` word, and skipping it left the parameter painted as a
        // plain string.  A *quoted* list is not a literal name list, so it is
        // still left alone.
        let Some(tok) = seg.argv.get(i + 1) else {
            continue;
        };
        let literal_list = match tok.kind {
            TokenType::Str => true,
            TokenType::Esc => !tok.in_quote && seg.single_token_word.get(i + 1) == Some(&true),
            _ => false,
        };
        if literal_list {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::ParamList);
        }
    }
}

/// Highlight the local-variable names bound by `upvar`'s
/// `?level? otherVar localVar ?otherVar localVar ...?` pair tail.
///
/// The sibling by-reference shapes — `namespace upvar ns o l ?o l?` and
/// `dict update dictVar key varName ?key varName? body` — declare their pair
/// tail as a [`tcl_registry::RepeatedArgLayout`] on their subcommand spec and
/// are handled generically by [`insert_multiname_var_overrides`]'s
/// `VarWrite` walk (issue #1185).
///
/// `upvar`'s own layout is not a [`tcl_registry::RepeatedArgLayout`] because
/// the registry already models it more precisely: its
/// [`tcl_registry::FrameEffectSpec`] declares
/// [`FrameArgLayout::AliasPairs`] (other/local pairs) with
/// [`FrameLevelWord::ArityParity`] (C Tcl reads the optional level word off
/// the *argument count parity*, never off the word's text — `Tcl_UpvarObjCmd`
/// tests `objc`; tclsh 9.0.4 / 8.6.14 agree).  This reads those two facts, so
/// the explicitly global `::upvar` behaves like the bare form and a
/// same-named user proc does not.
///
/// Highlighting only: the analyser already scopes these locals.  A `$`-computed
/// / array / quoted name is skipped.
fn insert_ref_var_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_registry::{FrameArgLayout, FrameLevelWord};
    let Some(effect) = registry.get(head).and_then(|s| s.frame_effect) else {
        return;
    };
    if effect.layout != FrameArgLayout::AliasPairs
        || effect.level_word != FrameLevelWord::ArityParity
    {
        return;
    }
    let n = seg.texts.len() - 1; // argument count
    if n < 2 {
        return;
    }
    // The level word is present exactly when the argument count is odd, and
    // shifts the first local from texts[2] to texts[3].
    let start = if n % 2 == 1 { 3 } else { 2 };
    for pos in (start..seg.texts.len()).step_by(2) {
        if let Some(tok) = seg.argv.get(pos)
            && matches!(tok.kind, TokenType::Esc)
            && !tok.in_quote
            && is_plain_var_name(&seg.texts[pos])
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::VarDecl);
        }
    }
}

/// Highlight the name arguments of a multi-name variable-declaring command —
/// `global name ?name ...?` (every argument), `variable name ?value name
/// value ...?` at namespace level (every *even* argument; the interleaved
/// values are left alone).
///
/// The stride is registry data: each spec declares a
/// [`tcl_registry::RepeatedArgLayout`] for its `VarWrite` tail, so this reads
/// [`ArgRole::VarWrite`] positions and never names a command or re-derives a
/// stride (issue #1185).  That also makes the explicitly global spellings
/// (`::global`, `::variable`) behave like the bare ones.
///
/// A `variable` *inside a definition body* is a grammar member handled by
/// [`insert_oo_body_overrides`] (where `TclOO` declares every name and snit
/// only the leading one), so this steps aside whenever a definition-body
/// grammar is in force.
///
/// Highlighting only: the analyser already tracks every one of these names via
/// the commands' lowering hooks, so no diagnostic depends on this.  An array
/// element (`arr(x)`), a `$`-computed name, or a quoted word is skipped so its
/// inner `$var` sub-tokens survive.
fn insert_multiname_var_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    arg_texts: &[&str],
    oo_grammar: Option<&'static DefinitionBodyGrammar>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if oo_grammar.is_some() {
        return;
    }
    for i in registry.arg_indices_for_role(head, arg_texts, tcl_registry::ArgRole::VarWrite) {
        let pos = i + 1;
        if let Some(tok) = seg.argv.get(pos)
            && matches!(tok.kind, TokenType::Esc)
            && !tok.in_quote
            && seg.texts.get(pos).is_some_and(|t| is_plain_var_name(t))
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::VarDecl);
        }
    }
}

/// Apply the enclosing definition-body grammar to a member call: recurse its
/// script bodies ([`ArgOverride::BodyScript`]), highlight its parameter list
/// ([`ArgOverride::ParamList`]), and declare its variable names
/// ([`ArgOverride::VarDecl`]).  The member keywords (`method`, `typemethod`,
/// `constructor`, `variable`, …) have no standalone `CommandSpec`; their layout
/// comes entirely from the registry grammar ([`crate::oo_body`]).
///
/// Only fires when `oo_grammar` is `Some` — i.e. this segment is a top-level
/// word of a definition body — so a same-named user proc is never
/// misclassified.
/// Colour the ensemble operation word of a scoped command as a subcommand
/// keyword — the `set` / `enable` in `top set …` / `top enable` inside a
/// `report::defstyle` style script.  Fires only when `head` is a command of the
/// enclosing scoped environment and its op resolves against that command's
/// operation set; the whole set is registry data (see [`tcl_registry::scoped`]).
fn insert_scoped_subcommand_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    scoped_env: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    head: &str,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let Some(env) = scoped_env else {
        return;
    };
    let Some(cmd) = env.command(head) else {
        return;
    };
    if let Some(op_text) = seg.texts.get(1)
        && cmd.subcommand(op_text).is_some()
        && let Some(tok) = seg.argv.get(1)
    {
        overrides
            .entry(tok.span.start())
            .or_insert(ArgOverride::SubcommandKeyword);
    }
}

fn insert_oo_body_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    oo_grammar: Option<&'static DefinitionBodyGrammar>,
    arg_texts: &[&str],
    dialect: DialectSet,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let Some(grammar) = oo_grammar else {
        return;
    };
    // A member call inside a definition *body*: the member keyword is the
    // command head (argv 0), so its arguments start at argv 1.
    insert_oo_member_overrides(
        seg,
        grammar,
        &seg.texts[0].clone(),
        arg_texts,
        0,
        dialect,
        overrides,
    );
}

/// Tag the member-call words of a definition-body member — its declared name,
/// parameter list, body script(s) and declared variables — from the grammar's
/// argument roles.
///
/// `base` is the `argv` index of the *member keyword*, so this serves both
/// shapes of a member call: inside a definition body the keyword is the command
/// head (`base = 0`, `method m {} {…}`), while the one-liner definer form puts
/// it after the class/object target (`base = 2`,
/// `oo::define C method m {} {…}`).  Sharing one path is what stops the
/// one-liner form silently losing its name / parameters / body — it used to get
/// only its keyword marked.
fn insert_oo_member_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    grammar: &'static DefinitionBodyGrammar,
    head: &str,
    arg_texts: &[&str],
    base: usize,
    dialect: DialectSet,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if !crate::oo_body::is_member(grammar, head) {
        return;
    }
    // A wrapper member (`self` / itcl `public` / `protected` / `private`) nests
    // an inner member keyword at arg 0 (`public method …`); it reads as a
    // keyword too, context-sensitively from the grammar.
    if grammar
        .member(head)
        .is_some_and(|m| m.kind == MemberKind::Wrapper)
        && arg_texts
            .first()
            .is_some_and(|inner| grammar.is_member(inner))
        && let Some(tok) = seg.argv.get(base + 1)
    {
        overrides
            .entry(tok.span.start())
            .or_insert(ArgOverride::Kind(TokenKind::Keyword));
    }
    // Script bodies — recurse (only a braced `Str` word carries a script).
    for idx in crate::oo_body::member_body_indices_in(grammar, head, arg_texts, dialect) {
        if let Some(tok) = seg.argv.get(base + idx + 1)
            && matches!(tok.kind, TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::BodyScript);
        }
    }
    // The member's declared name (`method foo …`, `property p …`).  The grammar
    // has always carried this role; consuming it is what stops a method name
    // painting as a plain string (#898 §2).
    for idx in crate::oo_body::member_name_indices_in(grammar, head, arg_texts, dialect) {
        if let Some(tok) = seg.argv.get(base + idx + 1)
            && matches!(tok.kind, TokenType::Esc | TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::MemberName);
        }
    }
    // Parameter lists — their names are declarations, like a `proc`'s.
    for idx in crate::oo_body::member_param_indices_in(grammar, head, arg_texts, dialect) {
        if let Some(tok) = seg.argv.get(base + idx + 1)
            && matches!(tok.kind, TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::ParamList);
        }
    }
    // Reference-only members: `superclass A B` names classes, `export m` names
    // methods.  They declare nothing, but their arguments are not free strings —
    // they name an entity defined elsewhere, so they take that entity's type.
    if let Some((ref_kind, indices)) = crate::oo_body::member_ref_indices(grammar, head, arg_texts)
    {
        let ov = match ref_kind {
            tcl_registry::definer::MemberRefKind::Class => ArgOverride::ClassNameRef,
            tcl_registry::definer::MemberRefKind::Method => ArgOverride::Kind(TokenKind::Method),
        };
        for idx in indices {
            if let Some(tok) = seg.argv.get(base + idx + 1)
                && matches!(tok.kind, TokenType::Esc | TokenType::Str)
                && !tok.in_quote
            {
                overrides.entry(tok.span.start()).or_insert(ov);
            }
        }
    }
    // Declared variable / component names (`variable a b c`, `typevariable v`,
    // `component c`, `onconfigure -opt valueVar …`).
    for idx in crate::oo_body::member_var_indices_in(grammar, head, arg_texts, dialect) {
        if let Some(tok) = seg.argv.get(base + idx + 1)
            && matches!(tok.kind, TokenType::Esc)
            && !tok.in_quote
            && is_plain_var_name(&seg.texts[base + idx + 1])
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::VarDecl);
        }
    }
    // Closed grammar options and namespace references retain their ordinary
    // token kinds even though definition members have no standalone command
    // specs. The positions come from the same generic member-role walker as
    // names, parameters, bodies, and variables above.
    for idx in crate::oo_body::member_option_indices_in(grammar, head, arg_texts, dialect) {
        if let Some(tok) = seg.argv.get(base + idx + 1) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
    }
    for idx in crate::oo_body::member_namespace_indices_in(grammar, head, arg_texts, dialect) {
        if let Some(tok) = seg.argv.get(base + idx + 1) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::Namespace));
        }
    }
}

/// Regex-pattern overrides for a `pattern_type == Regex` command.
///
/// Both facts are registry data: the *language* from
/// [`tcl_registry::CommandSpec::pattern_type`], and the *position* from the
/// [`tcl_registry::ArgRole::Pattern`] role the spec's resolver reports —
/// which is what shifts the pattern past a call's leading switches without
/// this walker re-implementing option parsing (issue #1185). The paired
/// `regsub` replacement template is claimed by
/// [`insert_format_overrides`] through its own `FormatType::Regsub` family.
fn insert_regex_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    arg_texts: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if !registry
        .get(head)
        .and_then(|s| s.pattern_type)
        .is_some_and(|p| p == tcl_registry::patterns::PatternType::Regex)
    {
        return;
    }
    // Sub-tokenise only the *literal* fragments of the pattern word as regex:
    // in `"abc$var.*"` the `abc` / `.*` fragments are regex, but `$var` is
    // variable interpolation Tcl resolves before `regexp` sees it (and
    // `"[cmd]"` is command substitution, not a char class). Marking the
    // literal fragments — not the whole word — leaves the `Var` / `Cmd`
    // fragments to the default classifier, so they render as Tcl and never
    // overlap the regex sub-tokens.
    let declared = registry.arg_indices_for_role(head, arg_texts, tcl_registry::ArgRole::Pattern);
    let positions: Vec<usize> = if declared.is_empty() {
        // A `Regex` command whose spec does not (yet) declare where its
        // pattern sits: fall back to the first positional word past the
        // leading switches, the layout every stock regex command shares.
        let mut idx = 0;
        while idx < arg_texts.len() && arg_texts[idx].starts_with('-') && arg_texts[idx] != "--" {
            if arg_texts[idx] == "-start" && idx + 1 < arg_texts.len() {
                idx += 2;
            } else {
                idx += 1;
            }
        }
        if idx < arg_texts.len() && arg_texts[idx] == "--" {
            idx += 1;
        }
        vec![idx]
    } else {
        declared
    };
    for idx in positions {
        if let Some(tok) = seg.argv.get(idx + 1) {
            mark_literal_fragments(seg, tok.span, ArgOverride::RegexPattern, overrides);
        }
    }
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
    ov: ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    for t in &seg.all_tokens {
        if t.span.start() >= word_span.start()
            && t.span.end() <= word_span.end()
            && matches!(t.kind, TokenType::Str | TokenType::Esc)
        {
            overrides.entry(t.span.start()).or_insert(ov);
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
            mark_literal_fragments(seg, full_span, ArgOverride::RegexPattern, overrides);
        }
    }
}

/// Conversion-string overrides for every format family the registry declares
/// — sprintf (`format` / `scan`), `clock`'s field string (a fixed argument or
/// the `-format` option value), `binary`'s cursor spec, and `regsub`'s
/// replacement template.
///
/// Entirely registry-driven ([`CommandRegistry::format_string_args`]): the
/// *position* comes from the [`tcl_registry::ArgRole::FormatString`] /
/// `ScanFormat` roles the specs and resolvers declare, and the *family* from
/// `format_string_type`. No command name appears here, so the explicitly
/// global spellings (`::format`, `::clock`, …) — which the previous
/// `match head { "format" => … }` silently missed — resolve identically, and
/// a same-named user proc or a dynamic head simply declares no family and is
/// left alone (issue #1185).
///
/// The `Regsub` family marks only the word's *literal* fragments, matching
/// how the regex pattern beside it is treated: a `$var` inside a replacement
/// template is variable interpolation Tcl performs before `regsub` sees it,
/// not part of the template.
fn insert_format_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    arg_texts: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    for found in registry.format_string_args(head, arg_texts) {
        let Some(tok) = seg.argv.get(found.index + 1) else {
            continue;
        };
        match found.kind {
            tcl_registry::FormatType::Sprintf => {
                overrides.insert(tok.span.start(), ArgOverride::SprintfFormat);
            }
            tcl_registry::FormatType::Clock => {
                overrides.insert(tok.span.start(), ArgOverride::ClockFormat);
            }
            tcl_registry::FormatType::Binary => {
                overrides.insert(tok.span.start(), ArgOverride::BinaryFormat);
            }
            tcl_registry::FormatType::Regsub => {
                mark_literal_fragments(seg, tok.span, ArgOverride::RegsubReplace, overrides);
            }
        }
    }
}

/// Known `-option` switches → `Decorator` (only real options declared in
/// the registry, so `puts -foo` stays a string); subcommand word at arg
/// index 1 → keyword carrying `defaultLibrary`.  Both consult the command's
/// registry spec.
///
/// The recognised-option set is the [`OptionSpec`]-driven answer to
/// issue #748 ("highlight words starting with `-` as options"): rather than
/// treat every `-`-prefixed word as an option — which would mishighlight a
/// bare minus, a negative number, or a `-$var` substitution — we highlight
/// exactly the switches the command declares.  The set spans the command's
/// flat [`CommandSpec::options`] *and* every [`CommandForm`]'s options (via
/// [`CommandSpec::switch_names`]), plus — when arg 1 selects a known
/// subcommand — that subcommand's own options (via
/// [`SubCommand::switch_names`]).  That is what makes the issue's own
/// example, `file delete -force filename`, light up: `-force` is declared on
/// the `delete` subcommand, not on `file` itself.
///
/// Matching is against the literal word text, so `-$variable` /
/// `-{$variable}` / `-[command]` — whose word text is not a declared option
/// name — never match; only a literal `-force`-style word does.
///
/// [`OptionSpec`]: tcl_registry::OptionSpec
/// Whether an option value's role is re-coloured by another semantic-token
/// pass (`insert_role_overrides` for `Body`/`Expr`, `insert_var_decl_overrides`
/// for `VarWrite`, `insert_format_overrides` for a conversion string).  Such
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

/// Resolve a `-word` against a command's declared option names, accepting a
/// unique prefix (`-inc` ⇒ `-increasing`) the way Tcl's option parsing
/// (`Tcl_GetIndexFromObj`) does.  An exact match always wins; an ambiguous
/// prefix (two distinct options share it, e.g. `lsort -i`) or no match returns
/// `None`.  A bare `-` never prefix-matches (only an exact-declared `-` /
/// `--` option does).
fn resolve_option_prefix<'a>(word: &str, names: &[&'a str]) -> Option<&'a str> {
    if let Some(exact) = names.iter().copied().find(|n| *n == word) {
        return Some(exact);
    }
    // Prefix matching needs at least one character past the leading dash.
    if word.len() < 2 {
        return None;
    }
    let mut matched: Option<&'a str> = None;
    for &n in names {
        if n.starts_with(word) {
            match matched {
                None => matched = Some(n),
                Some(prev) if prev == n => {}
                Some(_) => return None, // ambiguous: two distinct options
            }
        }
    }
    matched
}

fn insert_option_and_subcommand_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    dialect: DialectSet,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let Some(spec) = registry.get(head) else {
        return;
    };

    // Command-level options — the flat `options` list plus every
    // command-form's options.  Dialect-agnostic (`None`): a switch is still
    // visually an option even when it was introduced in a later Tcl release.
    let mut option_names = spec.switch_names(None);

    // Value-taking options whose value is a *generic* value — those get the
    // distinct `OptionValue` colour (the option/value split of issue #748).
    // Options whose value carries an analysis role (a `-command` script, a
    // `-textvariable` name, …) are deliberately excluded here so their value
    // is claimed by the role/var-decl passes instead (`BodyScript`, `VarDecl`,
    // …) — those run after this pass and would otherwise be blocked by the
    // `OptionValue` override.  Keyed name/alias → spec so multi-value arity
    // (`Fixed`/`Rest`) can colour every value word via `value_indices`.
    let mut value_options: FxHashMap<&str, &'static tcl_registry::hover::OptionSpec> =
        FxHashMap::default();
    let mut collect_value_options = |opts: &'static [tcl_registry::hover::OptionSpec]| {
        for opt in opts {
            // Skip options whose value carries an analysis role (claimed by the
            // role/var passes) or a declared enum set (claimed as `EnumMember`
            // by `insert_enum_value_overrides`) — leave those for the more
            // specific pass; only generic values get the `OptionValue` colour.
            if opt.takes_value()
                && !role_claimed_by_token_pass(opt.value_role())
                && opt.value_values().is_empty()
            {
                value_options.insert(opt.name, opt);
                for alias in opt.aliases {
                    value_options.insert(alias, opt);
                }
            }
        }
    };
    collect_value_options(spec.options);
    for form in spec.command_forms {
        collect_value_options(form.options);
    }

    // A known subcommand at arg index 1 is highlighted as a keyword, and its
    // per-subcommand options (`file delete -force`, `file link -symbolic`)
    // join the recognised set.  A unique-prefix abbreviation (`string le`)
    // resolves like Tcl's ensemble dispatch.
    if let Some(sub_text) = seg.texts.get(1)
        && let Some(sub) = spec.resolve_subcommand_for_dialect(sub_text, dialect)
    {
        option_names.extend(sub.switch_names(None, spec.dialects));
        collect_value_options(sub.options);
        if let Some(tok) = seg.argv.get(1) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::SubcommandKeyword);
        }

        // Two-level ensembles (`info object <subcommand>`, `info class
        // <subcommand>`): the word after the first-level subcommand is itself a
        // subcommand keyword, not a string (issue #798).  `is_sub_subcommand`
        // accepts a unique prefix (`info object cl` ⇒ `class`) the way Tcl's
        // ensemble dispatch does.  General over any registry-declared two-level
        // ensemble, not just `info`.
        if let Some(sub_sub_text) = seg.texts.get(2)
            && sub
                .resolve_sub_subcommand_for_dialect(sub_sub_text, dialect)
                .is_some()
            && let Some(tok) = seg.argv.get(2)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::SubcommandKeyword);
        }
    }

    for (i, text) in seg.texts.iter().enumerate().skip(1) {
        // Resolve `-word` against the declared option set, accepting a unique
        // prefix (`lsort -inc` ⇒ `-increasing`) the way Tcl's option parsing
        // (`Tcl_GetIndexFromObj`) does; an ambiguous prefix (`lsort -i`) is not
        // a recognised option.
        if text.starts_with('-')
            && let Some(canonical) = resolve_option_prefix(text, &option_names)
            && let Some(tok) = seg.argv.get(i)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);

            // The value word(s) this option consumes are re-coloured
            // `OptionValue`.  Arity-aware (`value_indices` handles One / Fixed /
            // Rest and stops at `--`); only *literal* values (`Esc`/`Str`) are
            // re-coloured — a `$var` / `[cmd]` substitution keeps its own
            // highlight, and a value that is itself a recognised option stays a
            // `Decorator` (the `or_insert` above already claimed it).
            if let Some(opt) = value_options.get(canonical) {
                for vi in opt.value_indices(&seg.texts, i) {
                    if let Some(val_tok) = seg.argv.get(vi)
                        && matches!(val_tok.kind, TokenType::Esc | TokenType::Str)
                    {
                        overrides
                            .entry(val_tok.span.start())
                            .or_insert(ArgOverride::Kind(TokenKind::OptionValue));
                    }
                }
            }
        }
    }
}

/// Whether a command's *head word* is a runtime-computed (non-static) command
/// name rather than a statically-resolvable one: a `$var` / `[cmd]`
/// substitution head, or a multi-fragment word (`chartV$node`, `${prefix}cmd`).
///
/// The command name of such a call is only known at runtime — an object handle
/// dispatched through a variable (`$chart method …`, #748), a `[Class new]` /
/// `[dict get …]` constructor-or-lookup result (#797), a computed command
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
/// the overwhelmingly-common `-switch value` shape.  This is the fallback half
/// of issue #748: colour those pairs like any built-in's.
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
    registry: &CommandRegistry,
    head: &str,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // Only unknown heads — a registered command's declared option set is the
    // authority, and a bare `-word` there (`puts -foo`) is deliberately a
    // string.
    if registry.get(head).is_some() {
        return;
    }
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

/// Object-handle → class-name map for the current document, keyed by the
/// handle text a `$var method` dispatch presents (minus the leading `$`) —
/// a scalar (`chart`) or array element (`arr(key)`).  Built once per document
/// from the [`CompilationUnit`] by
/// [`tcl_compiler::object_types::object_handle_classes`].
type ObjectClassMap = std::collections::HashMap<String, std::collections::HashSet<String>>;

/// Bareword instance-command name → qualified class name (issue #1312),
/// built from [`AnalysisResult::instance_classes`] gated on
/// [`AnalysisResult::created_instance_commands`] — the same contract the
/// LSP's `receiver_instance_class` uses.  Merged into [`ObjectClassMap`] by
/// [`collect_entries`] so a `CLASS create NAME` object types exactly like a
/// `set var [CLASS new]` one.
pub type NamedInstanceMap = std::collections::HashMap<String, String>;

/// The optional workspace-merged facts a semantic-tokens request can enrich
/// its object-dispatch resolution with — [`ClassHierarchy`] (cross-file
/// classes), [`VarNameArgRoles`] (cross-file proc parameter roles), and
/// [`NamedInstanceMap`] (cross-file `CLASS create NAME` bindings, issue
/// #1312).  Bundled into one `Copy` struct so the `range_*` full-arity entry
/// point stays within budget instead of growing a ninth positional
/// parameter.
#[derive(Clone, Copy, Default)]
pub struct WorkspaceTokenFacts<'a> {
    /// The workspace-merged class hierarchy, or the local single-file one.
    pub classes: Option<&'a ClassHierarchy>,
    /// The workspace-merged (or local) inferred proc parameter roles.
    pub proc_roles: Option<&'a VarNameArgRoles>,
    /// The workspace-merged (or local) `CLASS create NAME` bareword
    /// instance-command index (issue #1312).
    pub named_instances: Option<&'a NamedInstanceMap>,
}

/// Build a [`NamedInstanceMap`] from `analysis` — `None` when `analysis`
/// created no named instances, so the merge in [`collect_entries`] is a
/// no-op lookup-free skip for the overwhelming majority of documents.
fn named_instances_from_analysis(analysis: &AnalysisResult) -> NamedInstanceMap {
    analysis
        .instance_classes
        .iter()
        .filter(|(name, _)| analysis.created_instance_commands.contains(name.as_str()))
        .map(|(name, class)| (name.clone(), class.clone()))
        .collect()
}

/// Precise `$obj method …` highlighting via the registry's object-class model —
/// the object-handle half of issue #748.
///
/// When the command head is an object handle whose class is known — a `$var`
/// bound by `set var [Class new]` (tracked by [`tcl_compiler::object_types`]),
/// or a direct `[Class new] method …` dispatch — and the class declares
/// `method`, the method word is highlighted as a function and the method's
/// declared options / option values are coloured exactly like a built-in's
/// (`Decorator` for the switch, `EnumMember` for a closed-set value, else
/// `OptionValue`).
///
/// Runs before [`insert_generic_option_overrides`]: a recognised method's
/// options are claimed precisely first, and anything it leaves (an option not
/// in the spec, an un-provenanced receiver) is picked up by the generic
/// shape-based fallback.  A `$var` / `[cmd]` option value keeps its own
/// highlight; only literal (`Esc`/`Str`) values are recoloured.
fn insert_object_method_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    object_classes: &ObjectClassMap,
    object_collections: &ObjectClassMap,
    classes: Option<&ClassHierarchy>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let (Some(head_tok), Some(head_text), Some(method)) =
        (seg.argv.first(), seg.texts.first(), seg.texts.get(1))
    else {
        return;
    };
    // Candidate receiver classes implied by the head's shape: a `$var` object
    // handle, a direct `[Class new] …` constructor, a `[dict get $coll $k]`
    // / `[lindex $coll $i]` retrieval from an object collection (issue #797),
    // or a bareword instance-command name bound by a positional create call
    // (`ttk::treeview .t` / a registry naming factory — issue #927; `object_classes`
    // is name-keyed regardless of whether the name came from a `set` LHS or a
    // bareword factory, so the same map already carries these — see
    // `object_types::harvest_unit`'s `Statement::Call` arm).
    let mut candidates: Vec<String> = match head_tok.kind {
        TokenType::Var => object_handle_name(head_text)
            .and_then(|name| object_classes.get(name))
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default(),
        // `[Class new] method …`: a registry factory, else a *user* class named
        // by the constructor head (resolved against the class hierarchy, which
        // is workspace-merged, so a class defined in another file resolves),
        // else a `[dict get $coll $k]` retrieval from an object collection.
        TokenType::Cmd => {
            if let Some(cls) = constructor_class_of_head(head_text, registry) {
                vec![cls.to_string()]
            } else if let Some(cls) = user_constructor_class_of_head(head_text, classes, registry) {
                vec![cls]
            } else {
                collection_head_element_classes(head_text, registry, object_collections)
                    .map(|s| s.iter().cloned().collect())
                    .unwrap_or_default()
            }
        }
        TokenType::Esc => object_classes
            .get(head_text.as_str())
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
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
    if let Some(method_sub) = candidates
        .iter()
        .find_map(|cls| registry.instance_method(cls, method))
    {
        mark_method_word(seg, overrides);
        insert_registry_method_options(seg, method_sub, overrides);
        return;
    }
    // 2. User-defined class — resolve the method through the class hierarchy
    //    (workspace-wide when a project index is supplied, so a class defined in
    //    another file resolves too); for an `oo::configurable` receiver, colour
    //    `configure` / `cget` property options.
    if let Some(hierarchy) = classes
        && let Some(cls) = candidates
            .iter()
            .find(|c| user_class_provides_method(hierarchy, registry, c, method))
    {
        mark_method_word(seg, overrides);
        insert_user_configure_options(seg, hierarchy, cls, method, overrides);
    }
}

/// Highlight a dispatched object method's name word (`seg.argv[1]`) as a
/// [`TokenKind::Method`].
///
/// The *call site* of a method (`$obj add …`, `my Cleanup`, `[Class new] m …`)
/// is the same entity as its declaration, so it takes the same type — a method
/// is not a free procedure (#898 §2).  This is the one place a dispatched
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

/// The element classes of a collection-*retrieval* command head — a
/// single-level `[dict get $coll $key]` or `[lindex $coll $idx]` — looked up in
/// the object-collection map, or `None` when the head is not such a retrieval
/// or the collection is not tracked.  Resolves the receiver of the issue-#797
/// `[dict get $Pins $pin] configure -node …` dispatch.
///
/// Which calls retrieve an element, and from which argument, is registry data
/// ([`tcl_registry::types::ReturnElements::ElementOf`], read through
/// [`CommandRegistry::resolve_call`] exactly as the compiler's type inference
/// reads it) — so `::lindex` and `::dict get` resolve like their bare
/// spellings, and no command name is matched here (issue #1185).
fn collection_head_element_classes<'a>(
    head_text: &str,
    registry: &CommandRegistry,
    object_collections: &'a ObjectClassMap,
) -> Option<&'a std::collections::HashSet<String>> {
    use tcl_registry::types::ReturnElements;

    let (cmd, args) = tcl_compiler::value_shapes::parse_command_substitution(head_text)?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let resolved =
        registry.resolve_call(&cmd, &arg_refs, tcl_registry::dialects::DialectSet::empty())?;
    let ReturnElements::ElementOf { container_arg } = resolved.return_elements()? else {
        return None;
    };
    // The fact's indices are relative to after the subcommand word when one
    // matched (`dict get $d $k` counts from `$d`).
    let elem_args = if resolved.sub.is_some() {
        arg_refs.get(1..).unwrap_or(&[])
    } else {
        &arg_refs[..]
    };
    // Single-step retrieval only: exactly one index/key word after the
    // container — a multi-level `dict get $d a b` yields an inner dict, not an
    // element, so the fact does not apply.
    let container_idx = usize::from(container_arg);
    if elem_args.len() != container_idx + 2 {
        return None;
    }
    object_collections.get(object_handle_name(elem_args.get(container_idx)?)?)
}

/// Whether a *user-defined* class provides `method` for an instance dispatch:
/// the class hierarchy's MRO resolves it (a declared method on the class or an
/// ancestor), or it is an `TclOO` builtin every instance answers — `destroy`,
/// or `configure` / `cget` on an `oo::configurable` receiver.  `hierarchy` is
/// the local file's hierarchy or a workspace-merged project index.
fn user_class_provides_method(
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
    method: &str,
) -> bool {
    if hierarchy.method_target(class, method).is_some() {
        return true;
    }
    match method {
        "destroy" => true,
        "configure" | "cget" => class_is_configurable(hierarchy, registry, class),
        _ => false,
    }
}

/// Whether `class` (or any class in its MRO) is created by a metaclass whose
/// instances answer `configure` / `cget` against declared properties, or itself
/// declares such properties.
///
/// The metaclass test is registry data
/// ([`Traits::CONFIGURES_BY_PROPERTY`](tcl_registry::Traits::CONFIGURES_BY_PROPERTY)),
/// not the `metaclass == "oo::configurable"` spelling comparison this used to
/// make (issue #1275).  A metaclass the registry does not model answers
/// `false`: abstention, so an unknown factory is never treated as configurable.
/// tclsh 9.0.4: an `oo::configurable` instance answers `[$pt configure]` with
/// its property dict, an `oo::class` one answers `unknown method "configure"`.
fn class_is_configurable(
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
) -> bool {
    class_mro(hierarchy, class).iter().any(|c| {
        hierarchy.classes.get(c).is_some_and(|cd| {
            !cd.properties.is_empty()
                || registry.get(&cd.metaclass).is_some_and(|spec| {
                    spec.traits
                        .contains(tcl_registry::prelude::Traits::CONFIGURES_BY_PROPERTY)
                })
        })
    })
}

/// The MRO (self first) of `class` from `hierarchy`, or just `[class]` when the
/// hierarchy has no entry (an external / unindexed class).
fn class_mro(hierarchy: &ClassHierarchy, class: &str) -> Vec<String> {
    hierarchy
        .mro_map
        .get(class)
        .cloned()
        .unwrap_or_else(|| vec![class.to_string()])
}

/// Colour the `-property` options of a `configure` / `cget` dispatch on an
/// `oo::configurable` user class: a word `-<name>` whose `name` is a property
/// declared on the class or an ancestor becomes a [`ArgOverride::Decorator`],
/// and a following literal value an [`TokenKind::OptionValue`].  A non-property
/// `-word` is left to the generic option fallback.  No-op for any method other
/// than `configure` / `cget`.
fn insert_user_configure_options(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    hierarchy: &ClassHierarchy,
    class: &str,
    method: &str,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if method != "configure" && method != "cget" {
        return;
    }
    // Property names across the whole MRO (`-node`, `-name`, inherited …).
    let props: std::collections::HashSet<String> = class_mro(hierarchy, class)
        .iter()
        .filter_map(|c| hierarchy.classes.get(c))
        .flat_map(|cd| cd.properties.keys().cloned())
        .collect();
    if props.is_empty() {
        return;
    }
    for (i, text) in seg.texts.iter().enumerate().skip(2) {
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
        if let Some(val_tok) = seg.argv.get(i + 1)
            && matches!(val_tok.kind, TokenType::Esc | TokenType::Str)
            && seg
                .texts
                .get(i + 1)
                .is_some_and(|w| !w.starts_with('-') && w != "--")
        {
            overrides
                .entry(val_tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::OptionValue));
        }
    }
}

/// The bare handle name of a `$var` command head, matching the keys of an
/// [`ObjectClassMap`]: strips the leading `$` and any `${…}` braces, so
/// `$chart` → `chart`, `${chart}` → `chart`, `$arr(k)` → `arr(k)`.  Returns
/// `None` for a head that is not a plain variable substitution.
fn object_handle_name(head_text: &str) -> Option<&str> {
    let rest = head_text.strip_prefix('$')?;
    Some(
        rest.strip_prefix('{')
            .and_then(|r| r.strip_suffix('}'))
            .unwrap_or(rest),
    )
}

/// The registry class named by a direct manufacturer command-head dispatch,
/// or `None` when the head is not such a constructor call.
fn constructor_class_of_head<'r>(
    head_text: &str,
    registry: &'r CommandRegistry,
) -> Option<&'r str> {
    let (cmd, args) = tcl_compiler::value_shapes::parse_command_substitution(head_text)?;
    registry.exported_manufacturer_method(&cmd, args.first()?)?;
    registry.object_class(&cmd).map(|c| c.class_name)
}

/// The class named by an OO definition-body head (`oo::class create NAME { … }`
/// and the property-/instantiation-metaclasses at argv[2]; `oo::define NAME
/// { … }` at argv[1]), sliced from `source` so it outlives the walk.  `None`
/// when the head is not a body-bearing definer.
fn definer_class_name<'s>(
    head: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    source: &'s str,
    registry: &CommandRegistry,
) -> Option<&'s str> {
    let (name_idx, _) = definer_class_name_idx(head, seg, registry)?;
    let tok = seg.argv.get(name_idx)?;
    source.get(tok.span.start() as usize..tok.span.end() as usize)
}

/// The `argv` index of the class name at a definer head, and whether that
/// definer *declares* the class (`oo::class create Shape`, `snit::type Name`)
/// rather than merely referencing it (`oo::define Shape`).
///
/// Split out of [`definer_class_name`] so the token walk can type the name —
/// it was falling through to the default literal classification and painting as
/// a plain `string` in *both* 1.11.4 and 2.1.6 (#898 §2).
fn definer_class_name_idx(
    head: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<(usize, bool)> {
    let bare = head.strip_prefix("::").unwrap_or(head);
    let declares = bare != "oo::define";
    let name_idx = match bare {
        "oo::class" | "oo::configurable" | "oo::abstract" | "oo::singleton"
            if seg.texts.get(1).map(String::as_str) == Some("create") =>
        {
            2
        }
        "oo::define" if seg.texts.len() >= 3 => 1,
        // snit / itcl definers name the class directly at arg 1 and the body at
        // arg 2 (`snit::type Name { … }`, `itcl::class Name { … }`) — so a
        // `$self method …` / `$this method …` dispatch in the body resolves
        // against the class, exactly as `my` does for `TclOO`.  Driven by the
        // registry's definer-family grammar, not a hardcoded name list.
        _ if seg.texts.len() >= 3
            && matches!(
                registry
                    .get(head)
                    .and_then(|s| s.definition_body)
                    .map(|g| g.family),
                Some(DefinerFamily::Snit | DefinerFamily::Itcl)
            ) =>
        {
            1
        }
        _ => return None,
    };
    Some((name_idx, declares))
}

/// Mark the class name at a definer head so it emits as `Class` rather than a
/// bare literal (#898 §2).
fn insert_definer_class_name_override(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let head = &seg.texts[0];
    let Some((idx, declares)) = definer_class_name_idx(head, seg, registry) else {
        return;
    };
    let Some(tok) = seg.argv.get(idx) else {
        return;
    };
    overrides.entry(tok.span.start()).or_insert(if declares {
        ArgOverride::ClassNameDef
    } else {
        ArgOverride::ClassNameRef
    });
}

/// Resolve a class name *as written* at a definer head to a qualified key in
/// `hierarchy`: an exact match, its global-qualified form, or — as a last
/// resort — the unique class sharing its tail name.  `None` when unresolved or
/// the tail is ambiguous (no wrong-resolution from a homonym).
fn resolve_class_in_hierarchy(hierarchy: &ClassHierarchy, name: &str) -> Option<String> {
    // The shared call-site resolver (M4.2 dedup) — exact, canonical
    // global-qualified (#934 colon-run rule), then unique-tail.
    tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(name, &hierarchy.classes)
}

/// Resolve a self-call inside a class body against the enclosing class's MRO:
/// colour the method a callable, and — for `configure` / `cget` on an
/// `oo::configurable` class — its `-property` options.  The self-receiver is
/// `my` (`TclOO`), `[self]`/`[self object]` (`TclOO`, issue #1322), `$self`
/// (snit), or `$this` (itcl) — each of which dispatches on the enclosing
/// object.  No-op outside a class body, without a hierarchy, or for any
/// other head.
fn insert_self_method_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    classes: Option<&ClassHierarchy>,
    registry: &CommandRegistry,
    enclosing_class: Option<&str>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let (Some(hierarchy), Some(class_name), Some(head), Some(method)) = (
        classes,
        enclosing_class,
        seg.texts.first(),
        seg.texts.get(1),
    ) else {
        return;
    };
    // Three different axes, deliberately kept apart. `my` is the `TclOO`
    // self-dispatch *command keyword* — registry data, queried through
    // `method_dispatch_keyword` so a dialect that gains or loses it
    // propagates through its `CommandSpec` (issue #1050). `[self]`/`[self
    // object]` is a bracketed *command substitution* whose result is the
    // receiver, not a dispatch keyword — registry data via
    // `is_self_receiver_call`, keyed on the substitution's own head and
    // argument rather than matching `"self"` here (issue #1322). `$self` /
    // `$this` are snit / itcl *object-handle variable names*, a naming
    // convention of those class systems rather than a command at all, so
    // they stay matched by name here.
    let is_self_head = crate::definition::is_self_dispatch_keyword(head)
        || object_handle_name(head).is_some_and(|n| n == "self" || n == "this")
        || tcl_compiler::value_shapes::parse_command_substitution(head).is_some_and(
            |(cmd, args)| registry.is_self_receiver_call(&cmd, args.first().map(String::as_str)),
        );
    if !is_self_head {
        return;
    }
    let Some(class) = resolve_class_in_hierarchy(hierarchy, class_name) else {
        return;
    };
    if !user_class_provides_method(hierarchy, registry, &class, method) {
        return;
    }
    mark_method_word(seg, overrides);
    insert_user_configure_options(seg, hierarchy, &class, method, overrides);
}

/// The *user-defined* class named by a direct exported manufacturer head,
/// resolved against `hierarchy` (workspace-merged, so a class defined in
/// another file resolves).  Returns the qualified class name, or `None` when
/// the head is not a constructor call on a known class.  The constructor head
/// *is* the class command, so the class name is the head word itself — matched
/// as written and `::`-qualified.
fn user_constructor_class_of_head(
    head_text: &str,
    hierarchy: Option<&ClassHierarchy>,
    registry: &CommandRegistry,
) -> Option<String> {
    let hierarchy = hierarchy?;
    let (cmd, args) = tcl_compiler::value_shapes::parse_command_substitution(head_text)?;
    // This layer may know the user class but not the document that established
    // its metaclass.  Use the registry's conservative exported-manufacturer
    // union: ambiguity abstains, while a new family automatically widens the
    // accepted word set without a semantic-token edit.
    if !args
        .first()
        .is_some_and(|method| registry.is_manufacturer_method(method))
    {
        return None;
    }
    let qualified = format!("::{}", cmd.trim_start_matches("::"));
    [cmd.as_str(), qualified.as_str()]
        .into_iter()
        .find(|c| hierarchy.classes.contains_key(*c))
        .map(String::from)
}

/// Registry-known closed-set argument values → `EnumMember`.  The registry
/// records the legal value set for a positional argument as
/// [`CommandSpec::arg_values`] (keyed by 0-based index after the command
/// name) and, for ensemble subcommands, [`SubCommand::arg_values`] (keyed by
/// index after the subcommand word).  A literal word that matches one of the
/// declared values is highlighted as an enum member — so `string is alnum`,
/// `HTTP::respond 200 content`, or `when … timing enable` read as a fixed
/// keyword-like token rather than an arbitrary string.  Matching is against
/// the literal word text, so a `$var` / `[cmd]` at the same position is left
/// to the default classifier.
fn insert_enum_value_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    dialect: DialectSet,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let Some(spec) = registry.get(head) else {
        return;
    };
    // A closed-set value that is *also* a `Keyword`-role argument — e.g.
    // `control::do body while test`, whose `while`/`until` option is both a
    // declared value and the loop sense-word — is highlighted as a keyword by
    // `insert_role_overrides`, which is the more specific classification.  Skip
    // those command-level positions so the enum override does not claim the
    // token first (issue #760).
    let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
    let keyword_positions: rustc_hash::FxHashSet<usize> = registry
        .arg_indices_for_role(head, &arg_texts, tcl_registry::ArgRole::Keyword)
        .into_iter()
        .collect();
    let mut mark = |pos: usize, values: &[tcl_registry::hover::ArgValue]| {
        if let (Some(text), Some(tok)) = (seg.texts.get(pos), seg.argv.get(pos))
            && values.iter().any(|v| v.value == text.as_str())
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::EnumMember));
        }
    };

    // Command-level values: index is 0-based after the command name, so the
    // word sits at `seg.texts[idx + 1]`.
    for (idx, values) in spec.arg_values {
        if keyword_positions.contains(&(*idx as usize)) {
            continue;
        }
        mark(*idx as usize + 1, values);
    }

    // Subcommand-level values: index is 0-based after the subcommand word,
    // so add one more for the command name (`seg.texts[idx + 2]`).  Resolve
    // unique-prefix abbreviations like Tcl's ensemble dispatch.
    if let Some(sub_text) = seg.texts.get(1)
        && let Some(sub) = spec.resolve_subcommand_for_dialect(sub_text, dialect)
    {
        for (idx, values) in sub.arg_values {
            mark(*idx as usize + 2, values);
        }
    }

    // Option-value enum members — the value word(s) of a value-taking option
    // that declares an enumerable set (`-relief raised`, `-anchor center`).
    // Matched by name/alias, arity-aware via `value_indices`; a `$var`/`[cmd]`
    // value falls through `mark`'s literal check.
    let mut i = 1usize;
    while i < seg.texts.len() {
        let word = seg.texts[i].as_str();
        if word == "--" {
            break;
        }
        let opt = spec
            .options
            .iter()
            .chain(spec.command_forms.iter().flat_map(|f| f.options.iter()))
            .find(|o| o.matches(word));
        if let Some(opt) = opt {
            let vis = opt.value_indices(&seg.texts, i);
            let values = opt.value_values();
            if !values.is_empty() {
                for &vi in &vis {
                    mark(vi, values);
                }
            }
            i += 1 + vis.len();
            continue;
        }
        i += 1;
    }
}

/// `oo::define` / `oo::objdefine` inline definition keywords → `Keyword`.
///
/// In the *script* form (`oo::define Cls { method … }`) the definition words
/// are command heads inside the recursed body and are already highlighted by
/// [`emit_command_head`]'s grammar-member check.  The *inline* form
/// (`oo::define Cls method name args body`) puts the definition word at an
/// argument position, where it would otherwise render as a plain string.  The
/// target (class / object) sits at `seg.texts[1]`, so the definition keyword is
/// `seg.texts[2]`; `self` introduces a second, inner keyword at `seg.texts[3]`.
///
/// Which words are members comes from the definer command's own
/// `definition_body` grammar (`is_member`), not a hardcoded list — the same
/// source of truth the script form uses.  The *set of commands* that accept
/// inline member args (`oo::define` / `oo::objdefine`) is the outer-call shape,
/// which the member grammar does not model, so it stays an explicit guard
/// (`oo::class create Name …` puts a class name, not a member, at `texts[2]`).
fn insert_oo_define_keyword_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    dialect: DialectSet,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let Some(spec) = registry.get(seg.texts[0].as_str()) else {
        return;
    };
    // The *outer-call shape* — "argument 1 is the target, the member call
    // starts at argument 2" — is what separates a definer-**extension**
    // command from a definer that *creates* (`oo::class create Name ?body?`
    // puts a class name at that position, not a member keyword). The
    // registry already draws exactly that line with the `OoDefine` /
    // `OoObjdefine` analyser hooks, so this dispatches on them rather than
    // comparing spellings (issue #1185) — which also means the
    // explicitly-global `::oo::define` resolves like the bare form.
    if !matches!(
        spec.analyser_hook,
        Some(
            tcl_registry::hooks::AnalyserHookId::OoDefine
                | tcl_registry::hooks::AnalyserHookId::OoObjdefine
        )
    ) {
        return;
    }
    let Some(grammar) = spec.definition_body else {
        return;
    };
    let mut mark_keyword = |pos: usize| {
        if let Some(tok) = seg.argv.get(pos) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::Keyword));
        }
    };
    // The first definition word follows the class/object target
    // (`seg.texts[1]`), so it is `seg.texts[2]`.
    let Some(first) = seg.texts.get(2) else {
        return;
    };
    if !grammar.is_member(first) {
        return;
    }
    mark_keyword(2);
    // `self` introduces the real definition keyword (`method`, `constructor`,
    // …) at `seg.texts[3]`. This is the `oo::define` *definer-grammar*
    // wrapper word, not the `TclOO` `self` introspection command — a
    // different axis from `Traits::TCLOO_INTROSPECTION`, resolved through
    // the definer grammar's own `MemberKind::Wrapper` modelling.
    if first == "self" && seg.texts.get(3).is_some_and(|w| grammar.is_member(w)) {
        mark_keyword(3);
    }
    // The one-liner definer form carries a whole member call inline —
    // `oo::define C method m {a} {…}` / `oo::objdefine $obj method m {} {…}` —
    // so run the *same* member handling the body form gets, anchored at the
    // member keyword (argv 2).  Marking only the keyword left the method's name,
    // parameters and body untouched: the name painted as a plain string and the
    // body was never recursed.
    let member_args: Vec<&str> = seg.texts[3..].iter().map(String::as_str).collect();
    let first = first.clone();
    insert_oo_member_overrides(seg, grammar, &first, &member_args, 2, dialect, overrides);
}

/// `apply {params body ?ns?} …` — mark the braced lambda-literal argument
/// (`ArgRole::LambdaLiteral`) so its body (the second list element) is
/// re-segmented as a script.  Only a braced literal argument qualifies;
/// `apply $lambda …` (a variable) is left alone.  Matches C Tcl, where
/// `apply`'s first argument is a 2- or 3-element list `{argList body
/// ?namespace?}`.
///
/// Two shapes are recognised, both registry-driven — no command name is
/// compared anywhere in this function:
///
/// - **Direct**: `head` (this segmented command's own, already-resolved
///   head) carries `ArgRole::LambdaLiteral` at some argument index `K` — the
///   token at `argv[K + 1]` is the lambda literal.
/// - **List-quoted**: `head` instead carries `Traits::BUILDS_COMMAND_PREFIX`
///   (`list`) — the idiomatic way to build a deferred command around a
///   dynamic value, e.g. a pkgIndex.tcl entry capturing the install
///   directory: `package ifneeded name ver [list apply {dir {…}} $dir]`
///   (issue #954). `list`'s own first *argument*, if a literal bareword, is
///   resolved the same way any other command head is (registry `get`, which
///   strips a leading `::`); if that resolves to a `LambdaLiteral`-bearing
///   spec, the token at `argv[K + 2]` (shifted by one for `list` itself) is
///   the lambda literal. A dynamic `list` argument (`$var`, `[cmd]`) can't be
///   resolved statically and is left alone.
///
/// The list-quoted case is additionally gated on `deferred_role`: `list`
/// itself never invokes anything — it only ever returns a value — so
/// `[list apply {…} $x]` builds a real deferred invocation only when
/// *its own* enclosing argument slot is one that's later invoked/sourced
/// (`Body` / `LambdaLiteral` / `CommandPrefix`), e.g. `package ifneeded`'s
/// script argument. Plain data such as `set data [list apply {x {puts $x}}
/// value]` must not paint `x`/`puts`/`apply` as executable (codex review of
/// #954's follow-up) — `deferred_role` carries that enclosing-role check in
/// from [`collect_script`], computed once per `[…]` substitution. The direct
/// case needs no such gate: writing `apply {…}` literally *always* invokes
/// `apply` when reached, regardless of what its caller does with the result.
fn insert_lambda_literal_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    deferred_role: bool,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
    let mark = |idx: usize, overrides: &mut FxHashMap<u32, ArgOverride>| {
        if let Some(tok) = seg.argv.get(idx + 1)
            && matches!(tok.kind, TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::LambdaLiteral);
        }
    };

    let direct =
        registry.arg_indices_for_role(head, &arg_texts, tcl_registry::ArgRole::LambdaLiteral);
    if !direct.is_empty() {
        for idx in direct {
            mark(idx, overrides);
        }
        return;
    }

    if !deferred_role {
        return;
    }

    if !registry.get(head).is_some_and(|s| {
        s.traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
    }) {
        return;
    }
    // `list`'s own arg 0 must be a literal, unquoted, single-token bareword
    // to resolve statically — mirrors the bareword guard
    // `command_prefix::extract_prefix_head` uses for the same reason.
    let Some(inner_head_tok) = seg.argv.get(1) else {
        return;
    };
    if !matches!(inner_head_tok.kind, TokenType::Esc)
        || inner_head_tok.in_quote
        || seg.single_token_word.get(1) != Some(&true)
    {
        return;
    }
    let inner_head = seg.texts[1].as_str();
    let inner_arg_texts: Vec<&str> = seg.texts[2..].iter().map(String::as_str).collect();
    let inner_roles = registry.arg_indices_for_role(
        inner_head,
        &inner_arg_texts,
        tcl_registry::ArgRole::LambdaLiteral,
    );
    if inner_roles.is_empty() {
        return;
    }
    for idx in inner_roles {
        if let Some(tok) = seg.argv.get(idx + 2)
            && matches!(tok.kind, TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::LambdaLiteral);
        }
    }
    // The resolved command-name word itself (`apply` / `::apply`) is a real
    // call-site reference, same as a `CommandPrefix` bareword head — paint it
    // `Function` rather than leaving it to fall through to a plain string /
    // namespace-word guess.
    overrides
        .entry(inner_head_tok.span.start())
        .or_insert(ArgOverride::CommandRef);
}

/// Variable names a command declares / writes (`ArgRole::VarWrite`) →
/// `Variable` + `declaration`.  The registry marks the write target of `set`
/// / `incr` / `append` / `lappend` / `lassign` / `global` / `variable` / … ,
/// which the query [`CommandRegistry::arg_indices_for_role`] resolves
/// (including subcommand and dynamic-resolver commands such as `dict set`).
/// The argument is *known* to be a variable-name spot — not from the word's
/// text, but from a declared role: the static registry `ArgRole::VarWrite` /
/// `ArgRole::VarRead`, a `# tcl-lsp: stub … :var` / `:var_read` declaration, or
/// a user-proc parameter the analyser inferred to alias a caller variable
/// (`extra_var_write` / `extra_var_read`).  A **written** target retags as a
/// `Variable` declaration; a **read** reference as a plain `Variable` (it names
/// an existing variable, not a new one).  The only remaining question is token
/// geometry.  A word that lexes as a single unquoted [`TokenType::Esc`] token —
/// a scalar (`x`), a literal array element (`arr(key)`), or a namespaced name
/// (`::ns::arr(key)`) — is retagged as one whole-word token, matching how the
/// `$arr(key)` read highlights (issue #813).  A word with an inner substitution
/// (`arr($i)`, `$dynamic`) is multi-token (`single_token_word` is `false`), so
/// it is left to the default classifier and its inner `$var` sub-tokens survive.
fn insert_var_role_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    extra_var_write: &FxHashMap<String, Vec<u32>>,
    extra_var_read: &FxHashMap<String, Vec<u32>>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
    // `i` is 0-based after the command name → word at index `i + 1`.  A word is
    // retagged only when it is a single unquoted `Esc` token — the geometry that
    // makes it safe to paint whole (scalars, literal array elements, namespaced
    // names), while a substitution-bearing word stays multi-token.
    let mut retag = |i: usize, ov: ArgOverride| {
        let Some(word) = seg.argv.get(i + 1) else {
            return;
        };
        if seg.single_token_word.get(i + 1) == Some(&true) {
            // `Str` — a brace-quoted word — is a variable *name* here just as
            // much as a bareword is: braces suppress every substitution, so
            // `set {$n} 1` declares the variable literally called `$n` and
            // `[set {$n}]` reads it (tclsh 9.0.4 / 8.6.14: `info exists {$n}`
            // → 1 while `info exists n` → 0).  Falling through painted the
            // word as a plain `string`, hiding a declaration and inviting the
            // reader to see the `$n` inside as a substitution (issue #1078).
            // It is the quoting that makes such a name writable at all, so
            // this is the *only* spelling those variables ever have.
            if matches!(word.kind, TokenType::Esc | TokenType::Str) && !word.in_quote {
                overrides.entry(word.span.start()).or_insert(ov);
            }
            return;
        }
        // A multi-token word in a variable-name position is an **array element
        // whose index is a substitution** — `set env($lo)`, `unset
        // UnknownPending($name)`, `set auto_index([foo])`.  A literal index
        // (`env(PATH)`) is a single token and took the branch above; this one
        // stays multi-token, and used to be skipped entirely, so its literal
        // fragments fell through to the default classification and painted as
        // `string` (#898 §3) — pervasive in Tcl's own `init.tcl` / `package.tcl`.
        //
        // The representative `argv` token spans the whole word (segmenter:
        // `multi_token_word_argv_spans_full_word`), so paint every *literal*
        // fragment of it — the array name and the parens — as the variable; the
        // `$index` / `[cmd]` tokens inside classify themselves.
        let text = seg.texts.get(i + 1).map_or("", String::as_str);
        if word.in_quote || !text.contains('(') || !text.ends_with(')') {
            return;
        }
        for t in &seg.all_tokens {
            if matches!(t.kind, TokenType::Esc)
                && t.span.start() >= word.span.start()
                && t.span.end() <= word.span.end()
            {
                overrides.entry(t.span.start()).or_insert(ov);
            }
        }
    };
    // Writes first: a declaration wins over a read reference at the same
    // position (`dict with`'s arg 0 carries both roles).
    for i in registry.arg_indices_for_role(head, &arg_texts, tcl_registry::ArgRole::VarWrite) {
        retag(i, ArgOverride::VarDecl);
    }
    if let Some(indices) = extra_var_write.get(head) {
        for &i in indices {
            retag(i as usize, ArgOverride::VarDecl);
        }
    }
    for i in registry.arg_indices_for_role(head, &arg_texts, tcl_registry::ArgRole::VarRead) {
        retag(i, ArgOverride::VarRef);
    }
    if let Some(indices) = extra_var_read.get(head) {
        for &i in indices {
            retag(i as usize, ArgOverride::VarRef);
        }
    }
}

/// A command name passed as an argument — the registry `CommandPrefix` role
/// (`tk selection … -command`, a stub `:command_prefix`), or a proc parameter
/// the analyser inferred to be a `Command` (`extra_command`: `$cmd` used as a
/// head, or flowing into a command-name position).  Retag the literal at the
/// call site as a `Function`, gated by the same single-token `Esc` geometry as
/// the variable retag, so `dispatch mycmd …` paints `mycmd` as a command.
fn insert_command_role_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    extra_command: &FxHashMap<String, Vec<u32>>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
    let mut retag = |i: usize| {
        if let Some(tok) = seg.argv.get(i + 1)
            && seg.single_token_word.get(i + 1) == Some(&true)
            && matches!(tok.kind, TokenType::Esc)
            && !tok.in_quote
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::CommandRef);
        }
    };
    for i in registry.arg_indices_for_role(head, &arg_texts, tcl_registry::ArgRole::CommandPrefix) {
        retag(i);
    }
    // A bare command name held as data (`info body PROC`, `namespace origin
    // NAME`) is a command reference too, so paint it as a `Function`.
    for i in registry.arg_indices_for_role(head, &arg_texts, tcl_registry::ArgRole::CommandName) {
        retag(i);
    }
    if let Some(indices) = extra_command.get(head) {
        for &i in indices {
            retag(i as usize);
        }
    }
}

/// `true` when `text` is a plain (non-array, non-substituted) variable name
/// — the safe case to retag as a whole-word `Variable` declaration token.
fn is_plain_var_name(text: &str) -> bool {
    // Excludes array elements (`arr(x)`), substitutions (`$`/`[`), quoted /
    // braced words, and the stray `}` / `)` the degenerate empty-brace (`{}`)
    // span clamp can leave in sub-tokenised list content.
    !text.is_empty() && !text.contains(['(', ')', '$', '[', ']', '{', '}', '"', ' '])
}

/// `switch … { pat body … }` — the braced case list (the final word, when
/// option-skipped past the mode flags / `--`) holds all the pattern/body
/// pairs.  Tag it so `collect_script` pairs the elements and recurses each
/// body as a script, rather than walking the whole list as one opaque body
/// (which would leave the bodies unhighlighted).  `-regexp` mode additionally
/// sub-tokenises the patterns as regexes.
fn insert_case_list_override(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // The clause-list shape is registry data (`CommandSpec::case_list`), so this
    // walker names no command: `switch … {pat body …}` and Expect's
    // `expect {?-flags? pat body …}` are the same construct, and Expect's
    // `expect_before` / `expect_after` / … come along for free.  Previously this
    // was `if seg.texts[0] != "switch" { return }` — the hardcode AGENTS.md
    // calls migration debt — and Expect's clause bodies were never recursed,
    // so an entire `expect {…}` block rendered as flat per-line `string` tokens.
    let Some(spec) = registry.get(&seg.texts[0]).and_then(|s| s.case_list) else {
        return;
    };
    // Where the list sits (and whether the command-level regex option was
    // given) is `tcl_syntax`'s one implementation, shared with the reference
    // scanner and the fold walk — three walkers that must agree about where an
    // arm's body is or they disagree about what the code says.  The braced-list
    // form only: the inline `pat body …` form leaves more than one trailing
    // word and `clause_list_call` answers `None` for it.
    let args: Vec<&str> = seg.texts.iter().skip(1).map(String::as_str).collect();
    let dialect = registry
        .profile()
        .map_or_else(tcl_dialect::DialectSet::empty, |profile| {
            profile.availability_mask
        });
    let Some((_, invocation)) = registry.case_invocation(&seg.texts[0], &args, dialect) else {
        return;
    };
    let Some(index) = invocation.clause_list_index else {
        return;
    };
    // `args` is 0-based post-command-name; `seg.texts` / `seg.argv` are 1-based.
    if let Some(tok) = seg.argv.get(index + 1)
        && matches!(tok.kind, TokenType::Str)
    {
        overrides.insert(
            tok.span.start(),
            ArgOverride::CaseList(
                spec,
                invocation.mode == tcl_registry::spec::CaseMatchMode::Regexp,
            ),
        );
    }
}

/// Registry-driven role overrides: body / expr braced arguments (recursed
/// into rather than emitted opaque) and structural keyword words.  Added
/// last with `or_insert` so the more specific regex/format overrides win.
fn insert_role_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    arg_texts: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // `if {expr} {body}`, `proc n a {body}`, `while {expr} {body}`,
    // `expr {expr}`, … — keyed on each word's representative token
    // (`argv[i + 1]`; `argv[0]` is the head).  Only braced (`Str`) words
    // recurse; non-literal words fall through.
    for (role, ov) in [
        (tcl_registry::ArgRole::Body, ArgOverride::BodyScript),
        (tcl_registry::ArgRole::Expr, ArgOverride::ExprScript),
    ] {
        for i in registry.arg_indices_for_role(head, arg_texts, role) {
            if let Some(tok) = seg.argv.get(i + 1)
                && matches!(tok.kind, TokenType::Str)
            {
                overrides.entry(tok.span.start()).or_insert(ov);
            }
        }
    }

    // Structural keyword words (`if`'s then/elseif/else, `try`'s
    // on/trap/finally) sit at argument positions, not the command-name
    // slot, so the default classifier would render them as strings.  The
    // registry's `Keyword` role marks them; highlight as keywords.  Unlike
    // body/expr these are bare (`Esc`) or quoted (`Str`) literal words, so
    // no `Str`-only guard.
    for i in registry.arg_indices_for_role(head, arg_texts, tcl_registry::ArgRole::Keyword) {
        if let Some(tok) = seg.argv.get(i + 1)
            && matches!(tok.kind, TokenType::Esc | TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::KeywordArg);
        }
    }
    // Note: recursing the body of a `method` / `constructor` / … keyword used
    // as a command head inside a class-definition script is handled
    // context-sensitively by `insert_oo_body_overrides` (issue #747), which
    // only fires inside an actual OO definition body — so a same-named user
    // proc is never misclassified.
}

/// Sub-tokenise a `binary format`/`scan` field string into its
/// specifiers: digit runs → `BinaryCount`, specifier letters →
/// `BinarySpec`, a `u`/`s` modifier after an integer specifier (Tcl 8.5+)
/// or a trailing `*` → `BinaryFlag`.  Whitespace and unrecognised
/// characters are skipped.  Returns `false` when nothing was emitted.
fn push_binary_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    dialect: &str,
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
    let bytes = inner.as_bytes();
    let allow_mod = !matches!(dialect, "tcl8.4" | "f5");
    let mut i = 0;
    let mut emitted = false;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // Digit run → count.
        let count_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > count_start {
            push_subtoken(
                source,
                line_index,
                cstart + count_start,
                &inner[count_start..i],
                TokenKind::BinaryCount,
                entries,
            );
            emitted = true;
        }
        if i >= bytes.len() {
            break;
        }
        let spec = bytes[i];
        if !BINARY_FORMAT_SPECIFIERS.contains(&spec) {
            i += 1;
            continue;
        }
        push_subtoken(
            source,
            line_index,
            cstart + i,
            &inner[i..=i],
            TokenKind::BinarySpec,
            entries,
        );
        emitted = true;
        i += 1;
        // Signed/unsigned modifier (Tcl 8.5+) after an integer specifier.
        if i < bytes.len()
            && matches!(bytes[i], b'u' | b's')
            && BINARY_INT_SPECIFIERS.contains(&spec)
            && allow_mod
        {
            push_subtoken(
                source,
                line_index,
                cstart + i,
                &inner[i..=i],
                TokenKind::BinaryFlag,
                entries,
            );
            emitted = true;
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'*' {
            push_subtoken(
                source,
                line_index,
                cstart + i,
                "*",
                TokenKind::BinaryFlag,
                entries,
            );
            emitted = true;
            i += 1;
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
    let bytes = inner.as_bytes();
    let pos = TokenPositionContext { source, line_index };
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        // `%(?:[EO])?<spec>` — an optional `E`/`O` locale modifier only
        // counts when it precedes a spec letter (else the `E`/`O` is
        // itself the spec, as both are in the spec set).
        if bytes[i] == b'%' {
            let mut spec = i + 1;
            let modifier = (matches!(bytes.get(spec), Some(b'E' | b'O'))
                && bytes.get(spec + 1).copied().is_some_and(is_clock_spec))
            .then(|| {
                let m = spec;
                spec += 1;
                m
            });
            if bytes.get(spec).copied().is_some_and(is_clock_spec) {
                flush_run(pos, cstart, inner, run, i, TokenKind::String, entries);
                push_subtoken(
                    source,
                    line_index,
                    cstart + i,
                    "%",
                    TokenKind::ClockPercent,
                    entries,
                );
                if let Some(m) = modifier {
                    push_subtoken(
                        source,
                        line_index,
                        cstart + m,
                        &inner[m..=m],
                        TokenKind::ClockModifier,
                        entries,
                    );
                }
                push_subtoken(
                    source,
                    line_index,
                    cstart + spec,
                    &inner[spec..=spec],
                    TokenKind::ClockSpec,
                    entries,
                );
                emitted = true;
                i = spec + 1;
                run = i;
                continue;
            }
        }
        i += 1;
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

/// `clock format`/`scan` specifier letters (and `%`).
fn is_clock_spec(b: u8) -> bool {
    matches!(
        b,
        b'a' | b'A'
            | b'b'
            | b'B'
            | b'c'
            | b'C'
            | b'd'
            | b'D'
            | b'e'
            | b'E'
            | b'g'
            | b'G'
            | b'h'
            | b'H'
            | b'I'
            | b'j'
            | b'J'
            | b'k'
            | b'l'
            | b'm'
            | b'M'
            | b'N'
            | b'O'
            | b'p'
            | b'P'
            | b'q'
            | b'Q'
            | b's'
            | b'S'
            | b'u'
            | b'U'
            | b'V'
            | b'w'
            | b'W'
            | b'x'
            | b'X'
            | b'y'
            | b'Y'
            | b'z'
            | b'Z'
            | b'%'
    )
}

/// Sub-tokenise a `format`/`scan` conversion string into its `%`
/// specifier components (`FormatPercent` / `FormatFlag` / `FormatWidth`
/// / `FormatSpec`), with literal runs classified as `string`.  Returns
/// `false` (emitting nothing) when there are no `%` specifiers.
fn push_sprintf_subtokens(
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
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(cuts) = parse_sprintf_cuts(bytes, i)
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

/// `format`/`scan` conversion type letters.
fn is_sprintf_type(b: u8) -> bool {
    matches!(
        b,
        b'a' | b'A'
            | b'b'
            | b'B'
            | b'c'
            | b'd'
            | b'i'
            | b'e'
            | b'E'
            | b'f'
            | b'g'
            | b'G'
            | b'o'
            | b's'
            | b'u'
            | b'x'
            | b'X'
            | b'%'
    )
}

/// Parse one `%`-specifier at `b[start]` into its component
/// `(end, kind)` cuts (monotonic ends, consumed in order by
/// [`emit_part`]), or `None` when it isn't a valid conversion (no type
/// letter — the `%` is then a literal).
fn parse_sprintf_cuts(b: &[u8], start: usize) -> Option<Vec<(usize, TokenKind)>> {
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
    if j < n && is_sprintf_type(b[j]) {
        cuts.push((j + 1, TokenKind::FormatSpec));
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
        } else if matches!(ch, b'b' | b'B' | b'm' | b'M' | b'y' | b'Y' | b'A' | b'Z') {
            TokenKind::RegexpAnchor
        } else {
            TokenKind::RegexpEscape
        };
    }
    match matched {
        "^" | "$" => TokenKind::RegexpAnchor,
        "|" => TokenKind::RegexpAlternation,
        "." => TokenKind::RegexpCharClass,
        // A group's *closer* is as much a group delimiter as its opener — it
        // used to fall through to the quantifier catch-all below and paint
        // every `)` in the quantifier colour (#898 §5).
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
/// (`{…}`) or quoted (`"…"`) strings that span lines (issue #757) — highlighted
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
#[derive(Clone, Copy)]
struct ScriptCtx<'a> {
    full_source: &'a str,
    dialect: &'a str,
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
    /// The document's statically proven command-identity facts — which
    /// registry command each head spelling really names at each point in the
    /// file.  Folds together `namespace import` (`test` → `tcltest::test`,
    /// issue #776), `interp alias`, static `rename`, and a top-level `proc`
    /// that shadows a built-in (issue #1185).  Every fact is offset-keyed, so
    /// a binding cannot retroactively re-tag an earlier call; every shape that
    /// cannot be proven leaves the head alone.  Empty for a document that
    /// binds nothing.  See [`tcl_compiler::head_identity`].
    head_identities: &'a tcl_compiler::head_identity::HeadIdentityMap,
    /// Object-handle → class-name provenance for the whole document, so a
    /// `$var method …` dispatch can resolve the method's options through the
    /// registry's object-class model (issue #748).  Empty when no
    /// [`CompilationUnit`] is available or the document creates no tracked
    /// object handles.
    object_classes: &'a ObjectClassMap,
    /// Object-*collection* variable → element class, so a `[dict get $coll $k]`
    /// / `[lindex $coll $i]` retrieval used as a command head resolves the
    /// element's method (issue #797).  Empty without a [`CompilationUnit`].
    object_collections: &'a ObjectClassMap,
    /// Class hierarchy, when available — the MRO + `ClassDef`s (methods +
    /// `oo::configurable` properties) used to resolve a dispatched method
    /// against a *user* class, not just a registry-modelled one.  This is the
    /// current file's hierarchy, or a workspace-merged project index so a class
    /// defined in another file resolves too (issue #797 follow-up).  `None` for
    /// the pure-segmentation path.
    classes: Option<&'a ClassHierarchy>,
    /// The class whose definition body we are currently inside (as written at
    /// the `oo::class create NAME` / `oo::define NAME` head), sliced from the
    /// source so it lives as long as the walk.  Lets a `my method …` self-call
    /// in a method body resolve against the enclosing class's MRO — the single
    /// most common `TclOO` dispatch form.  `None` outside any class body.
    enclosing_class: Option<&'a str>,
    /// Extra variable-name (`ArgRole::VarWrite`) argument positions the static
    /// registry doesn't model, keyed by command / proc name: source-derived
    /// `# tcl-lsp: stub … :var` roles unioned with the analyser's inferred
    /// user-proc parameter roles.  The `VarWrite` retag reads this alongside
    /// the registry so a `myproc arr(key) …` call highlights its array-element
    /// target (issue #813 follow-up).  Empty on the pure-segmentation path.
    extra_var_write: &'a FxHashMap<String, Vec<u32>>,
    /// Extra variable-name (`ArgRole::VarRead`) argument positions the static
    /// registry doesn't model — the read-side counterpart of `extra_var_write`
    /// (stub `:var_read` roles and inferred user-proc `VarRead` params).  These
    /// retag as a plain `Variable` (no `declaration` modifier), since a read
    /// references an existing variable.  Empty on the pure-segmentation path.
    extra_var_read: &'a FxHashMap<String, Vec<u32>>,
    /// Extra command-name (`ArgRole::CommandPrefix` / inferred
    /// `ProcArgTrait::Command`) argument positions, keyed by command / proc
    /// name: stub `:command_prefix` roles unioned with the analyser's inferred
    /// user-proc `Command` params.  These retag as a `Function` so a literal
    /// command name passed to a dispatcher highlights as a command.  Empty on
    /// the pure-segmentation path.
    extra_command: &'a FxHashMap<String, Vec<u32>>,
}

/// Emit one clause-list *pattern* element.
///
/// A keyword pattern (`default`; Expect's `timeout` / `eof` / `full_buffer`)
/// matches no text — it is a keyword, never a regex and never a string.
/// Otherwise a regex-mode pattern is sub-tokenised as a regex, and an
/// exact/glob one is classified as an ordinary literal.
fn push_case_pattern(
    line_index: &LineIndex,
    full_source: &str,
    pat_tok: Token,
    text: &str,
    spec: &'static tcl_registry::CaseListSpec,
    regexp: bool,
    entries: &mut Vec<Entry>,
) {
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
    } else if let Some(kind) = classify_arg_token(pat_tok, full_source) {
        push_token(line_index, full_source, pat_tok, kind, 0, entries);
    }
}

fn collect_case_list(
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
    depth: u32,
    spec: &'static tcl_registry::CaseListSpec,
    regexp: bool,
) {
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
                u32::try_from(cstart + e.start).unwrap_or(0),
                u32::try_from(cstart + e.end).unwrap_or(0),
            ),
            content_offset,
        )
    };

    for clause in tcl_syntax::case_list::split_case_list(inner, &shape) {
        let mut clause_regexp = regexp;
        for f in &clause.flags {
            let text = inner.get(f.start..f.end).unwrap_or_default();
            if spec.clause_regex_flag == Some(text) {
                clause_regexp = true;
            }
            let ftok = as_token(*f);
            // A flag word is a decorator; its *value* word (`-timeout 5`) takes
            // its own literal classification.
            let kind = if spec.clause_flags.contains(&text) {
                Some(TokenKind::Decorator)
            } else {
                classify_arg_token(ftok, full_source)
            };
            if let Some(kind) = kind {
                push_token(line_index, full_source, ftok, kind, 0, entries);
            }
        }

        if let Some(p) = clause.pattern {
            let text = inner.get(p.start..p.end).unwrap_or_default();
            push_case_pattern(
                line_index,
                full_source,
                as_token(p),
                text.trim_start_matches('{'),
                spec,
                clause_regexp,
                entries,
            );
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
            } else if let Some(kind) = classify_arg_token(btok, full_source) {
                push_token(line_index, full_source, btok, kind, 0, entries);
            }
        }
    }
}

/// Recurse the body of an `ArgRole::LambdaLiteral` `{params body ?ns?}`
/// lambda literal (`apply`'s shape, reached directly or list-quoted — see
/// [`insert_lambda_literal_overrides`]).
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
/// `None` reset for the same recursion, and the same fresh-frame reasoning
/// as the interprocedural/param-trait/declaration fixes for issue #954's
/// follow-up).
fn collect_lambda_literal(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>, depth: u32) {
    if MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        // Not a braced literal (should not happen — the override only fires
        // for `Str` tokens); fall back to a plain classification.
        if let Some(kind) = classify_arg_token(tok, full_source) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let lambda_ctx = ScriptCtx {
        oo_grammar: None,
        enclosing_class: None,
        scoped_env: None,
        ..ctx
    };
    // Flatten the lambda's list elements (params, body, ?ns?).
    let mut words: Vec<Token> = Vec::new();
    for seg in segment_commands_with_offset_and_config(
        inner,
        u32::try_from(cstart).unwrap_or(0),
        tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
    ) {
        words.extend(seg.argv.iter().copied());
    }
    for (idx, word_tok) in words.iter().enumerate() {
        if idx == 0 {
            // Element 0 is the parameter list — a braced `{a b}` list or a
            // bare single name (`apply {dir {…}}`); `collect_param_list`
            // emits its names as declarations either way, and leaves a
            // computed (`$dynamic`) list to the default classifier.
            collect_param_list(ctx, *word_tok, entries);
        } else if idx == 1
            && let Some((bstart, body)) = subspec_content(full_source, *word_tok)
        {
            // Element 1 is the body — recurse it as a script when braced.
            collect_script(
                lambda_ctx,
                body,
                u32::try_from(bstart).unwrap_or(0),
                entries,
                depth + 1,
                false,
            );
        } else if let Some(kind) = classify_arg_token(*word_tok, full_source) {
            push_token(line_index, full_source, *word_tok, kind, 0, entries);
        }
    }
}

/// Emit the name(s) of a `foreach` / `lmap` / `dict for` variable spec as
/// variable declarations.  A single bareword is one name; a braced/quoted list
/// is flattened via the list grammar and each element name emitted separately.
/// A non-name element (a `$`-computed word, an array element) keeps a plain
/// `string` classification so nothing is dropped and it does not masquerade as
/// a variable.
fn collect_loop_var_list(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        if let Some(kind) = classify_arg_token(tok, full_source) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let mut scan = 0usize;
    while let Ok(Some(el)) = tcl_syntax::list::find_element(inner, scan) {
        if let Some(name) = inner.get(el.value.clone()) {
            let (kind, mods) = if is_plain_var_name(name) {
                (TokenKind::Variable, MOD_DECLARATION)
            } else {
                (TokenKind::String, 0)
            };
            push_span_entries(
                full_source,
                line_index,
                cstart + el.value.start,
                name,
                kind,
                mods,
                entries,
            );
        }
        if el.next <= scan {
            break;
        }
        scan = el.next;
    }
}

/// Emit a procedure parameter list's names as variable declarations.  Each
/// top-level list element is either a bareword parameter name or a `{name
/// ?default...?}` pair; the name is a `Variable` declaration and any default
/// words are classified (number / string).  A non-name element is left to the
/// default classifier.
fn collect_param_list(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        // A bare (unbraced) argument list is a single-element list naming one
        // parameter (`apply {dir {…}}`, `proc p x {…}`).  When it is a plain
        // name, emit it as a `Parameter` declaration — matching the braced
        // path — rather than letting it fall through to `string`.  A computed
        // arg list (`$dynamic`) is not a plain name and keeps its default
        // classification.
        if full_source
            .get(tok.span.start() as usize..tok.span.end() as usize)
            .is_some_and(is_plain_var_name)
        {
            push_token(
                line_index,
                full_source,
                tok,
                TokenKind::Parameter,
                MOD_DECLARATION,
                entries,
            );
        } else if let Some(kind) = classify_arg_token(tok, full_source) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let mut scan = 0usize;
    while let Ok(Some(el)) = tcl_syntax::list::find_element(inner, scan) {
        let braced = el.value.start > 0 && inner.as_bytes().get(el.value.start - 1) == Some(&b'{');
        if let Some(elem) = inner.get(el.value.clone()) {
            let elem_abs = cstart + el.value.start;
            if braced {
                // `{name ?default...?}` — the first word is the parameter name.
                emit_param_default_pair(full_source, line_index, elem_abs, elem, entries);
            } else if is_plain_var_name(elem) {
                push_span_entries(
                    full_source,
                    line_index,
                    elem_abs,
                    elem,
                    TokenKind::Parameter,
                    MOD_DECLARATION,
                    entries,
                );
            }
        }
        if el.next <= scan {
            break;
        }
        scan = el.next;
    }
}

/// Emit the name + default words of a `{name ?default...?}` parameter pair:
/// the leading word as a `Parameter` declaration, each following word by its
/// literal classification (number / string).
fn emit_param_default_pair(
    source: &str,
    line_index: &LineIndex,
    abs: usize,
    text: &str,
    entries: &mut Vec<Entry>,
) {
    let mut scan = 0usize;
    let mut first = true;
    while let Ok(Some(el)) = tcl_syntax::list::find_element(text, scan) {
        if let Some(word) = text.get(el.value.clone()) {
            let word_abs = abs + el.value.start;
            let (kind, mods) = if first {
                (TokenKind::Parameter, MOD_DECLARATION)
            } else if is_number_literal(word) {
                (TokenKind::Number, 0)
            } else {
                (TokenKind::String, 0)
            };
            if first && !is_plain_var_name(word) {
                // Not a plain name — leave it (and the rest) to the default.
            } else {
                push_span_entries(source, line_index, word_abs, word, kind, mods, entries);
            }
            first = false;
        }
        if el.next <= scan {
            break;
        }
        scan = el.next;
    }
}

/// The lexical context a command head is classified in: the enclosing
/// definition-body grammar and scoped command environment (both `None` at top
/// level).  Bundled so [`emit_command_head`] keeps a small signature.
#[derive(Clone, Copy)]
struct HeadContext {
    oo_grammar: Option<&'static DefinitionBodyGrammar>,
    scoped_env: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
}

/// The command head being classified: its token, the source text, and the
/// head's *effective command identity* — the registry name the spelling really
/// resolves to at this point in the document (an imported bare name resolves
/// to its qualified spec; a static `interp alias` / `rename` resolves to its
/// target; a shadowed built-in resolves to nothing).  Bundled so
/// [`emit_command_head`] keeps a small signature.
#[derive(Clone, Copy)]
struct CommandHead<'a> {
    tok: Token,
    text: &'a str,
    /// The registry name to resolve grammar against — empty when the head was
    /// rebound (see [`tcl_compiler::head_identity::HeadIdentity::spec_name`]).
    resolved: &'a str,
    /// Whether the head's registry binding was provably taken over by a
    /// `rename` / alias / shadowing `proc` (issue #1185).
    rebound: bool,
}

fn emit_command_head(
    line_index: &LineIndex,
    full_source: &str,
    head: CommandHead<'_>,
    head_ctx: HeadContext,
    registry: &CommandRegistry,
    entries: &mut Vec<Entry>,
) {
    let CommandHead {
        tok: head_tok,
        text: head_text,
        resolved: resolved_head,
        rebound,
    } = head;
    let HeadContext {
        oo_grammar,
        scoped_env,
    } = head_ctx;
    // A member sub-keyword of the enclosing definition body (`method`,
    // `typemethod`, `constructor`, …) is a keyword — context-sensitively, so a
    // same-named user proc outside a definition body is unaffected.  This
    // covers the snit-specific members (`typemethod`, `typeconstructor`,
    // `onconfigure`, …) that [`is_language_keyword_sub_keyword`] does not cover.
    if !head_text.contains("::")
        && oo_grammar.is_some_and(|g| crate::oo_body::is_member(g, head_text))
    {
        push_token(
            line_index,
            full_source,
            head_tok,
            TokenKind::Keyword,
            0,
            entries,
        );
        return;
    }
    // A command of the enclosing scoped environment (`top`, `data`, `columns`
    // inside a `report::defstyle` style script) highlights as a library
    // function — context-sensitively, so a same-named command outside the scope
    // is unaffected.  Registry data drives it (no command name here).
    if !head_text.contains("::") && scoped_env.is_some_and(|e| e.is_command(head_text)) {
        push_token(
            line_index,
            full_source,
            head_tok,
            TokenKind::Function,
            MOD_DEFAULT_LIBRARY,
            entries,
        );
        return;
    }
    let full_kind = classify_command_head(head, registry);
    // Split any `…::name` head (namespace-qualified command or keyword) into a
    // namespace prefix + final-segment command token.
    if head_text.contains("::")
        && let Some(idx) = head_text.rfind("::")
    {
        // Byte length of the `…::` prefix (head_text bytes == span bytes).
        let prefix_len = u32::try_from(idx + 2).unwrap_or(0);
        let start = head_tok.span.start();
        // Namespace prefix token.  It carries `defaultLibrary` when the command
        // it qualifies is a registry built-in (`tcl::mathop::+`, `tcl::tm::path`)
        // — the prefix is as much part of the built-in's name as the tail, which
        // already gets the modifier below, and a theme that dims stdlib names was
        // dimming only half of one (#898 §11).
        // Resolved, not written: a `rename`d-away built-in must lose the
        // modifier and a proven alias of one must gain it (issue #1185).
        let builtin_mods = if registry.get(resolved_head).is_some() {
            MOD_DEFAULT_LIBRARY
        } else {
            0
        };
        push_token(
            line_index,
            full_source,
            Token {
                span: tcl_lexer::Span::new(start, start + prefix_len),
                ..head_tok
            },
            TokenKind::Namespace,
            builtin_mods,
            entries,
        );
        // Final-segment command token: keyword when the full name is a
        // language keyword (TclOO `oo::class` etc.), else function;
        // `defaultLibrary` when the full name is a registry built-in.
        let tail = &head_text[idx + 2..];
        let is_keyword = !rebound
            && (registry.get(resolved_head).is_some_and(|s| {
                s.traits
                    .contains(tcl_registry::prelude::Traits::LANGUAGE_KEYWORD)
            }) || is_language_keyword_sub_keyword(tail));
        let kind = if is_keyword {
            TokenKind::Keyword
        } else {
            TokenKind::Function
        };
        let mods = if kind == TokenKind::Function && registry.get(resolved_head).is_some() {
            MOD_DEFAULT_LIBRARY
        } else {
            0
        };
        push_token(
            line_index,
            full_source,
            Token {
                span: tcl_lexer::Span::new(start + prefix_len, head_tok.span.end()),
                ..head_tok
            },
            kind,
            mods,
            entries,
        );
        return;
    }
    // Use the resolved head for the built-in lookup: a bare name imported from
    // an exported namespace (`namespace import tcltest::*` → `test`) resolves
    // to its qualified registry spec, so it carries `defaultLibrary` too.
    let mods = if full_kind == TokenKind::Function && registry.get(resolved_head).is_some() {
        MOD_DEFAULT_LIBRARY
    } else {
        0
    };
    push_token(line_index, full_source, head_tok, full_kind, mods, entries);
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
/// computed. It gates [`insert_lambda_literal_overrides`]'s list-quoted-lambda
/// recognition (codex review of #954's follow-up) and is otherwise `false`:
/// every other recursion (the top-level script, a body, a lambda body, a
/// case-list clause, an expression) processes source that is *itself*
/// executed code, not a value that might or might not be invoked later, so
/// list-quoted detection inside it is decided fresh at the next `[…]` hop
/// rather than inherited.
/// The head's *effective command identity*, resolved once so every
/// registry-driven pass in [`collect_script`] keys off it (issue #1185).
///
/// Covers a command imported from an exported namespace (`namespace import
/// tcltest::*` → `test` = `tcltest::test`, issue #776), a static `interp alias`
/// / `rename`, and a top-level `proc` that shadows a built-in.  Facts are
/// offset-keyed, so a binding never retroactively re-tags an earlier call, and
/// a head with nothing proven about it keeps its own spelling.
///
/// The overwhelmingly common document binds nothing, so the lookup is skipped
/// entirely rather than hashing every head in the file.
fn head_identity_of<'a>(
    ctx: ScriptCtx<'a>,
    head_text: &'a str,
    head_tok: Token,
) -> tcl_compiler::head_identity::HeadIdentity<'a> {
    if ctx.head_identities.is_empty() {
        return tcl_compiler::head_identity::HeadIdentity::Command(head_text);
    }
    ctx.head_identities
        .resolve(head_text, head_tok.span.start())
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
/// than a misleading command highlight (issue #797).
fn emit_static_command_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    identity: tcl_compiler::head_identity::HeadIdentity<'_>,
    entries: &mut Vec<Entry>,
) {
    let Some(&head_tok) = seg.argv.first() else {
        return;
    };
    emit_command_head(
        ctx.line_index,
        ctx.full_source,
        CommandHead {
            tok: head_tok,
            text: &seg.texts[0],
            resolved: identity.spec_name(),
            rebound: identity.is_rebound(),
        },
        HeadContext {
            oo_grammar: ctx.oo_grammar,
            scoped_env: ctx.scoped_env,
        },
        ctx.registry,
        entries,
    );
}

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
    let full_source = ctx.full_source;
    let registry = ctx.registry;
    for seg in segment_commands_with_offset_and_config(
        text,
        base_offset,
        tcl_lexer::LexerConfig::for_file_dialect(ctx.dialect).at_depth(depth),
    ) {
        if seg.argv.is_empty() {
            continue;
        }
        // Classify the command-head token.  A head that resolves to a registry
        // built-in carries the `defaultLibrary` modifier.
        let head_tok = seg.argv[0];
        let head_text = &seg.texts[0];
        let identity = head_identity_of(ctx, head_text, head_tok);
        let resolved_head: &str = identity.spec_name();
        let computed_head = head_is_computed(&seg);
        if !computed_head {
            emit_static_command_head(ctx, &seg, identity, entries);
        }

        // The command's argument words (head excluded), borrowed once as
        // `&[&str]` and shared by every registry-driven pass below — the
        // override builder and the OO-body context check both need it, and
        // the registry API takes `&[&str]`, so building it here keeps the
        // hot path to a single bridging allocation per command.
        let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();

        let mut overrides = special_arg_kinds(
            &seg,
            registry,
            resolved_head,
            ctx.oo_grammar,
            ctx.scoped_env,
            &arg_texts,
            ctx.object_classes,
            ctx.object_collections,
            ctx.classes,
            tcl_dialect::DialectProfile::by_name(ctx.dialect).availability_mask,
            ctx.extra_var_write,
            ctx.extra_var_read,
            ctx.extra_command,
            deferred_role,
        );
        // A `[list HEAD …]` sitting in a deferred (script) slot *is* the
        // command `HEAD …` — Tk's own `uplevel #0 [list upvar #0
        // ::tk::Priv.$disp ::tk::Priv]`.  Overlay the overrides that command
        // would get written literally, so its declarations highlight alike
        // (issue #1138).  `deferred_role` is what keeps inert data
        // (`set x [list upvar 1 a b]`) out: `list` itself invokes nothing.
        merge_list_quoted_command_overrides(&seg, ctx, registry, deferred_role, &mut overrides);
        // `my method …` inside a class body resolves against the enclosing
        // class's MRO (the most common `TclOO` dispatch form).
        insert_self_method_overrides(
            &seg,
            ctx.classes,
            registry,
            ctx.enclosing_class,
            &mut overrides,
        );
        // Regex-source tracking: retag a `set` value word that feeds a regexp
        // pattern as a (substitution-aware) regex.
        mark_regex_source_words(&seg, ctx.regex_sources, &mut overrides);

        // The definition-body grammar the recursion into THIS command's body
        // arguments should carry: an outer definer body switches to its
        // grammar, a member body (inside a definition body) switches off,
        // everything else inherits.  `oo::define`/`oo::objdefine` only switch
        // on for their bare script form, not their member (`method …`) forms —
        // hence the args are consulted.  Command substitutions and expressions
        // always run in ordinary (non-definition) context (see `plain_ctx`).
        // The outer-definer lookup reads the *resolved* head; the member
        // sub-keyword test reads the written one (issue #1275).
        let head_words = crate::oo_body::HeadWords {
            written: head_text,
            resolved: resolved_head,
        };
        let next_oo = crate::oo_body::next_definition_grammar(
            head_words,
            &arg_texts,
            ctx.oo_grammar,
            registry,
        );
        // The class whose body the recursion enters: a `oo::class create NAME`
        // (and the property-/instantiation-metaclasses) names it at argv[2], an
        // `oo::define NAME { … }` at argv[1].  Slice it from the source so it
        // outlives the walk; otherwise inherit the enclosing class (so a
        // `method …` body keeps its class).  Lets `my method …` in the body
        // resolve against that class.
        let next_class =
            definer_class_name(head_text, &seg, full_source, registry).or(ctx.enclosing_class);
        // The scoped command environment the recursion into THIS command's body
        // should carry: a command whose spec declares a `body_scope` switches it
        // on (`report::defstyle`'s style script); otherwise it persists so the
        // scoped commands stay resolvable inside nested control-flow bodies and
        // `[…]` substitutions within the style script.
        let next_scoped = registry
            .get(resolved_head)
            .and_then(|s| s.body_scope)
            .or(ctx.scoped_env);
        let body_ctx = ScriptCtx {
            oo_grammar: next_oo,
            enclosing_class: next_class,
            scoped_env: next_scoped,
            ..ctx
        };

        let deferred_role_starts =
            deferred_role_arg_starts(&seg, registry, resolved_head, &arg_texts);

        for tok in &seg.all_tokens {
            // Skip every token that falls inside a *static* head word — not
            // just the exact head token.  Such a head is one word that
            // `emit_command_head` already emitted as a single command token;
            // its sub-fragments (`ns::`, `cmd` for a `ns::cmd` head) also
            // appear in `all_tokens`, and emitting those would overlap the head
            // token (invalid — LSP clients reject overlapping semantic tokens).
            //
            // A *computed* head is deliberately NOT emitted by
            // `emit_command_head` (see above), so its tokens must flow through
            // the argument path here: the `[…]` head token recurses into its
            // inner script and the `$var` head token reads as a variable
            // (issue #797).
            if !computed_head
                && tok.span.start() >= head_tok.span.start()
                && tok.span.end() <= head_tok.span.end()
            {
                continue;
            }
            emit_arg_token(
                ctx,
                body_ctx,
                *tok,
                overrides.get(&tok.span.start()),
                deferred_role_starts.contains(&tok.span.start()),
                entries,
                depth,
            );
        }
    }
}

/// Overlay the overrides a `[list HEAD word …]` build would get if the
/// command it packs had been written literally.
///
/// `seg` is the substitution's *own* segmented content (`list upvar #0 A B`),
/// which the walker reached by recursing into the `[…]`. When its enclosing
/// slot is a deferred one ([`deferred_role_arg_starts`]) and the build is a
/// literal one ([`tcl_compiler::script_arg::list_build_effective_command`]),
/// the effective command `upvar #0 A B` is run through the same override
/// builder and its results merged.  Every word keeps its real span, so an
/// overlaid override lands on the user's own text.
///
/// First-writer-wins (`or_insert`), matching the rest of the map: an override
/// the `list` view already claimed for a span is never displaced.
///
/// This is the highlighting half of issue #1138 — the analyser's half is
/// [`tcl_compiler::analyser`]'s body gate, and both ask the *same* predicate
/// so a shape that navigates cannot fail to highlight.
fn merge_list_quoted_command_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    ctx: ScriptCtx<'_>,
    registry: &CommandRegistry,
    deferred_role: bool,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if !deferred_role {
        return;
    }
    let Some(built) = tcl_compiler::script_arg::list_build_effective_command(registry, seg) else {
        return;
    };
    let built_args: Vec<&str> = built.texts[1..].iter().map(String::as_str).collect();
    let built_head = built.texts[0].as_str();
    let overlay = special_arg_kinds(
        &built,
        registry,
        built_head,
        // A built command runs as ordinary code, never as a definition-body
        // member — `[list method foo {} {}]` is not an `oo::define` member
        // word, so the enclosing grammar must not be applied to it.
        None,
        ctx.scoped_env,
        &built_args,
        ctx.object_classes,
        ctx.object_collections,
        ctx.classes,
        tcl_dialect::DialectProfile::by_name(ctx.dialect).availability_mask,
        ctx.extra_var_write,
        ctx.extra_var_read,
        ctx.extra_command,
        // The built command is the invocation itself; nothing further defers
        // it, so its own arguments are decided fresh at the next `[…]` hop.
        false,
    );
    for (span_start, override_kind) in overlay {
        overrides.entry(span_start).or_insert(override_kind);
    }
}

/// This command's own argument slots whose registry role means "the value
/// here is later invoked/sourced as a command" — `Body` / `LambdaLiteral` /
/// `CommandPrefix` — as the set of their representative tokens' start
/// offsets. A `[…]` substitution occupying one of these slots recurses with
/// `deferred_role = true` (see [`collect_script`]) so list-quoted-lambda
/// detection ([`insert_lambda_literal_overrides`]) only fires for a
/// genuinely deferred invocation (`package ifneeded … [list apply {…}
/// $dir]`), never for inert data (`set x [list apply {…} value]`) — codex
/// review of #954's follow-up.
fn deferred_role_arg_starts(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    head: &str,
    arg_texts: &[&str],
) -> FxHashSet<u32> {
    [
        tcl_registry::ArgRole::Body,
        tcl_registry::ArgRole::LambdaLiteral,
        tcl_registry::ArgRole::CommandPrefix,
    ]
    .into_iter()
    .flat_map(|role| registry.arg_indices_for_role(head, arg_texts, role))
    .filter_map(|i| seg.argv.get(i + 1))
    .map(|t| t.span.start())
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
    if cond && let Some(kind) = classify_arg_token(tok, ctx.full_source) {
        push_token(ctx.line_index, ctx.full_source, tok, kind, 0, entries);
    }
}

/// The fixed `(kind, modifier)` for overrides that emit their token verbatim,
/// or `None` for overrides that need custom handling (recursion / sub-tokens).
fn verbatim_token_kind(ov: ArgOverride) -> Option<(TokenKind, u32)> {
    match ov {
        ArgOverride::Kind(kind) => Some((kind, 0)),
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
    // Command substitutions / expressions never run in a definition-body
    // context, whatever the enclosing command is.
    let plain_ctx = ScriptCtx {
        oo_grammar: None,
        ..ctx
    };
    // Overrides that emit their token verbatim collapse to one path.
    if let Some((kind, modifier)) = override_kind.copied().and_then(verbatim_token_kind) {
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
            let emitted = push_sprintf_subtokens(line_index, full_source, *tok, entries);
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
            } else if let Some(kind) = classify_arg_token(*tok, full_source) {
                push_token(line_index, full_source, *tok, kind, 0, entries);
            }
        }
        Some(ArgOverride::ExprScript) => {
            collect_expr(plain_ctx, *tok, entries, depth + 1);
        }
        Some(ArgOverride::CaseList(spec, regexp)) => {
            collect_case_list(body_ctx, *tok, entries, depth + 1, spec, *regexp);
        }
        Some(ArgOverride::KeywordArg) => {
            push_keyword_arg(line_index, full_source, *tok, entries);
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
    } else if let Some(kind) = classify_arg_token(tok, full_source) {
        // String / bareword args with backslash escapes split
        // into literal `String` runs + `Escape` sub-tokens.
        if kind == TokenKind::String && push_escape_subtokens(line_index, full_source, tok, entries)
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
        if let Some(kind) = classify_arg_token(tok, full_source) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let math = tcl_lexer::expr_math_functions();
    for et in tcl_lexer::tokenise_expr(inner, Some(ctx.dialect)) {
        use tcl_lexer::ExprTokenType as E;
        let abs_start = cstart + et.start as usize;
        match et.kind {
            E::Command => {
                // `[cmd …]` inside the expression — recurse into the inner
                // script (strip the surrounding `[` / `]`).
                let has_open = et.text.starts_with('[');
                let body = et.text.trim_start_matches('[').trim_end_matches(']');
                collect_script(
                    ctx,
                    body,
                    u32::try_from(abs_start + usize::from(has_open)).unwrap_or(0),
                    entries,
                    depth + 1,
                    false,
                );
            }
            E::Number => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Number,
                    entries,
                );
            }
            E::Variable => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Variable,
                    entries,
                );
            }
            // The grouping / ternary / argument-separator punctuation is as much
            // an operator as `+` or `&&`, and the expr lexer already tells them
            // apart — the walk just dropped them into the `_` arm below, so
            // `expr {($a + $b) * $c}` left its parens unstyled and
            // `$a > 1 ? "y" : "n"` left its `?` and `:` unstyled (#898 §6).
            E::Operator | E::ParenOpen | E::ParenClose | E::Comma | E::TernaryQ | E::TernaryC => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Operator,
                    entries,
                );
            }
            E::String => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::String,
                    entries,
                );
            }
            E::Bool => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Keyword,
                    entries,
                );
            }
            E::Function if !et.text.is_empty() && !et.text.contains('\n') => {
                let pos = line_index
                    .position_at_utf16(u32::try_from(abs_start).unwrap_or(0), full_source);
                let mods = if math.contains(et.text.as_str()) {
                    MOD_DEFAULT_LIBRARY
                } else {
                    0
                };
                entries.push((
                    pos.line,
                    pos.character.get(),
                    utf16_len(&et.text),
                    TokenKind::Function,
                    mods,
                ));
            }
            _ => {}
        }
    }
}

/// Walk the segmenter + comment scan and return raw
/// [`Entry`] tuples sorted by position.  Shared by `full` and `range`.
/// Augment the object-handle map with loop variables that iterate an object
/// collection — `dict for {k v} $coll {…}`, `dict map …`, `foreach v $coll {…}`,
/// `lmap …` — so a `$v method …` dispatch in the loop body resolves like a
/// `[dict get $coll $k] method …` retrieval.
///
/// A *syntactic* recursive scan of the source rather than an IR pass: the IR
/// lowers a `dict for` used as a bare statement to a barrier, but a loop nested
/// in a command substitution or `set` value (`return [dict map {k v} $coll
/// {…}]`) is folded into a value string and never surfaces as a loop.  The
/// syntactic scan sees every body regardless of how it lowers.  No-op when no
/// object collection is tracked.
fn augment_loop_var_handles(
    source: &str,
    dialect: &str,
    object_collections: &ObjectClassMap,
    object_classes: &mut ObjectClassMap,
) {
    if object_collections.is_empty() {
        return;
    }
    scan_loop_vars(
        source,
        source,
        0,
        dialect,
        object_collections,
        object_classes,
        0,
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

/// Recursive worker for [`augment_loop_var_handles`]: segment `text` (anchored
/// at `base_offset`), bind any loop's value variable(s) that iterate a tracked
/// object collection, then recurse into every braced-script word.
fn scan_loop_vars(
    full_source: &str,
    text: &str,
    base_offset: u32,
    dialect: &str,
    collections: &ObjectClassMap,
    handles: &mut ObjectClassMap,
    depth: u32,
) {
    if MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    let registry = tcl_registry::registry_for_dialect(dialect);
    for seg in segment_commands_with_offset_and_config(
        text,
        base_offset,
        tcl_lexer::LexerConfig::for_file_dialect(dialect).at_depth(depth),
    ) {
        bind_loop_vars_for_call(full_source, &seg, registry, collections, handles);
        // Recurse into braced-script words (loop bodies, proc / method / class
        // bodies, `namespace eval` blocks, `if`/`switch` arms, …) and into
        // `[…]` command substitutions (a loop can be `return [dict map …]`).
        for (i, tok) in seg.argv.iter().enumerate() {
            if !seg.single_token_word.get(i).copied().unwrap_or(false) {
                continue;
            }
            let inner_span = match tok.kind {
                TokenType::Str => subspec_content(full_source, *tok),
                TokenType::Cmd => {
                    let cstart = tok.span.start() as usize + tok.content_offset as usize;
                    let cend = (tok.span.end() as usize).min(full_source.len());
                    (cend > cstart)
                        .then(|| full_source.get(cstart..cend).map(|inner| (cstart, inner)))
                        .flatten()
                }
                _ => None,
            };
            if let Some((cstart, inner)) = inner_span {
                scan_loop_vars(
                    full_source,
                    inner,
                    u32::try_from(cstart).unwrap_or(0),
                    dialect,
                    collections,
                    handles,
                    depth + 1,
                );
            }
        }
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
/// ([`tcl_registry::HandleBindingSpec`], issue #1185) — the walker names no
/// command, so `::set` and a provable alias of it bind exactly like `set`.  Two
/// layouts exist today:
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
    dialect: &str,
    classes: Option<&ClassHierarchy>,
    object_classes: &mut ObjectClassMap,
) {
    // The member-body installers this dialect's definers inject, resolved once
    // per document rather than per segment.
    let member_bindings: FxHashMap<&'static str, tcl_registry::HandleBindingSpec> =
        tcl_registry::registry_for_dialect(dialect)
            .member_body_handle_bindings()
            .into_iter()
            .collect();
    scan_snit_handles(
        &HandleScanCtx {
            full_source: source,
            dialect,
            classes,
            member_bindings: &member_bindings,
        },
        source,
        0,
        object_classes,
        0,
    );
}

/// Bind one resolved [`BoundHandle`] into the handle map.
///
/// The two class sources are resolved differently and both abstain rather than
/// guess: a [`HandleClassSource::Word`] is a type name used as written (only a
/// static bareword qualifies), while a
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
    registry: &CommandRegistry,
    handles: &mut ObjectClassMap,
) {
    if bound.name.is_empty() || bound.name.contains(['$', '[', ' ']) {
        return;
    }
    match bound.class_source {
        tcl_registry::HandleClassSource::Word(_) => {
            let type_name = bound.class_word;
            if type_name.is_empty() || type_name.contains(['$', '[', ' ']) {
                return;
            }
            let qualified = format!("::{}", type_name.trim_start_matches("::"));
            handles
                .entry(bound.name.to_owned())
                .or_default()
                .insert(qualified);
        }
        tcl_registry::HandleClassSource::ConstructionValue(_) => {
            let Some(hierarchy) = classes else { return };
            let Some((cmd, args)) =
                tcl_compiler::value_shapes::parse_command_substitution(bound.class_word)
            else {
                return;
            };
            let Some(class) = resolve_class_in_hierarchy(hierarchy, &cmd) else {
                return;
            };
            if !family_constructs_by_bare_word(hierarchy, registry, &class) {
                return;
            }
            // A bare construction needs an instance-name argument that is not a
            // (non-`create`) typemethod call on the type.
            if !args.first().is_some_and(|a| {
                a == "create" || !class_declares_typemethod(hierarchy, registry, &class, a)
            }) {
                return;
            }
            handles
                .entry(bound.name.to_owned())
                .or_default()
                .insert(class);
        }
    }
}

/// Whether `class`'s definer family constructs an instance from a **bare
/// instance name** (`$type $name`), rather than only through an explicit
/// `create` / `new`.
///
/// Two independent sources, both **data** rather than a shape matched here:
///
/// * the class's metaclass spec's definition-body grammar declares it
///   ([`DefinitionBodyGrammar::bare_word_construction`]) — snit's `$type
///   $name` shorthand — replacing the `metaclass.starts_with("snit::")`
///   spelling test the scan used to make (issue #1185); or
/// * the class's metaclass is a **user** metaclass whose recorded class
///   factory proves its unrecognised-word fallback both constructs an object
///   and returns that word (`ClassFactory::unknown_binds_instance`) — Tk's
///   `::tk::IconList .il` idiom (issue #1303). The proof is made once, where
///   the metaclass is written, so this reads a fact rather than re-deriving
///   one from the call's shape.
///
/// A class whose metaclass is neither answers `false`: abstention, so an
/// unproved factory is never treated as one.
///
/// [`DefinitionBodyGrammar::bare_word_construction`]: tcl_registry::definer::DefinitionBodyGrammar::bare_word_construction
fn family_constructs_by_bare_word(
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
) -> bool {
    let Some(class_def) = hierarchy.classes.get(class) else {
        return false;
    };
    if registry
        .get(&class_def.metaclass)
        .and_then(|spec| spec.definition_body)
        .is_some_and(|grammar| grammar.bare_word_construction)
    {
        return true;
    }
    resolve_class_in_hierarchy(hierarchy, &class_def.metaclass)
        .and_then(|meta| hierarchy.classes.get(&meta)?.factory.as_ref())
        .is_some_and(|factory| factory.unknown_binds_instance)
}

/// Whether `name` is a type-command call (typemethod) on class `class` — one
/// of the built-in typemethods the class's definer family provides, or a
/// declared `typemethod` anywhere in the MRO.
///
/// The built-in set is registry data
/// ([`DefinitionBodyGrammar::builtin_type_methods`]), not a spelling list
/// here: snit gives every type `info` and `destroy`, and a future definer
/// family declares its own. `create` is deliberately not in that set —
/// `Type create inst` *is* a construction, not a typemethod call.
///
/// [`DefinitionBodyGrammar::builtin_type_methods`]: tcl_registry::definer::DefinitionBodyGrammar::builtin_type_methods
fn class_declares_typemethod(
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
    name: &str,
) -> bool {
    let builtin = hierarchy
        .classes
        .get(class)
        .and_then(|cd| registry.get(&cd.metaclass))
        .and_then(|spec| spec.definition_body)
        .is_some_and(|grammar| grammar.is_builtin_type_method(name));
    builtin
        || class_mro(hierarchy, class).iter().any(|c| {
            hierarchy
                .classes
                .get(c)
                .is_some_and(|cd| cd.class_methods.contains_key(name))
        })
}

/// Bind every loop variable of one call that iterates a known object
/// collection (issue #1185).
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
    source: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    collections: &ObjectClassMap,
    handles: &mut ObjectClassMap,
) {
    // A recovery segment can expose a plausible-looking `texts` prefix even
    // when the command's final word is incomplete. Binding from that prefix
    // would make a malformed or dynamic loop look proven.
    if seg.is_partial {
        return;
    }
    let Some(name) = seg.texts.first() else {
        return;
    };
    let refs: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
    for idx in registry.arg_indices_for_role(name, &refs, tcl_registry::ArgRole::LoopVarList) {
        let (Some(_var_list), Some(collection)) = (refs.get(idx), refs.get(idx + 1)) else {
            continue;
        };
        let Some(classes) = object_handle_name(collection).and_then(|c| collections.get(c)) else {
            continue;
        };
        // Segment text is not a Tcl list parser. Prove the word is static,
        // then use the shared Tcl list grammar; malformed values abstain.
        let Some(var_names) = static_loop_var_names(source, seg, idx + 1) else {
            continue;
        };
        let keyed = registry
            .arg_type_hint(name, &refs, idx + 1)
            .is_some_and(|hint| hint.expected == Some(tcl_registry::types::TclType::Dict));
        if keyed {
            if let Some(value_var) = var_names.get(1) {
                bind_loop_var(handles, value_var, classes);
            }
        } else {
            for var in &var_names {
                bind_loop_var(handles, var, classes);
            }
        }
    }
}

/// Decode a literal loop-variable-list word using Tcl's canonical list
/// grammar. Substitutions, expansions, compound words, incomplete commands,
/// and malformed list values all abstain.
fn static_loop_var_names(
    source: &str,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    word_index: usize,
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
    tcl_syntax::list::split_list(text).ok().map(|elements| {
        elements
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect()
    })
}

/// Everything an object-handle scan needs that does **not** change as the walk
/// recurses into nested scripts — bundled so the recursive worker keeps a small
/// signature.
struct HandleScanCtx<'a> {
    /// The whole document, for resolving a nested script's absolute span.
    full_source: &'a str,
    /// The document's dialect, for the lexer config and registry.
    dialect: &'a str,
    /// The class hierarchy (local, or workspace-merged in project mode), or
    /// `None` when no analysis is available.
    classes: Option<&'a ClassHierarchy>,
    /// The member-body installers this dialect's definers inject, resolved once
    /// per document (see
    /// [`CommandRegistry::member_body_handle_bindings`](tcl_registry::CommandRegistry::member_body_handle_bindings)).
    member_bindings: &'a FxHashMap<&'static str, tcl_registry::HandleBindingSpec>,
}

/// Recursive worker for [`augment_snit_handles`].
fn scan_snit_handles(
    ctx: &HandleScanCtx<'_>,
    text: &str,
    base_offset: u32,
    handles: &mut ObjectClassMap,
    depth: u32,
) {
    if MAX_TOKEN_RECURSION.exceeded(depth) {
        return;
    }
    let HandleScanCtx {
        full_source,
        dialect,
        classes,
        member_bindings,
    } = *ctx;
    let registry = tcl_registry::registry_for_dialect(dialect);
    for seg in segment_commands_with_offset_and_config(
        text,
        base_offset,
        tcl_lexer::LexerConfig::for_file_dialect(dialect).at_depth(depth),
    ) {
        let texts = &seg.texts;
        if let Some(head) = texts.first() {
            let args: Vec<&str> = texts[1..].iter().map(String::as_str).collect();
            // The layout comes from the registry — `set`'s own
            // `CommandSpec::binds_handle`, or the member-body installer the
            // class system's definition-body grammar declares — so no command
            // word is spelled here and `::set` binds exactly like `set`
            // (issue #1185).
            if let Some(binding) = member_bindings
                .get(head.as_str())
                .copied()
                .or_else(|| registry.handle_binding(head).copied())
                && let Some(bound) = binding.resolve(&args)
            {
                bind_object_handle(&bound, classes, registry, handles);
            }
        }
        for (i, tok) in seg.argv.iter().enumerate() {
            if !seg.single_token_word.get(i).copied().unwrap_or(false) {
                continue;
            }
            let inner_span = match tok.kind {
                TokenType::Str => subspec_content(full_source, *tok),
                TokenType::Cmd => {
                    let cstart = tok.span.start() as usize + tok.content_offset as usize;
                    let cend = (tok.span.end() as usize).min(full_source.len());
                    (cend > cstart)
                        .then(|| full_source.get(cstart..cend).map(|inner| (cstart, inner)))
                        .flatten()
                }
                _ => None,
            };
            if let Some((cstart, inner)) = inner_span {
                scan_snit_handles(
                    ctx,
                    inner,
                    u32::try_from(cstart).unwrap_or(0),
                    handles,
                    depth + 1,
                );
            }
        }
    }
}

fn collect_entries(
    source: &str,
    dialect: &str,
    registry: &CommandRegistry,
    cu: Option<&CompilationUnit>,
    classes: Option<&ClassHierarchy>,
    proc_roles: Option<&VarNameArgRoles>,
    named_instances: Option<&NamedInstanceMap>,
) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let line_index = LineIndex::new(source);

    // Extra variable-name argument positions the static registry doesn't model,
    // split by direction (written → `Variable` declaration, read → plain
    // `Variable`): source-derived `# tcl-lsp: stub … :var` / `:var_read` roles
    // (every path) unioned with the analyser's inferred user-proc roles
    // (`proc_roles`, when the caller supplied an analysis / project index).
    // Empty (and lookup-free) when neither source contributes.
    let mut extra_var_write: FxHashMap<String, Vec<u32>> = FxHashMap::default();
    let mut extra_var_read: FxHashMap<String, Vec<u32>> = FxHashMap::default();
    let mut extra_command: FxHashMap<String, Vec<u32>> = FxHashMap::default();
    add_stub_var_roles(
        source,
        &mut extra_var_write,
        &mut extra_var_read,
        &mut extra_command,
    );
    if let Some(roles) = proc_roles {
        roles.extend_into(
            &mut extra_var_write,
            &mut extra_var_read,
            &mut extra_command,
        );
    }

    // Regex-source spans: the def-site literal words (`set my_re ".*"`) whose
    // variable flows into a `regexp`/`regsub` pattern, keyed by word start so
    // the walk can retag the matching argument.  Empty (no map lookups) when
    // there is no analysis or no such flow.
    let regex_sources: FxHashMap<u32, Span> = cu.map_or_else(FxHashMap::default, |cu| {
        tcl_compiler::regex_source::regex_source_literal_spans(source, cu, registry, dialect)
            .into_iter()
            .map(|span| (span.start(), span))
            .collect()
    });

    // The document's command-identity facts: bare-name aliases for commands
    // imported from an exported namespace (`namespace import tcltest::*` →
    // `test` = `tcltest::test`), plus every statically proven `interp alias` /
    // `rename` / built-in-shadowing `proc` (issue #1185).  Empty (no lookups)
    // unless the document actually binds something.
    let head_identities =
        tcl_compiler::head_identity::command_head_identities(source, dialect, registry);

    // Object-handle → class provenance (`set chart [ticklecharts::chart new]`
    // → `chart`), so a `$chart Xaxis -name …` dispatch resolves the method's
    // options through the registry (issue #748).  Empty without a
    // `CompilationUnit` or when the document creates no tracked object handles.
    let mut object_classes: ObjectClassMap = cu.map_or_else(ObjectClassMap::default, |cu| {
        tcl_compiler::object_types::object_handle_classes(cu, registry)
    });

    // A bareword instance command bound by a *user*-class `CLASS create
    // NAME` (issue #1312) — the object-type lattice above only tracks `set`
    // assignments and registry naming factories, never a plain `CLASS create
    // NAME` statement, so a named instance's class comes from the analyser's
    // `instance_classes` instead, gated on `created_instance_commands`
    // exactly like the LSP's `receiver_instance_class` (hover / definition /
    // completion already resolve this shape; this closes the same gap for
    // the semantic-token / W308 dispatch resolver).  Merged into the same
    // name-keyed map a registry naming factory (`ttk::treeview .t`) already
    // populates, so `insert_object_method_overrides`'s bareword branch needs
    // no new code path — see `object_types::harvest_unit`'s `Statement::Call`
    // arm doc, which this mirrors for user classes.
    if let Some(named) = named_instances {
        for (name, class) in named {
            object_classes
                .entry(name.clone())
                .or_default()
                .insert(class.clone());
        }
    }

    // Object-*collection* → element-class map (`dict set Pins $k [Pin new]` →
    // `Pins` is a `Dict` of `Pin`), so a `[dict get $Pins $k] method …`
    // retrieval dispatch resolves the element's method (issue #797).  Empty
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
    // map (issue #797, SpiceGenTcl `allNodes` / `actOnParam` shape).
    augment_loop_var_handles(source, dialect, &object_collections, &mut object_classes);

    // snit object-handle bindings the compiler CFG doesn't surface — `install
    // NAME using TYPE` components and `set NAME [Type inst]` bare constructors —
    // via a source scan, since snit method bodies (where these live) are not
    // lowered into the CFG `object_handle_classes` reads.
    augment_snit_handles(source, dialect, classes, &mut object_classes);

    // Walk every segmented command (recursing into braced bodies, braced
    // expressions, and `[…]` command substitutions) and classify each token.
    let ctx = ScriptCtx {
        full_source: source,
        dialect,
        registry,
        line_index: &line_index,
        oo_grammar: None,
        scoped_env: None,
        regex_sources: &regex_sources,
        head_identities: &head_identities,
        object_classes: &object_classes,
        object_collections: &object_collections,
        classes,
        enclosing_class: None,
        extra_var_write: &extra_var_write,
        extra_var_read: &extra_var_read,
        extra_command: &extra_command,
    };
    collect_script(ctx, source, 0, &mut entries, 0, false);

    // Comments aren't in the segmenter's command stream
    // (it strips them).  Scan the source for `#` comments
    // separately.
    push_comment_tokens(source, &line_index, &mut entries);

    // BIG-IP object references (iRules dialect): overlay `object` tokens
    // at recognised pool / data-group / virtual / … name positions.
    // Skipped when an entry already covers the position (e.g. a
    // single-line body's enclosing `string` token) so the token stream
    // never carries overlaps.  Multi-line bodies aren't tokenised by the
    // main walk, so refs inside them surface cleanly.
    if dialect == "f5-irules" {
        for span in crate::irules_object_refs::object_ref_spans(source, registry) {
            push_object_token(source, &line_index, span, &mut entries);
        }
    }

    // Sort by (line, column) so the delta encoding works.
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
    entries
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
    let segments = tcl_lexer::split_backslash_escapes(text);
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
    // left its opening and closing `"` unstyled (#898 §1).  Entries are sorted by
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

/// Classify a non-head token by its lexer-assigned kind.
fn classify_arg_token(tok: Token, source: &str) -> Option<TokenKind> {
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
            if is_number_literal(text) {
                Some(TokenKind::Number)
            } else if text.contains("::") && tok.content_offset == 0 {
                // Only a *bare* word can be a namespace reference.  A quoted or
                // braced word is a string literal even when its content happens
                // to contain `::` — `append cmd "::scan \$field"` was painting
                // the whole quoted word as a namespace, and in doing so lost the
                // `\$` escape inside it (#898 §8).
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

/// `true` when `text` is a Tcl number literal — integer
/// (optionally signed, hex `0x...` or binary `0b...`) or
/// floating-point.
fn is_number_literal(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let trimmed = text.trim_start_matches(['+', '-']);
    if trimmed.is_empty() {
        return false;
    }
    if let Some(rest) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit() || c == '_');
    }
    if let Some(rest) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        return !rest.is_empty() && rest.chars().all(|c| matches!(c, '0' | '1' | '_'));
    }
    // Integer or float.  Use Rust's parsers for simplicity.
    text.parse::<i64>().is_ok() || text.parse::<f64>().is_ok()
}

/// Scan `source` for `#` comment lines and push each one as
/// a Comment-kind entry.
fn push_comment_tokens(source: &str, line_index: &LineIndex, entries: &mut Vec<Entry>) {
    let bytes = source.as_bytes();
    let mut line_start = true;
    // Byte offset up to which the rest of an already-emitted comment line is
    // skipped.  Derived from `char_indices` so the cursor never desyncs from
    // the iterator — the previous hand-incremented `byte_pos` drifted past the
    // buffer end on multi-comment files, slicing out of bounds (panic).
    let mut skip_until: usize = 0;
    for (idx, c) in source.char_indices() {
        if idx < skip_until {
            continue;
        }
        if c == '\n' {
            line_start = true;
            continue;
        }
        if c.is_whitespace() {
            continue;
        }
        if line_start && c == '#' {
            // Find the end of the comment, honouring backslash line
            // continuation: a physical line ending in an *odd* run of
            // backslashes (before the newline) continues the comment onto the
            // next physical line, matching Tcl's parser (issue #759).  An even
            // run (e.g. `\\`) is an escaped backslash and terminates the line.
            let mut p = idx;
            loop {
                let content_start = p;
                while p < bytes.len() && bytes[p] != b'\n' {
                    p += 1;
                }
                // Trailing backslashes on this physical line, ignoring a CRLF
                // `\r` immediately before the newline.
                let mut end = p;
                if end > content_start && bytes[end - 1] == b'\r' {
                    end -= 1;
                }
                let mut backslashes = 0usize;
                while end > content_start && bytes[end - 1] == b'\\' {
                    backslashes += 1;
                    end -= 1;
                }
                if backslashes % 2 == 1 && p < bytes.len() {
                    p += 1; // consume the `\n` and continue on the next line
                    continue;
                }
                break;
            }
            let comment_start = u32::try_from(idx).unwrap_or(0);
            let pos = line_index.position_at_utf16(comment_start, source);
            // A `#` is only a Tcl comment in command position.  This naive scan
            // can't see command position, but a physical line already covered by
            // an emitted token is inside a multi-line string / braced literal
            // (whose per-line entries are pushed before this scan), or is a
            // `switch` case-list `#` pattern element (not a comment — Tcl's
            // "comments don't work in switch" gotcha), so the `#` there is not a
            // comment.  The overlap test is per-*position* (not per-line) so a
            // genuine `;#` tail comment — whose line also carries code tokens —
            // still survives.  Suppress it to avoid an overlapping token the LSP
            // client would reject (#757, #758).
            let already_covered = entries.iter().any(|(l, c, ln, _, _)| {
                *l == pos.line && *c <= pos.character.get() && pos.character.get() < *c + *ln
            });
            if !already_covered {
                // Emit one entry per covered line: a continuation comment spans
                // several physical lines and the LSP encoding cannot represent a
                // token crossing a newline.  `push_span_entries` also strips the
                // line-ending `\r` from each segment.
                push_span_entries(
                    source,
                    line_index,
                    idx,
                    &source[idx..p],
                    TokenKind::Comment,
                    0,
                    entries,
                );
            }
            // Skip the remainder of the comment; the terminating `\n` (at `p`)
            // is processed normally and resets `line_start`.
            skip_until = p;
            line_start = false;
            continue;
        }
        // A command separator `;` returns us to command position, so a `#`
        // right after it is a trailing comment (`puts hi ;# tail`) — matching
        // Tcl and the TextMate grammar (issue #759 review).  A `;` inside a
        // string / braced literal is harmless here: the `#` it exposes is
        // already covered by that literal's tokens and suppressed above.
        if c == ';' {
            line_start = true;
            continue;
        }
        line_start = false;
    }
}

/// Push a single token into the entries list, computing
/// (line, column, length-in-chars, kind).
/// The closing delimiter a word opened at `start` expects, if it is delimited.
///
/// The lexer's span convention (documented on the `switch` case-list rebuild
/// above) is that a delimited word's `span.end()` sits **at** its closing `}` /
/// `"`, not past it — so an emitter that takes `start..end` verbatim covers
/// `opener + content` and silently drops the terminator.  Every delimited-word
/// emit path therefore has to ask for the closer back.  Issue #898 §1.
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
        // leaves just past `span.end()` (#898 §1).  Not applied to the clamped
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
    // line rather than dropped — see [`push_span_entries`] and issue #757.
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

/// Emit a structural keyword word (`if`'s then/elseif/else, `try`'s
/// on/trap/finally) as a `Keyword` token.  Offsets past any leading
/// delimiter so a quoted `"else"` — whose span starts on the opening
/// quote — marks `else` rather than `"els`, and trims the matching
/// trailing delimiter.
fn push_keyword_arg(line_index: &LineIndex, source: &str, tok: Token, entries: &mut Vec<Entry>) {
    if let Some((cstart, inner)) = subspec_content(source, tok) {
        let content = inner.trim_end_matches(['"', '}']);
        if !content.is_empty() {
            push_subtoken(
                source,
                line_index,
                cstart,
                content,
                TokenKind::Keyword,
                entries,
            );
            return;
        }
    }
    push_token(line_index, source, tok, TokenKind::Keyword, 0, entries);
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
mod tests {
    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn kinds(src: &str, dialect: &str, registry: &CommandRegistry) -> Vec<u32> {
        full(src, dialect, registry)
            .data
            .chunks(5)
            .map(|c| c[3])
            .collect()
    }

    /// Decode the packed stream into absolute `(line, col, len)` triples.
    fn decode(src: &str, dialect: &str, registry: &CommandRegistry) -> Vec<(u32, u32, u32)> {
        let st = full(src, dialect, registry);
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::new();
        for c in st.data.chunks(5) {
            let (dl, dc, len) = (c[0], c[1], c[2]);
            if dl > 0 {
                line += dl;
                col = dc;
            } else {
                col += dc;
            }
            out.push((line, col, len));
        }
        out
    }

    /// Decode the packed stream into absolute
    /// `(line, col, len, kind, modifiers)` tuples.
    fn decode_full(
        src: &str,
        dialect: &str,
        registry: &CommandRegistry,
    ) -> Vec<(u32, u32, u32, u32, u32)> {
        let st = full(src, dialect, registry);
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::new();
        for c in st.data.chunks(5) {
            let (dl, dc, len, kind, mods) = (c[0], c[1], c[2], c[3], c[4]);
            if dl > 0 {
                line += dl;
                col = dc;
            } else {
                col += dc;
            }
            out.push((line, col, len, kind, mods));
        }
        out
    }

    /// Assert no two tokens on the same line overlap (next starts at or
    /// after the previous token's end) — the client "Overlapping semantic
    /// tokens detected" invariant.
    fn assert_non_overlapping(src: &str, registry: &CommandRegistry) {
        let toks = decode(src, "tcl", registry);
        for w in toks.windows(2) {
            let (l0, c0, len0) = w[0];
            let (l1, c1, _) = w[1];
            if l0 == l1 {
                assert!(
                    c1 >= c0 + len0,
                    "overlap on line {l0}: token at col {c1} starts before \
                     previous token end {} (src={src:?}, toks={toks:?})",
                    c0 + len0,
                );
            }
        }
    }

    #[test]
    fn quoted_var_at_string_start_no_overlap() {
        // Regression: the lexer's empty-content clamp made the opening `"`
        // fragment span `"$`, overlapping the `$x` variable token.  The
        // opening fragment must shrink to just the `"`.
        let r = reg();
        assert_non_overlapping("puts \"$x y\"\n", &r);
        assert_non_overlapping("set x 1\nputs \"$x — résumé — 日本語\"\n", &r);
        // Adjacent substitutions: the empty ESC between `$a` and `$b`
        // carries no delimiter, so it must vanish entirely (no zero-area
        // overlap at the `$b`).
        assert_non_overlapping("puts \"$a$b\"\n", &r);
        // Command substitution introducer `[` at string start.
        assert_non_overlapping("puts \"[expr {1+2}] z\"\n", &r);
        // Dense line with several adjacent substitutions/strings.
        assert_non_overlapping("set a 1;set b 2;puts \"$a [expr {$a+$b}] $b\";# tail\n", &r);
    }

    #[test]
    fn quoted_string_opening_fragment_is_single_quote() {
        // `puts "$x y"` — the opening string fragment is exactly the `"`
        // (col 5, len 1), not `"$` (len 2).
        let toks = decode("puts \"$x y\"\n", "tcl", &reg());
        // The opening `"` lands at byte/col 5 on line 0 with length 1.
        assert!(
            toks.contains(&(0, 5, 1)),
            "expected a length-1 string token at col 5, got {toks:?}",
        );
    }

    #[test]
    fn known_option_classified_as_decorator() {
        // `regexp -nocase {pat} $s` — `-nocase` is a real option → decorator.
        let ks = kinds("regexp -nocase {pat} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
        // `puts -foo` — `-foo` is not an option of `puts` → not a decorator.
        let ks = kinds("puts -foo\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
    }

    #[test]
    fn abbreviated_option_classified_as_decorator() {
        // Tcl option parsing accepts unique prefixes: `lsort -inc` ⇒
        // `-increasing`, `lsearch -ex` ⇒ `-exact`.
        for src in ["lsort -inc {3 1 2}\n", "lsearch -ex {a b} b\n"] {
            let ks = kinds(src, "tcl", &reg());
            assert!(
                ks.contains(&(TokenKind::Decorator as u32)),
                "expected decorator for {src:?}; got {ks:?}"
            );
        }
        // An ambiguous prefix (`lsort -i` → -index/-indices/-integer/…) is not
        // a recognised option and stays a string.
        let ks = kinds("lsort -i {3 1 2}\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "ambiguous prefix must not be a decorator; got {ks:?}"
        );
    }

    #[test]
    fn resolve_option_prefix_resolves_and_rejects() {
        let names = ["-increasing", "-index", "-nocase", "-real"];
        assert_eq!(resolve_option_prefix("-nocase", &names), Some("-nocase")); // exact
        assert_eq!(resolve_option_prefix("-noc", &names), Some("-nocase")); // unique prefix
        assert_eq!(resolve_option_prefix("-r", &names), Some("-real")); // unique prefix
        assert_eq!(resolve_option_prefix("-in", &names), None); // ambiguous
        assert_eq!(resolve_option_prefix("-", &names), None); // bare dash
        assert_eq!(resolve_option_prefix("-zzz", &names), None); // unknown
    }

    #[test]
    fn subcommand_option_classified_as_decorator() {
        // Issue #748's own example: `file delete -force filename`.  `-force`
        // is declared on the `delete` *subcommand* (not on `file` itself), so
        // it is only recognised once subcommand options are consulted.
        let ks = kinds("file delete -force filename\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -force decorator; got {ks:?}"
        );
        // A subcommand option on a different subcommand: `file link -symbolic`.
        let ks = kinds("file link -symbolic a b\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -symbolic decorator; got {ks:?}"
        );
        // A `-`-word that is not a declared option stays a plain string, even
        // on a command that has subcommand options elsewhere.
        let ks = kinds("file delete -bogus filename\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-bogus is not a real option; got {ks:?}"
        );
        // A `-$var` substitution word must never be treated as an option.
        let ks = kinds("file delete -$flag filename\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-$flag is a substitution, not an option; got {ks:?}"
        );
    }

    #[test]
    fn unknown_head_options_classified_generically() {
        // The ngspice / ticklecharts pattern (issue #748): `$chart Xaxis -name
        // {v(anode), V} -type value -min 0.4` — the head `$chart` is an object
        // handle, unknown to the registry, so its `-switch value` pairs are
        // highlighted by the generic heuristic.
        let ks = kinds(
            "$chart Xaxis -name {v} -type value -min 0.4\n",
            "tcl",
            &reg(),
        );
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -name/-type/-min decorators on an unknown head; got {ks:?}"
        );
        assert!(
            ks.contains(&(TokenKind::OptionValue as u32)),
            "expected option values on an unknown head; got {ks:?}"
        );
        // A negative number is not an option: `$obj move -5 10`.
        let ks = kinds("$obj move -5 10\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-5 is a negative number, not an option; got {ks:?}"
        );
        // A `-$var` substitution word is not an option.
        let ks = kinds("$obj configure -$flag v\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-$flag is a substitution, not an option; got {ks:?}"
        );
        // A known command keeps the strict declared-option behaviour: `puts
        // -foo` stays a string even though the generic pass exists.
        let ks = kinds("puts -foo\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "a known command's undeclared -foo must stay a string; got {ks:?}"
        );
        // A plain bareword unknown head is a (possibly user-defined) command
        // name, not a computed dispatch: `mycmd -foo bar` stays a string so a
        // user proc's argument is not mistaken for an option.
        let ks = kinds("mycmd -foo bar\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "a bareword user command's -foo must stay a string; got {ks:?}"
        );
        // Negative special-float literals are numbers, not options: `-inf`,
        // `-Inf`, `-nan` all start with a letter but Tcl parses them as values.
        let ks = kinds("$obj set -inf\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-inf is a negative float literal, not an option; got {ks:?}"
        );
        let ks = kinds("$obj set -NaN\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-NaN is a float literal, not an option; got {ks:?}"
        );
    }

    #[test]
    fn generic_option_scan_stops_at_double_dash() {
        // Tcl's `--` ends option processing: `-real` before it is an option,
        // the `--` is the marker, and `-literal` after it is a positional
        // operand (a plain string), not an option.
        // Columns: `-real` at 9, `1` at 15, `--` at 17, `-literal` at 20.
        let mut deco: Vec<u32> = decode_full("$obj cfg -real 1 -- -literal\n", "tcl", &reg())
            .iter()
            .filter(|(_, _, _, k, _)| *k == TokenKind::Decorator as u32)
            .map(|&(_, c, _, _, _)| c)
            .collect();
        deco.sort_unstable();
        // Exactly `-real` (9) and `--` (17) — never `-literal` (20).
        assert_eq!(
            deco,
            vec![9, 17],
            "expected -real and -- as the only decorators, not -literal after --; got {deco:?}"
        );
    }

    #[test]
    fn value_taking_option_value_classified_as_option_value() {
        // `lsort -index 2 $l` — `-index` takes a value, so the literal `2` is
        // an option value (distinct from the `-index` decorator).
        let ks = kinds("lsort -index 2 $l\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -index decorator; got {ks:?}"
        );
        assert!(
            ks.contains(&(TokenKind::OptionValue as u32)),
            "expected the `2` value to be an OptionValue; got {ks:?}"
        );
        // A boolean option takes no value — the following word is not recoloured.
        // `lsort -unique $l`: `$l` stays a variable, not an OptionValue.
        let ks = kinds("lsort -unique $l\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::OptionValue as u32)),
            "boolean -unique must not mark a following value; got {ks:?}"
        );
        // A `$var` value keeps its variable highlight, not OptionValue.
        let ks = kinds("lsort -index $i $l\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::OptionValue as u32)),
            "a $var option value keeps its variable highlight; got {ks:?}"
        );
    }

    #[test]
    fn argparse_global_switch_values_classified_as_option_value() {
        // The `argparse` package command's value-taking global switches
        // (`-template`, `-level`, …) colour their following literal as an
        // OptionValue, while boolean switches (`-inline`) do not. argparse is
        // registered (package-gated), so the classifier resolves its spec.
        let ks = kinds(
            "argparse -template foo -level 2 -inline {d}\n",
            "tcl",
            &reg(),
        );
        let n_val = ks
            .iter()
            .filter(|&&k| k == TokenKind::OptionValue as u32)
            .count();
        assert_eq!(
            n_val, 2,
            "expected `foo` and `2` as OptionValues (not boolean -inline's `{{d}}`); got {ks:?}"
        );
        // A boolean global switch does not recolour the following word.
        let ks = kinds("argparse -inline {d}\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::OptionValue as u32)),
            "boolean -inline must not mark a following value; got {ks:?}"
        );
    }

    #[test]
    fn subcommand_enum_value_classified_as_enum_member() {
        // `string is alnum $s` — `alnum` is a closed-set value declared on
        // the `is` subcommand → enumMember, not a plain string.
        let ks = kinds("string is alnum $s\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::EnumMember as u32)),
            "expected an enumMember token; got {ks:?}"
        );
        // A value not in the set stays a string.
        let ks = kinds("string is bogusclass $s\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::EnumMember as u32)),
            "bogusclass is not a class; got {ks:?}"
        );
    }

    #[test]
    fn return_code_option_classified_as_decorator_issue_967() {
        // Issue #967: `return -code error "bad"` highlighted `-code` as a
        // plain string instead of an option. `-code` is a declared OptionSpec
        // on `return` (a decorator) and `error` is one of its closed-set
        // values (an enumMember); `"bad"` stays a plain string.
        let ks = kinds("return -code error \"bad\"\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -code to be a decorator; got {ks:?}"
        );
        assert!(
            ks.contains(&(TokenKind::EnumMember as u32)),
            "expected error to be an enumMember; got {ks:?}"
        );

        // `-level` is likewise a declared option (Tcl 8.5+) and its value
        // (`0`) is an OptionValue, not a plain number/string.
        let ks = kinds("return -level 0 \"bad\"\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -level to be a decorator; got {ks:?}"
        );
        assert!(
            ks.contains(&(TokenKind::OptionValue as u32)),
            "expected 0 to be an OptionValue; got {ks:?}"
        );

        // `-options $opts` — `-options` is a decorator and its dict value
        // stays highlighted as the variable it is (not recoloured away).
        let ks = kinds("return -options $opts\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -options to be a decorator; got {ks:?}"
        );
        assert!(
            ks.contains(&(TokenKind::Variable as u32)),
            "expected $opts to keep its variable highlight; got {ks:?}"
        );

        // TN: a plain word `-code` passed to a command with no declared
        // OptionSpec (`concat` takes no options at all) must not be painted
        // as an option — it is just a string argument.
        let ks = kinds("concat -code error\n", "tcl", &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-code is not a declared option of concat; got {ks:?}"
        );
    }

    #[test]
    fn info_object_class_sub_subcommand_classified_as_keyword() {
        // Issue #798: in `info object class $obj`, the `class` word is a
        // second-level subcommand (OBJECT INTROSPECTION), not a string. Both the
        // first-level `object` and the second-level `class` must read as
        // keywords (the `info` head itself is a Function).
        let kind_at = |src: &str, col: u32| -> u32 {
            decode_full(src, "tcl", &reg())
                .into_iter()
                .find(|&(_, c, _, _, _)| c == col)
                .map_or_else(
                    || panic!("no token at column {col} in {src:?}"),
                    |(_, _, _, k, _)| k,
                )
        };

        // `info object class $obj` — column 12 is `class`.
        let src = "info object class $obj\n";
        assert_eq!(
            kind_at(src, 12),
            TokenKind::Keyword as u32,
            "`class` sub-subcommand should be a keyword"
        );

        // `info class superclasses $cls` — column 11 is `superclasses`.
        let src = "info class superclasses $cls\n";
        assert_eq!(
            kind_at(src, 11),
            TokenKind::Keyword as u32,
            "`superclasses` sub-subcommand should be a keyword"
        );

        // A non-subcommand third word stays a string: `info object frobnicate`
        // — `frobnicate` is not a recognised OBJECT INTROSPECTION operation.
        let src = "info object frobnicate $obj\n";
        assert_eq!(
            kind_at(src, 12),
            TokenKind::String as u32,
            "an unknown third word must stay a string, not a keyword"
        );

        // The issue's actual form: `info object class` nested inside a command
        // substitution within an `if` expression. Highlighting must recurse into
        // the bracketed inner command, not just top-level statements. The column
        // of `class` is located dynamically to stay robust.
        let src =
            "if {([info object class $element {::Foo::Analysis}]) && ([info exists C])} {\n}\n";
        let col = u32::try_from(src.find(" class ").expect("has ` class `") + 1).unwrap();
        assert_eq!(
            kind_at(src, col),
            TokenKind::Keyword as u32,
            "`class` must highlight as a keyword even nested in an if-expr command substitution"
        );

        // Unique-prefix abbreviation (#798 fix 1): `info object cl` is Tcl's
        // abbreviation of `class`; column 12 is `cl`.
        let src = "info object cl $obj\n";
        assert_eq!(
            kind_at(src, 12),
            TokenKind::Keyword as u32,
            "a unique-prefix sub-subcommand should highlight as a keyword"
        );
        // An ambiguous prefix stays a string: `info class c` matches both
        // `call` and `constructor`, so it must not be painted a keyword.
        let src = "info class c $cls\n";
        assert_eq!(
            kind_at(src, 11),
            TokenKind::String as u32,
            "an ambiguous prefix must not highlight as a keyword"
        );
    }

    #[test]
    fn subcommand_prefix_resolution_is_dialect_aware() {
        let kind_at = |src: &str, dialect: &str, col: u32| -> u32 {
            decode_full(src, dialect, &reg())
                .into_iter()
                .find(|&(_, c, _, _, _)| c == col)
                .map_or_else(
                    || panic!("no token at col {col} in {src:?}"),
                    |(_, _, _, k, _)| k,
                )
        };
        // `string rev` is `reverse` (added 8.5): a keyword in 8.6, but an
        // unknown word in 8.4 where `reverse` does not exist (verified: tclsh8.4
        // rejects `string rev`).  Column 7 is `rev`.
        let src = "string rev abc\n";
        assert_eq!(kind_at(src, "tcl8.6", 7), TokenKind::Keyword as u32);
        assert_eq!(
            kind_at(src, "tcl8.4", 7),
            TokenKind::String as u32,
            "`string rev` is not a subcommand in 8.4"
        );

        // `info class def`: uniquely `definition` in 8.6 (keyword), but
        // ambiguous with `definitionnamespace` in 9.0 (verified against tclsh)
        // → stays a string.  Column 11 is `def`.
        let src = "info class def ::C\n";
        assert_eq!(
            kind_at(src, "tcl8.6", 11),
            TokenKind::Keyword as u32,
            "`info class def` is `definition` in 8.6"
        );
        assert_eq!(
            kind_at(src, "tcl9.0", 11),
            TokenKind::String as u32,
            "`info class def` is ambiguous in 9.0"
        );
    }

    #[test]
    fn abbreviated_first_level_subcommand_highlights_as_keyword() {
        // `string le $s` — `le` is Tcl's unique-prefix abbreviation of
        // `length`; it must highlight as a subcommand keyword (column 7).
        let src = "string le $s\n";
        let toks = decode_full(src, "tcl", &reg());
        let kind = toks
            .into_iter()
            .find(|&(_, c, _, _, _)| c == 7)
            .map(|(_, _, _, k, _)| k)
            .expect("token at col 7");
        assert_eq!(
            kind,
            TokenKind::Keyword as u32,
            "abbreviated subcommand `le` should be a keyword"
        );
        // An ambiguous prefix (`string t`) stays a string.
        let src = "string t $s\n";
        let kind = decode_full(src, "tcl", &reg())
            .into_iter()
            .find(|&(_, c, _, _, _)| c == 7)
            .map(|(_, _, _, k, _)| k)
            .expect("token at col 7");
        assert_eq!(
            kind,
            TokenKind::String as u32,
            "ambiguous prefix `t` must not be a keyword"
        );
    }

    #[test]
    fn oo_define_inline_keyword_classified_as_keyword() {
        // `oo::define Cls method foo {} {}` — the inline `method` keyword sits
        // at an argument position and must read as a keyword, not a string.
        let ks = kinds("oo::define Cls method foo {} {}\n", "tcl", &reg());
        let n_kw = ks
            .iter()
            .filter(|&&k| k == TokenKind::Keyword as u32)
            .count();
        // Two keyword tokens: the `oo::define` head and the inline `method`.
        assert!(n_kw >= 2, "expected >=2 keyword tokens; got {ks:?}");
        // `oo::define Cls self method foo {} {}` — the inner keyword after
        // `self` is highlighted too.
        let ks = kinds("oo::define Cls self method foo {} {}\n", "tcl", &reg());
        let n_kw = ks
            .iter()
            .filter(|&&k| k == TokenKind::Keyword as u32)
            .count();
        assert!(
            n_kw >= 3,
            "expected >=3 keyword tokens (head+self+method); got {ks:?}"
        );
    }

    #[test]
    fn var_write_target_carries_declaration_modifier() {
        // `set x 1` — `x` is a write target → variable + declaration.
        let toks = decode_full("set x 1\n", "tcl", &reg());
        let x = toks.iter().find(|(_, col, len, kind, _)| {
            *col == 4 && *len == 1 && *kind == TokenKind::Variable as u32
        });
        assert!(x.is_some(), "expected variable token for `x`; got {toks:?}");
        assert_eq!(
            x.unwrap().4,
            MOD_DECLARATION,
            "expected declaration modifier"
        );
    }

    #[test]
    fn bare_set_read_is_not_a_declaration() {
        // `set x` (no value) reads the variable — not a declaration.
        let ks = kinds("set x\n", "tcl", &reg());
        // No token should carry the declaration modifier here; `x` stays a
        // plain string.  (Modifier is checked in the full decode.)
        let toks = decode_full("set x\n", "tcl", &reg());
        assert!(
            !toks.iter().any(|(_, _, _, _, m)| *m == MOD_DECLARATION),
            "bare `set x` must not declare; got {toks:?} kinds {ks:?}"
        );
    }

    #[test]
    fn unset_marks_every_name_as_variable() {
        // `unset x y z` — every name is a variable, not just the first
        // (issue #774: only the first argument was highlighted).
        let toks = decode_full("unset x y z\n", "tcl", &reg());
        let vars = toks
            .iter()
            .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
            .count();
        assert_eq!(
            vars, 3,
            "all three unset names must highlight as variables; got {toks:?}"
        );
        // Leading `-nocomplain` / `--` options are not variables.
        let toks = decode_full("unset -nocomplain -- a b\n", "tcl", &reg());
        let vars = toks
            .iter()
            .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
            .count();
        assert_eq!(
            vars, 2,
            "only `a` and `b` are variables, not the leading options; got {toks:?}"
        );
    }

    #[test]
    fn global_marks_every_name_as_variable() {
        // `global a b c` — every name is a variable, not just the first.
        let toks = decode_full("proc p {} { global a b c }\n", "tcl", &reg());
        let vars = toks
            .iter()
            .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
            .count();
        assert_eq!(
            vars, 3,
            "all three global names must highlight as variables; got {toks:?}"
        );
    }

    #[test]
    fn array_element_write_not_retagged() {
        // `set arr($i) 1` — the target has a `$` substitution; leave it to the
        // default classifier so the inner `$i` variable still tokenises.
        let toks = decode_full("set arr($i) 1\n", "tcl", &reg());
        // The `$i` inside must still surface as a variable token.
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
            "expected inner $i variable token; got {toks:?}"
        );
    }

    #[test]
    fn literal_array_element_write_is_variable_declaration() {
        // A literal array-element write target highlights as one whole-word
        // `Variable` declaration, matching the `$arr(key)` read (issue #813).
        for src in [
            "set arr(key) 1\n",
            "incr count(hits)\n",
            "append log(err) x\n",
        ] {
            let toks = decode_full(src, "tcl", &reg());
            let decl = toks
                .iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION);
            assert!(
                decl,
                "expected an array-element variable declaration; got {toks:?} for {src:?}"
            );
        }
        // The target `arr(key)` is a single token spanning the whole element.
        let toks = decode_full("set arr(key) 1\n", "tcl", &reg());
        let whole = toks.iter().any(|(_, col, len, k, m)| {
            *col == 4 && *len == 8 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
        });
        assert!(
            whole,
            "expected a single length-8 variable token over `arr(key)`; got {toks:?}"
        );
        assert_non_overlapping("set arr(key) 1\n", &reg());
    }

    #[test]
    fn namespaced_array_element_write_is_variable_declaration() {
        // A namespaced array (`::ns::arr(key)`) is still a plain element.
        let toks = decode_full("set ::ns::arr(key) 1\n", "tcl", &reg());
        let decl = toks
            .iter()
            .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION);
        assert!(
            decl,
            "expected a variable declaration for the namespaced array element; got {toks:?}"
        );
    }

    #[test]
    fn unset_array_element_is_variable() {
        // `unset arr(key)` — `unset` is a VarWrite command, so the literal
        // element retags as one whole-word `Variable` declaration spanning
        // `arr(key)` (col 6, len 8).
        let toks = decode_full("unset arr(key)\n", "tcl", &reg());
        let whole = toks.iter().any(|(_, col, len, k, m)| {
            *col == 6 && *len == 8 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
        });
        assert!(
            whole,
            "expected `arr(key)` as one variable declaration; got {toks:?}"
        );
        // `unset arr($i)` — the computed subscript stays multi-token: the inner
        // `$i` (col 10) survives as its own variable, and the whole word is not
        // painted (no declaration at the word start, col 6).
        // `unset arr($i)` — the computed subscript keeps the word multi-token.
        // The inner `$i` must survive as its own variable, so the word must NOT
        // be painted whole (that would swallow the substitution)…
        let toks = decode_full("unset arr($i)\n", "tcl", &reg());
        let inner_i = toks
            .iter()
            .any(|(_, col, _, k, _)| *col == 10 && *k == TokenKind::Variable as u32);
        assert!(inner_i, "expected the inner `$i` variable; got {toks:?}");
        let painted_whole = toks
            .iter()
            .any(|(_, col, len, k, _)| *col == 6 && *len == 8 && *k == TokenKind::Variable as u32);
        assert!(
            !painted_whole,
            "computed subscript must not retag the whole word; got {toks:?}"
        );
        // …but its *literal* fragments — the array name and the closing paren —
        // are still part of the variable reference, not free-floating strings.
        // They used to fall through to the default classification and paint as
        // `string`, which is what #898 §3 was: `set env($lo)`, `unset
        // UnknownPending($name)` and friends all over Tcl's own library.
        let name_frag = toks.iter().any(|(_, col, len, k, m)| {
            *col == 6 && *len == 4 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
        });
        assert!(
            name_frag,
            "expected the `arr(` fragment as a variable; got {toks:?}"
        );
        let close_frag = toks
            .iter()
            .any(|(_, col, len, k, _)| *col == 12 && *len == 1 && *k == TokenKind::Variable as u32);
        assert!(
            close_frag,
            "expected the `)` fragment as a variable; got {toks:?}"
        );
    }

    #[test]
    fn varread_role_highlights_variable_name() {
        // A read-role variable-name argument (`info exists`, `array names`)
        // highlights its name as a plain `Variable` — no `declaration`
        // modifier, since a read references an existing variable (issue #813
        // follow-up / read side).
        for src in [
            "info exists arr(key)\n",
            "info exists scalar\n",
            "array names arr\n",
            "array get arr\n",
        ] {
            let toks = decode_full(src, "tcl", &reg());
            let var_ref = toks
                .iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == 0);
            assert!(
                var_ref,
                "expected a plain Variable reference for {src:?}; got {toks:?}"
            );
            // A read must not carry the declaration modifier.
            let decl = toks
                .iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION);
            assert!(!decl, "a read must not declare; got {toks:?} for {src:?}");
        }
    }

    #[test]
    fn varread_value_argument_is_not_a_variable() {
        // `dict get $d k` — `$d` is a value (a dict), not a var-name spot, so
        // the read-role retag must not fire on it; only the `$d` substitution
        // itself is a variable, and `k` (the key) is a plain string.
        let toks = decode_full("dict get $d k\n", "tcl", &reg());
        // Exactly one variable: the `$d` substitution.
        let vars = toks
            .iter()
            .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
            .count();
        assert_eq!(vars, 1, "only `$d` is a variable; got {toks:?}");
    }

    #[test]
    fn stub_var_arg_highlights_array_element() {
        // A `# tcl-lsp: stub` with a `:var` argument marks that position a
        // variable-name spot, so a literal array element passed there
        // highlights like `set arr(key) …` — even on the registry-only path,
        // since stub roles are derived from the document source (issue #813
        // follow-up).
        let src = "# tcl-lsp: stubs-begin\n\
                   # tcl-lsp: stub mywrite {varName:var value}\n\
                   # tcl-lsp: stubs-end\n\
                   mywrite arr(key) 1\n";
        let toks = decode_full(src, "tcl", &reg());
        let decl = toks.iter().any(|(line, _, _, k, m)| {
            *line == 3 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
        });
        assert!(
            decl,
            "expected the stubbed :var array element to highlight; got {toks:?}"
        );
    }

    #[test]
    fn stub_var_read_arg_highlights_as_reference() {
        // A `# tcl-lsp: stub … :var_read` argument marks a read-position
        // variable name, so a literal array element there highlights as a plain
        // `Variable` reference (no `declaration` modifier).
        let src = "# tcl-lsp: stubs-begin\n\
                   # tcl-lsp: stub myexists {varName:var_read}\n\
                   # tcl-lsp: stubs-end\n\
                   myexists arr(key)\n";
        let toks = decode_full(src, "tcl", &reg());
        let var_ref = toks
            .iter()
            .any(|(line, _, _, k, m)| *line == 3 && *k == TokenKind::Variable as u32 && *m == 0);
        assert!(
            var_ref,
            "expected the stubbed :var_read array element as a reference; got {toks:?}"
        );
        let decl_on_call = toks.iter().any(|(line, _, _, k, m)| {
            *line == 3 && *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION
        });
        assert!(!decl_on_call, "a read must not declare; got {toks:?}");
    }

    #[test]
    fn user_proc_var_name_arg_highlights_array_element() {
        // A user proc whose parameter the analyser infers to alias a caller
        // variable (`upvar $varName` + write) makes a literal array element at
        // that call-site position a variable-name spot — so `myset arr(key) 1`
        // highlights `arr(key)` like `set arr(key) 1` (issue #813 follow-up).
        // The plumbing is analysis-driven, so it needs the enriched path.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "proc myset {varName value} {\n\
                     upvar 1 $varName v\n\
                     set v $value\n\
                   }\n\
                   myset arr(key) 1\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let decl_on_call = |toks: &[(u32, u32, u32, u32, u32)]| {
            toks.iter().any(|&(line, _, _, k, m)| {
                line == 4 && k == TokenKind::Variable as u32 && m == MOD_DECLARATION
            })
        };
        // Without analysis, `myset` is an unknown command → `arr(key)` at the
        // call stays a plain string.
        let plain = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        assert!(
            !decl_on_call(&plain),
            "no proc-role highlight without analysis; got {plain:?}"
        );
        // With analysis, the inferred `varName` role retags `arr(key)`.
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            decl_on_call(&toks),
            "expected arr(key) at the myset call to highlight; got {toks:?}"
        );
    }

    #[test]
    fn user_proc_var_name_computed_subscript_survives() {
        // `myset arr($i) 1` — even with the proc role, a computed subscript is
        // multi-token, so it is not painted whole and the inner `$i` survives.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "proc myset {varName value} {\n\
                     upvar 1 $varName v\n\
                     set v $value\n\
                   }\n\
                   myset arr($i) 1\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        let inner_i = toks
            .iter()
            .any(|&(line, _, _, k, _)| line == 4 && k == TokenKind::Variable as u32);
        assert!(
            inner_i,
            "expected the inner $i on the call line to survive; got {toks:?}"
        );
    }

    #[test]
    fn user_proc_var_read_arg_highlights_as_reference() {
        // A proc that only *reads* its upvar'd parameter (`upvar $varName v`
        // then reads `v`) infers a `VarRead` role, so a literal name at the
        // call site highlights as a plain `Variable` reference — no
        // `declaration` modifier.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "proc myexists {varName} {\n\
                     upvar 1 $varName v\n\
                     return [info exists v]\n\
                   }\n\
                   myexists arr(key)\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        let ref_on_call = toks
            .iter()
            .any(|&(line, _, _, k, m)| line == 4 && k == TokenKind::Variable as u32 && m == 0);
        assert!(
            ref_on_call,
            "expected arr(key) at the myexists call as a reference; got {toks:?}"
        );
    }

    #[test]
    fn user_proc_dynamic_name_read_args_highlight() {
        // A proc that reads a dynamic variable name built from its parameters
        // inside a command substitution — `return [set ${v}($k)]` — infers a
        // read role for both `v` and `k`, so the literal call-site args
        // (`b aa foo`) highlight as plain `Variable` references.  Exercises the
        // full path: cmd-substitution recursion + compound-name inference in
        // the analyser, through to the token retag.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "array set aa {foo 1}\n\
                   proc b {v k} {\n\
                       return [set ${v}($k)]\n\
                   }\n\
                   b aa foo\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // On the call line (`b aa foo`, line 4) both `aa` (col 2) and `foo`
        // (col 5) highlight as plain `Variable` references (no declaration).
        let refs: Vec<_> = toks
            .iter()
            .filter(|&&(line, _, _, k, m)| line == 4 && k == TokenKind::Variable as u32 && m == 0)
            .collect();
        assert!(
            refs.iter().any(|&&(_, col, _, _, _)| col == 2)
                && refs.iter().any(|&&(_, col, _, _, _)| col == 5),
            "expected `aa` and `foo` as variable references on the call line; got {toks:?}"
        );
    }

    #[test]
    fn user_proc_command_arg_highlights_as_function() {
        // A dispatcher proc invokes its parameter as a command (`$cmd …`), so
        // the analyser infers `cmd` is a `Command`.  The literal command name at
        // the call site (`dispatch greet …`) then highlights as a `Function`.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "proc dispatch {cmd arg} {\n\
                     $cmd $arg\n\
                   }\n\
                   dispatch greet hello\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        // Without analysis, `greet` is an unknown command's plain string arg.
        let plain = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        let greet_fn = |toks: &[(u32, u32, u32, u32, u32)]| {
            toks.iter().any(|&(line, col, _, k, _)| {
                line == 3 && col == 9 && k == TokenKind::Function as u32
            })
        };
        assert!(
            !greet_fn(&plain),
            "no command role without analysis; got {plain:?}"
        );
        // With analysis, `greet` (col 9 on the call line) highlights as a command.
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            greet_fn(&toks),
            "expected `greet` at the dispatch call to highlight as a Function; got {toks:?}"
        );
    }

    #[test]
    fn command_prefix_callback_head_highlights_as_function() {
        // A registry `CommandPrefix` callback head retags as a Function — driven
        // by the declarative role (no analysis needed).  Covers a core
        // `-command` and a Tk `scale -command` (the script→prefix conversion):
        // under the old `script()` a bareword single-word callback head was not
        // recursed, so it did not highlight; as a prefix it now does.
        let registry = reg();
        let slice = |src: &str, line: u32, col: u32, len: u32| -> String {
            src.lines()
                .nth(line as usize)
                .and_then(|l| l.get(col as usize..(col + len) as usize))
                .unwrap_or_default()
                .to_owned()
        };
        let highlights_fn = |src: &str, name: &str| {
            decode_full(src, "tcl9.0", &registry)
                .iter()
                .any(|&(line, col, len, k, _)| {
                    k == TokenKind::Function as u32 && slice(src, line, col, len) == name
                })
        };
        assert!(
            highlights_fn(
                "proc myCompare {a b} { expr {$a - $b} }\nlsort -command myCompare {3 1 2}\n",
                "myCompare",
            ),
            "the lsort -command callback head must highlight as a Function"
        );
        assert!(
            highlights_fn(
                "proc onScale {v} { }\nscale .s -command onScale\n",
                "onScale",
            ),
            "the Tk scale -command callback head must highlight as a Function (script→prefix conversion)"
        );
    }

    #[test]
    fn method_body_recurses_in_class_definition_script() {
        // `oo::class create C { method m {} { set z 3 } }` — the method body
        // must be tokenised (C Tcl evaluates it as a script), so `set` reads
        // as a function and `z` as a variable declaration, not one opaque
        // string.
        let src = "oo::class create C {\n  method m {} {\n    set z 3\n  }\n}\n";
        let toks = decode_full(src, "tcl", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
            "expected `z` as a variable declaration inside the method body; got {toks:?}"
        );
        // constructor body too.
        let src = "oo::class create C {\n  constructor {} {\n    set q 9\n  }\n}\n";
        let toks = decode_full(src, "tcl", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
            "expected `q` declared inside the constructor body; got {toks:?}"
        );
    }

    #[test]
    fn option_command_value_recurses_as_body_script() {
        // `button .b -command {puts $x}` — the `-command` value is a script
        // body (Phase 3: ArgRole::Body), so it recurses: `$x` inside the braces
        // resolves as a Variable rather than one opaque string.
        let toks = decode_full("button .b -command {puts $x}\n", "tk", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
            "expected $x resolved inside the -command body; got {toks:?}"
        );
    }

    #[test]
    fn console_eval_body_recurses_into_script() {
        // `console eval {puts $x}` (issue #925) — the `console eval` script
        // argument is a body (ArgRole::Body via the `console` SubCommand
        // table), so it recurses: `$x` inside the braces resolves as a
        // Variable rather than the whole `{...}` staying one opaque string.
        let toks = decode_full("console eval {puts $x}\n", "tk", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
            "expected $x resolved inside the `console eval` body; got {toks:?}"
        );
    }

    #[test]
    fn consoleinterp_eval_and_record_bodies_recurse_into_script() {
        // `consoleinterp eval`/`record` (issue #925 follow-up) — both take a
        // script argument that should recurse the same way `console eval`
        // does.
        for sub in ["eval", "record"] {
            let src = format!("consoleinterp {sub} {{puts $x}}\n");
            let toks = decode_full(&src, "tk", &reg());
            assert!(
                toks.iter()
                    .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
                "expected $x resolved inside `consoleinterp {sub}` body; got {toks:?}"
            );
        }
    }

    #[test]
    fn option_enum_value_is_enum_member() {
        // `button .b -relief raised` — the closed-set option value is coloured
        // as an EnumMember (Phase 5), not a generic OptionValue.
        let toks = decode_full("button .b -relief raised\n", "tk", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::EnumMember as u32),
            "expected `raised` as EnumMember; got {toks:?}"
        );
    }

    #[test]
    fn option_textvariable_value_is_variable_declaration() {
        // `entry .e -textvariable myvar` — the value names a variable the widget
        // reads/writes (Phase 3: ArgRole::VarWrite), so it is a Variable
        // declaration, not a plain `OptionValue` string.
        let toks = decode_full("entry .e -textvariable myvar\n", "tk", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
            "expected myvar as a variable declaration; got {toks:?}"
        );
    }

    #[test]
    fn regex_pattern_with_substitution_splits_regex_and_tcl() {
        // `regexp "abc$var.*" $s` — literal `abc` / `.*` sub-tokenise as
        // regex, but `$var` stays a Tcl variable (Tcl resolves it before
        // regexp runs), with no overlap.
        let toks = decode_full("regexp \"abc$var.*\" $s\n", "tcl", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
            "expected $var as a variable; got {toks:?}"
        );
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::RegexpQuantifier as u32),
            "expected the `*` quantifier from the literal part; got {toks:?}"
        );
        // No overlaps.
        let simple = decode("regexp \"abc$var.*\" $s\n", "tcl", &reg());
        for w in simple.windows(2) {
            let (l0, c0, len0) = w[0];
            let (l1, c1, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "overlap; toks={simple:?}");
            }
        }
        // `regexp "$only" $s` — a bare-substitution pattern is just a
        // variable, not a regex anchor.
        let toks = decode_full("regexp \"$only\" $s\n", "tcl", &reg());
        assert!(
            !toks
                .iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::RegexpAnchor as u32),
            "the `$` must not be a regex anchor; got {toks:?}"
        );
    }

    #[test]
    fn regex_source_variable_highlights_def_site_literal() {
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "set my_re \".*abc\"\nregexp $my_re $s\n";
        // Without a CompilationUnit the `set` value is a plain string.
        let plain = decode_full(src, "tcl9.0", &registry);
        assert!(
            !plain
                .iter()
                .any(|&(l, _, _, k, _)| l == 0 && k == TokenKind::RegexpQuantifier as u32),
            "no regex tokens without a CU; got {plain:?}"
        );
        // With a CU, the `.*abc` literal at the `set` reads as a regex.
        let cu = CompilationUnit::build_for(src, &registry, false);
        let st = full_with_cu(src, "tcl9.0", &registry, Some(&cu));
        let toks = decode_semantic(&st);
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, _)| l == 0 && k == TokenKind::RegexpQuantifier as u32),
            "expected the `*` from the def-site literal as a regex quantifier; got {toks:?}"
        );
        // No overlaps introduced.
        for w in toks.windows(2) {
            let (l0, c0, len0, _, _) = w[0];
            let (l1, c1, _, _, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "overlap; toks={toks:?}");
            }
        }
    }

    #[test]
    fn regex_source_tracks_inside_oo_method_body() {
        // TclOO method bodies are lowered as their own `FunctionUnit`s, so a
        // `set re "…"; regexp $re` inside a method highlights the def-site
        // literal just like one in a proc — end-to-end through the CU overlay.
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::class create C {\n  method m {s} {\n    set re \".*x\"\n    regexp $re $s\n  }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        assert!(
            toks.iter()
                .any(|&(_, _, _, k, _)| k == TokenKind::RegexpQuantifier as u32),
            "expected a regex quantifier from the method-body def-site literal; got {toks:?}"
        );
        for w in toks.windows(2) {
            let (l0, c0, len0, _, _) = w[0];
            let (l1, c1, _, _, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "overlap; toks={toks:?}");
            }
        }
    }

    #[test]
    fn object_method_options_resolve_via_registry() {
        // The ngspice / ticklecharts pattern (issue #748), end-to-end through
        // the CompilationUnit: `set chart [ticklecharts::chart new]` binds the
        // handle's class, so `$chart Xaxis -name … -type value …` resolves the
        // method and its declared options through the registry's object-class
        // model — the precise path, not the shape-based fallback.
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "set chart [ticklecharts::chart new]\n$chart Xaxis -name {v(anode), V} -type value -min 0.4 -splitLine {show True}\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        let ks: Vec<u32> = toks.iter().map(|&(_, _, _, k, _)| k).collect();
        // `Xaxis` resolved as a callable method.
        assert!(
            ks.contains(&(TokenKind::Function as u32)),
            "expected Xaxis as a Function token; got {ks:?}"
        );
        // `-name` / `-type` / `-min` / `-splitLine` are decorators.
        assert!(
            ks.iter()
                .filter(|&&k| k == TokenKind::Decorator as u32)
                .count()
                >= 4,
            "expected the four axis options as decorators; got {ks:?}"
        );
        // `-type value` is a closed-set member; `-min 0.4` a generic value.
        assert!(
            ks.contains(&(TokenKind::EnumMember as u32)),
            "expected -type's `value` as an EnumMember; got {ks:?}"
        );
        assert!(
            ks.contains(&(TokenKind::OptionValue as u32)),
            "expected -min's `0.4` as an OptionValue; got {ks:?}"
        );
    }

    #[test]
    fn direct_constructor_dispatch_resolves_method() {
        // A direct `[Class new] method …` dispatch (no intermediate variable)
        // resolves the method and its options through the registry too.
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "[ticklecharts::chart new] Yaxis -name Y -max 10\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        let ks: Vec<u32> = toks.iter().map(|&(_, _, _, k, _)| k).collect();
        assert!(
            ks.contains(&(TokenKind::Decorator as u32))
                && ks.contains(&(TokenKind::OptionValue as u32)),
            "expected -name/-max decorators + values on direct dispatch; got {ks:?}"
        );
    }

    #[test]
    fn collection_dispatch_resolves_user_configurable_method() {
        // Issue #797 end-to-end: a `Pins` dict is filled with `[Pin new]`
        // handles in one method and an element is dispatched with
        // `[dict get $Pins $pin] configure -node …` in another.  The receiver
        // resolves to the user `oo::configurable` class, so `configure` is a
        // method and `-node` (a declared property) an option — not the plain
        // strings they read as without collection-element + user-class
        // resolution.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {pin} { dict append Pins $pin [Pin new] }\n\
                     method cfg {pin node} { [dict get $Pins $pin] configure -node $node }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        // The dispatch line carries `dict` (a `defaultLibrary` Function); the
        // resolved *method* is a plain Function (no `defaultLibrary`), which is
        // the signal that distinguishes resolution from the built-in.
        let user_method_on_dispatch_line = |toks: &[(u32, u32, u32, u32, u32)]| {
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0)
        };
        // Without analysis: `configure` stays an unresolved string — only
        // `dict` (defaultLibrary) is a Function on the line.
        let plain = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        assert!(
            !user_method_on_dispatch_line(&plain),
            "without analysis, no user method resolves; got {plain:?}"
        );
        // With analysis: the dynamic dispatch resolves `configure` as a method.
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            user_method_on_dispatch_line(&toks),
            "`configure` on the retrieved Pin should resolve to a method; got {toks:?}"
        );
    }

    #[test]
    fn dict_for_loop_var_dispatch_resolves() {
        // `dict for {k pin} $Pins {$pin configure …}` — iterating an object
        // collection binds `pin` to an element, so the loop-body dispatch
        // resolves the user method just like the `[dict get …]` retrieval
        // (SpiceGenTcl `allNodes` / `floating` shape, issue #797).
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { dict append Pins $p [Pin new] }\n\
                     method dump {} { dict for {k pin} $Pins { puts [$pin configure -node] } }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `configure` on the loop var resolves to a user method (plain Function,
        // no `defaultLibrary`) on the `dump` method's line.
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0),
            "expected `configure` on the dict-for value var to resolve; got {toks:?}"
        );
    }

    #[test]
    fn my_self_call_resolves() {
        // `my method …` inside a class body resolves against the enclosing
        // class's MRO — the single most common TclOO dispatch form (2935
        // occurrences across the tcllib/tklib/SpiceGenTcl corpus).
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::class create C {\n\
                   \x20   method helper {} {}\n\
                   \x20   method run {} { my helper }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `my helper` on line 2 resolves the sibling method.
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
            "expected `my helper` to resolve; got {toks:?}"
        );
    }

    /// A bare (namespace-less) `apply {{} {...}}` runs its body in a *fresh*
    /// call frame in the global namespace by default — never the enclosing
    /// method's object namespace — so `my helper` inside it is not actually a
    /// call to the enclosing class's method at runtime (`my` isn't defined in
    /// `::`); a class-definition-body scan would raise "invalid command name
    /// my" here. The `enclosing_class` context must not leak into an
    /// `apply`-lambda body the way it correctly persists into an ordinary
    /// nested `if`/`foreach` body, or this gets painted as a resolved,
    /// legitimate method call anyway (codex-review-adjacent follow-up to
    /// issue #954: the same fresh-frame class of bug already fixed for
    /// call-graph namespace resolution, param traits, and declarations).
    #[test]
    fn my_call_inside_apply_lambda_body_does_not_resolve() {
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::class create C {\n\
                   \x20   method helper {} {}\n\
                   \x20   method run {} { apply {{} {my helper}} }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            !toks
                .iter()
                .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
            "a bare apply lambda's `my helper` must not resolve as the \
             enclosing class's method — it isn't one at runtime; got {toks:?}"
        );
    }

    #[test]
    fn snit_self_call_resolves() {
        // `$self method …` inside a snit method body resolves against the
        // enclosing snit type — snit's analogue of `TclOO`'s `my`, and by far
        // the dominant unresolved receiver on real corpora (`$self` alone is
        // ~12.6% of the unresolved `$var` dispatches; see experiments/tcloo_diag).
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "snit::type C {\n\
                   \x20   method helper {} {}\n\
                   \x20   method run {} { $self helper }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
            "expected `$self helper` in a snit method to resolve; got {toks:?}"
        );
    }

    #[test]
    fn snit_install_component_dispatch_resolves() {
        // `install axis using verticalAxis …` types the `axis` component, so a
        // `$axis method` dispatch in the snit body resolves — the snit component
        // idiom.  A source scan supplies the class (snit bodies aren't lowered).
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "snit::widget verticalAxis { method draw {} {} }\n\
                   snit::widget chart {\n\
                   \x20   component axis\n\
                   \x20   constructor {args} {\n\
                   \x20     install axis using verticalAxis $win.a\n\
                   \x20     $axis draw\n\
                   \x20   }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `$axis draw` on line 5 resolves the component's method.
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 5 && k == TokenKind::Method as u32 && m == 0),
            "expected `$axis draw` on an installed component to resolve; got {toks:?}"
        );
    }

    /// snit's `installhull using TYPE …` binds the widget's **implicit** hull
    /// component, whose name appears nowhere in the call (issue #1275).
    ///
    /// VERIFIED against tcllib snit(n): "Given this form, `installhull` creates
    /// the hull widget, and initializes any options delegated to the hull from
    /// the Tk option database."  The second documented form, `installhull
    /// $win`, names an already-created widget and carries no static type word,
    /// so it must state nothing.
    /// Issue #1275's third residual — `configure` / `cget` resolution must key
    /// off registry data about the metaclass, not the `oo::configurable`
    /// spelling.
    ///
    /// tclsh 9.0.4 oracle: an `oo::configurable create Point { property x y … }`
    /// instance answers `[$pt configure]` with `-x 27 -y 0`, while an
    /// `oo::class` instance answers `unknown method "configure": must be
    /// destroy or m` for both `configure` and `cget`.
    #[test]
    fn configure_resolves_by_metaclass_trait_not_spelling() {
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let resolves = |src: &str, line: u32| {
            let cu = CompilationUnit::build_for(src, &registry, false);
            let analysis = Analyser::new().analyse(src, "tcl9.0");
            decode_semantic(&full_with_cu_and_analysis(
                src,
                "tcl9.0",
                &registry,
                Some(&cu),
                Some(&analysis),
            ))
            .iter()
            .any(|&(l, _, _, k, m)| l == line && k == TokenKind::Method as u32 && m == 0)
        };
        // A configurable class declaring no property of its own: only the
        // metaclass fact can answer, and it must.
        let src = "oo::configurable create Point {}\nset pt [Point new]\n$pt configure -x 1\n";
        assert!(
            resolves(src, 2),
            "`configure` on an `oo::configurable` instance must resolve"
        );
        // A plain `oo::class` instance answers no `configure` at all.
        let src =
            "oo::class create Plain { method m {} {} }\nset p [Plain new]\n$p configure -x 1\n";
        assert!(
            !resolves(src, 2),
            "`configure` on a plain `oo::class` instance must not resolve"
        );
    }

    #[test]
    fn snit_installhull_types_the_implicit_hull_component() {
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let resolves = |src: &str| {
            let cu = CompilationUnit::build_for(src, &registry, false);
            let analysis = Analyser::new().analyse(src, "tcl9.0");
            decode_semantic(&full_with_cu_and_analysis(
                src,
                "tcl9.0",
                &registry,
                Some(&cu),
                Some(&analysis),
            ))
            .iter()
            .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0)
        };
        let src = "snit::widget frameish { method draw {} {} }\n\
                   snit::widgetadaptor chart {\n\
                   \x20   constructor {args} {\n\
                   \x20     installhull using frameish\n\
                   \x20     $hull draw\n\
                   \x20   }\n\
                   }\n";
        assert!(
            resolves(src),
            "`installhull using frameish` must type the implicit `hull` component"
        );
        // The already-created form has no static type word — abstain.
        let src = "snit::widget frameish { method draw {} {} }\n\
                   snit::widgetadaptor chart {\n\
                   \x20   constructor {args} {\n\
                   \x20     installhull $win\n\
                   \x20     $hull draw\n\
                   \x20   }\n\
                   }\n";
        assert!(
            !resolves(src),
            "`installhull $win` states no class; `$hull draw` must not resolve"
        );
    }

    #[test]
    fn snit_bare_constructor_dispatch_resolves() {
        // snit's bare-word constructor `set eng [Engine ${selfns}::e]` (no
        // `create` keyword) types `eng` as the snit class, so `$eng run` in
        // another method resolves.  A source scan supplies it (snit bodies
        // aren't lowered), gated on Engine being a known snit-family class.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "snit::type Engine { method run {} {} }\n\
                   snit::type Wrapper {\n\
                   \x20   variable eng\n\
                   \x20   constructor {} { set eng [Engine ${selfns}::e] }\n\
                   \x20   method go {} { $eng run }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `$eng run` on line 4 resolves the method.
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0),
            "expected `$eng run` on a bare-constructor handle to resolve; got {toks:?}"
        );
    }

    #[test]
    fn snit_typemethod_call_does_not_type_handle() {
        // `set x [Engine spawn]` is a *typemethod* call, not a construction —
        // its result is not an Engine, so `$x run` must NOT resolve (soundness:
        // the bare-constructor scan excludes declared typemethods).
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "snit::type Engine { method run {} {} \n\
                   \x20   typemethod spawn {} {} }\n\
                   set x [Engine spawn]\n\
                   $x run\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `$x run` on line 3 must stay unresolved (`run` not a Function token).
        assert!(
            !toks
                .iter()
                .any(|&(l, _, _, k, _)| l == 3 && k == TokenKind::Function as u32),
            "expected `$x run` on a typemethod result to abstain; got {toks:?}"
        );
    }

    #[test]
    fn my_configure_property_options_resolve() {
        // `my configure -prop` inside an oo::configurable body colours the
        // property option too.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create C {\n\
                   \x20   property node\n\
                   \x20   constructor {} { my configure -node 0 }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `configure` resolves (Function) and `-node` is a decorator on line 2.
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
            "expected `my configure` to resolve; got {toks:?}"
        );
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, _)| l == 2 && k == TokenKind::Decorator as u32),
            "expected `-node` property option; got {toks:?}"
        );
    }

    #[test]
    fn proc_return_object_dispatch_resolves() {
        // `proc make {} {return [C new]}; set o [make]; $o m` — the factory's
        // return type flows to `o`, so the dispatch resolves (interproc return).
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::class create C { method mrun {} {} }\n\
                   proc make {} { return [C new] }\n\
                   set o [make]\n\
                   $o mrun\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 3 && k == TokenKind::Method as u32 && m == 0),
            "expected `$o mrun` on a factory return to resolve; got {toks:?}"
        );
    }

    #[test]
    fn interproc_param_dispatch_resolves() {
        // `set p [Pin new]; connect $p` binds `connect`'s parameter `dev` to
        // ::Pin (interprocedural provenance), so `$dev configure -node` inside
        // the proc resolves the method — the param-receiver case.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   proc connect {dev t} { $dev configure -node $t }\n\
                   set p [Pin new]\n\
                   connect $p n1\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `configure` on the proc's object parameter (line 1) resolves.
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 1 && k == TokenKind::Method as u32 && m == 0),
            "expected `$dev configure` in the proc body to resolve; got {toks:?}"
        );
    }

    /// Expected outcome for a `tcloo_dispatch_cases` fixture row.
    #[derive(Clone, Copy, PartialEq)]
    enum Expect {
        Resolve,
        Abstain,
    }

    use Expect::{Abstain, Resolve};

    /// The `TclOO` object-method dispatch fixture rows:
    /// `(name, source, method word, 0-based dispatch line, expectation)`.
    const TCLOO_DISPATCH_CASES: &[(&str, &str, &str, u32, Expect)] = &[
        // ---- statically determinable → resolve ----
        (
            "var_new",
            "oo::class create C { method mrun {} {} }\nset o [C new]\n$o mrun\n",
            "mrun",
            2,
            Resolve,
        ),
        (
            "direct_new",
            "oo::class create C { method mrun {} {} }\n[C new] mrun\n",
            "mrun",
            1,
            Resolve,
        ),
        (
            // Issue #1322: TclOO's own same-object dispatch idiom — `self`
            // called with no argument returns the current object's own
            // command name, and dispatching through it (`[self] m`) reaches
            // the enclosing class exactly like `my m`.
            "bare_self_receiver",
            "oo::class create C {\n    method mrun {} {}\n    method call {} { [self] mrun }\n}\n",
            "mrun",
            2,
            Resolve,
        ),
        (
            // The explicit spelling `self object` — documented as
            // equivalent to a bare `self` (issue #1322).
            "self_object_receiver",
            "oo::class create C {\n    method mrun {} {}\n    method call {} { [self object] mrun }\n}\n",
            "mrun",
            2,
            Resolve,
        ),
        (
            "configurable_property",
            "oo::configurable create C { property node }\nset o [C new]\n$o configure -node 1\n",
            "configure",
            2,
            Resolve,
        ),
        (
            "inherited_method",
            "oo::class create B { method base {} {} }\noo::class create D { superclass B }\nset o [D new]\n$o base\n",
            "base",
            3,
            Resolve,
        ),
        (
            "mixin_method",
            "oo::class create M { method mixm {} {} }\noo::class create C { mixin M }\nset o [C new]\n$o mixm\n",
            "mixm",
            3,
            Resolve,
        ),
        (
            "dict_collection",
            "oo::class create C { method mrun {} {} }\ndict set d k [C new]\n[dict get $d k] mrun\n",
            "mrun",
            2,
            Resolve,
        ),
        (
            "foreach_loopvar",
            "oo::class create C { method mrun {} {} }\nlappend objs [C new]\nforeach o $objs { $o mrun }\n",
            "mrun",
            2,
            Resolve,
        ),
        (
            "interproc_param",
            "oo::class create C { method mrun {} {} }\nproc f {o} { $o mrun }\nset p [C new]\nf $p\n",
            "mrun",
            1,
            Resolve,
        ),
        (
            "proc_return",
            "oo::class create C { method mrun {} {} }\nproc make {} { return [C new] }\nset o [make]\n$o mrun\n",
            "mrun",
            3,
            Resolve,
        ),
        (
            "my_self_call",
            "oo::class create C {\n  method helper {} {}\n  method run {} { my helper }\n}\n",
            "helper",
            2,
            Resolve,
        ),
        (
            "snit_self_call",
            "snit::type C {\n  method helper {} {}\n  method run {} { $self helper }\n}\n",
            "helper",
            2,
            Resolve,
        ),
        (
            "itcl_this_call",
            "itcl::class C {\n  method helper {} {}\n  method run {} { $this helper }\n}\n",
            "helper",
            2,
            Resolve,
        ),
        (
            "snit_install_component",
            "snit::widget Ax { method draw {} {} }\nsnit::widget C {\n  constructor {} { install ax using Ax $win.a\n    $ax draw }\n}\n",
            "draw",
            3,
            Resolve,
        ),
        (
            "snit_bare_constructor",
            "snit::type Eng { method run {} {} }\nsnit::type C {\n  variable e\n  constructor {} { set e [Eng ${selfns}::x] }\n  method go {} { $e run }\n}\n",
            "run",
            4,
            Resolve,
        ),
        (
            "oo_define_added",
            "oo::class create C {}\noo::define C { method added {} {} }\nset o [C new]\n$o added\n",
            "added",
            3,
            Resolve,
        ),
        (
            "registry_class",
            "set c [ticklecharts::chart new]\n$c Xaxis -name x\n",
            "Xaxis",
            1,
            Resolve,
        ),
        (
            // Tk widget instance dispatch, bareword receiver (issue #927):
            // `ttk::treeview .t` names a widget path exactly like a registry
            // naming factory (`struct::graph g`) — `.t instate …` resolves
            // through the same self-referential `object_class`.
            "widget_bareword",
            "ttk::treeview .t\n.t instate {selected} {}\n",
            "instate",
            1,
            Resolve,
        ),
        (
            // Tk widget instance dispatch, `$var`-captured constructor
            // return value (the issue's own `set lb [listbox .l]` example).
            "widget_var_captured",
            "set lb [listbox .l]\n$lb curselection\n",
            "curselection",
            1,
            Resolve,
        ),
        // ---- genuinely dynamic → must abstain (soundness) ----
        (
            "introspection_class",
            "oo::class create C { method mrun {} {} }\nset o [C new]\nset cls [info object class $o]\n[$cls new] mrun\n",
            "mrun",
            3,
            Abstain,
        ),
        (
            "oo_copy",
            "oo::class create C { method mrun {} {} }\nset a [C new]\nset b [oo::copy $a]\n$b mrun\n",
            "mrun",
            3,
            Abstain,
        ),
        (
            "unknown_param",
            "proc f {o} { $o mrun }\n",
            "mrun",
            0,
            Abstain,
        ),
        (
            // `CLASS create NAME` (issue #1312) — resolved via
            // `instance_classes` gated on `created_instance_commands`,
            // merged into the object-class map `insert_object_method_overrides`'s
            // bareword branch already reads (see `NamedInstanceMap`).
            "named_object",
            "oo::class create C { method mrun {} {} }\nC create obj\nobj mrun\n",
            "mrun",
            2,
            Resolve,
        ),
        (
            // The snit *named-constructor* shape: `$o` bound by `foo create
            // x` types as `foo` (the signature scan records snit types as
            // classes), so the dispatch resolves like any handle.
            "snit_named_object",
            "snit::type foo { method smeth {} {} }\nset o [foo create x]\n$o smeth\n",
            "smeth",
            2,
            Resolve,
        ),
    ];

    /// Golden fixture for `TclOO` object-method dispatch resolution — validated
    /// against the Tcler's-wiki pattern catalogue and a real corpus (tcllib,
    /// tklib, `SpiceGenTcl`).  Two guarantees:
    ///
    /// * **`Resolve`** — a statically-determinable dispatch colours its method a
    ///   callable (regression guard for every form we support).
    /// * **`Abstain`** — a genuinely-dynamic dispatch (or a form we do not model)
    ///   leaves its method a plain string, never a *mis-highlighted* callable
    ///   (soundness guard: no false positives).
    ///
    /// Adding a pattern here is how object-dispatch support is expanded and
    /// measured; flip an `Abstain` to `Resolve` when a form becomes supported.
    #[test]
    fn tcloo_dispatch_pattern_fixture() {
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let cases = TCLOO_DISPATCH_CASES;
        let registry = reg();
        let mut failures = Vec::new();
        for &(name, src, method, line, expect) in cases {
            let cu = CompilationUnit::build_for(src, &registry, false);
            let analysis = Analyser::new().analyse(src, "tcl9.0");
            let toks = decode_semantic(&full_with_cu_and_analysis(
                src,
                "tcl9.0",
                &registry,
                Some(&cu),
                Some(&analysis),
            ));
            // Column of the method word on the dispatch line (word-boundary,
            // first occurrence — unambiguous in these snippets).  ASCII, so byte
            // == UTF-16 column.
            let src_line = src.lines().nth(line as usize).unwrap_or("");
            let mcol = src_line.match_indices(method).find_map(|(i, _)| {
                let before = src_line.as_bytes().get(i.wrapping_sub(1)).copied();
                let after = src_line.as_bytes().get(i + method.len()).copied();
                let boundary =
                    |b: Option<u8>| b.is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'_');
                (boundary(before) && boundary(after))
                    .then(|| u32::try_from(i).expect("column fits u32"))
            });
            // The method resolves iff *its own* token is a callable `Function`.
            let resolved = mcol.is_some_and(|c| {
                toks.iter()
                    .any(|&(l, tc, _, k, _)| l == line && tc == c && k == TokenKind::Method as u32)
            });
            let ok = match expect {
                Resolve => resolved,
                Abstain => !resolved,
            };
            if !ok {
                let want = if expect == Resolve {
                    "resolve"
                } else {
                    "abstain"
                };
                failures.push(format!(
                    "  {name}: `{method}` expected to {want} but did not"
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "TclOO dispatch fixture regressions:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn cross_file_constructor_dispatch_resolves() {
        // A class defined in one file (`Pin`), dispatched on via a direct
        // constructor in *another* file — `[::Pin new] configure -node …`.
        // Resolving against a workspace-merged `ClassHierarchy` (what
        // `project_class_index` builds) lights up the method even though the
        // class is not in this file (issue #797 follow-up, the mro_eval
        // cross-file lever).
        use tcl_compiler::analyser::{Analyser, build_class_hierarchy};
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let lib = "oo::configurable create ::Pin { property node }\n";
        let hierarchy = build_class_hierarchy(Analyser::new().analyse(lib, "tcl9.0").all_classes);
        let user = "[::Pin new] configure -node 5\n";
        let cu = CompilationUnit::build_for(user, &registry, false);
        let toks = decode_semantic(&full_with_cu_and_classes(
            user,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&hierarchy),
        ));
        // `configure` (col 12, after `[::Pin new] `) resolves to a method.
        assert!(
            toks.iter().any(|&(l, c, _, k, m)| l == 0
                && c == 12
                && k == TokenKind::Method as u32
                && m == 0),
            "cross-file `[::Pin new] configure` should resolve; got {toks:?}"
        );
        // Without the hierarchy it stays an unresolved string.
        let none = decode_semantic(&full_with_cu_and_classes(
            user,
            "tcl9.0",
            &registry,
            Some(&cu),
            None,
        ));
        assert!(
            none.iter()
                .any(|&(l, c, _, k, _)| l == 0 && c == 12 && k == TokenKind::String as u32),
            "without a hierarchy, configure is a string; got {none:?}"
        );
    }

    #[test]
    fn dict_map_in_return_dispatch_resolves() {
        // `return [dict map {k v} $coll {$v method …}]` — the loop is nested in
        // a command substitution, so the IR never surfaces it as a loop; the
        // syntactic scan still binds `v` to the collection element (SpiceGenTcl
        // `getPinsNodes` / `getParams` shape, issue #797).
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { dict append Pins $p [Pin new] }\n\
                     method nodes {} { return [dict map {k pin} $Pins {$pin configure -node}] }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0),
            "expected `configure` in the return-nested dict map to resolve; got {toks:?}"
        );
    }

    #[test]
    fn user_object_handle_method_resolves() {
        // `set p [Pin new]; $p configure -node x` — a directly-bound user-class
        // handle resolves its method + property option against the ClassDef.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   set p [Pin new]\n\
                   $p configure -node 5\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            "tcl9.0",
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // `configure` at line 2 resolves to a Function.
        assert!(
            toks.iter()
                .any(|&(l, _, _, k, _)| l == 2 && k == TokenKind::Method as u32),
            "expected `configure` as a Function on the $p dispatch; got {toks:?}"
        );
    }

    #[test]
    fn regex_source_tracks_inside_namespace_eval_body() {
        // `namespace eval` bodies are lowered as their own synthetic body units,
        // so a `set re "…"; regexp $re` inside the eval highlights the def-site
        // literal — end-to-end through the CU overlay, matching a proc body.
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "namespace eval ::ns {\n  set re \".*x\"\n  regexp $re $s\n}\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        assert!(
            toks.iter()
                .any(|&(_, _, _, k, _)| k == TokenKind::RegexpQuantifier as u32),
            "expected a regex quantifier from the ns-eval body def-site literal; got {toks:?}"
        );
        for w in toks.windows(2) {
            let (l0, c0, len0, _, _) = w[0];
            let (l1, c1, _, _, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "overlap; toks={toks:?}");
            }
        }
    }

    #[test]
    fn regex_source_tracks_inside_apply_lambda_body() {
        // `apply` lambda bodies are synthetic body units too — a def-site regex
        // literal inside the lambda highlights as a regex.
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "apply {{s} {\n  set re \".*x\"\n  regexp $re $s\n}} foo\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(src, "tcl9.0", &registry, Some(&cu)));
        assert!(
            toks.iter()
                .any(|&(_, _, _, k, _)| k == TokenKind::RegexpQuantifier as u32),
            "expected a regex quantifier from the apply lambda body def-site literal; got {toks:?}"
        );
        for w in toks.windows(2) {
            let (l0, c0, len0, _, _) = w[0];
            let (l1, c1, _, _, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "overlap; toks={toks:?}");
            }
        }
    }

    /// Decode a `SemanticTokens` value directly into
    /// `(line, col, len, kind, mods)` tuples.
    fn decode_semantic(st: &SemanticTokens) -> Vec<(u32, u32, u32, u32, u32)> {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::new();
        for c in st.data.chunks(5) {
            let (dl, dc, len, kind, mods) = (c[0], c[1], c[2], c[3], c[4]);
            if dl > 0 {
                line += dl;
                col = dc;
            } else {
                col += dc;
            }
            out.push((line, col, len, kind, mods));
        }
        out
    }

    #[test]
    fn comment_inside_multiline_string_is_not_a_comment() {
        // A `#`-first line inside a multi-line `"…"` string is string text,
        // not a command comment — it must not emit a Comment token (which
        // would overlap the `$x` variable substitution).
        let src = "append s \"line1\n# not a comment $x\nline3\"\n";
        let toks = decode_full(src, "tcl", &reg());
        assert!(
            !toks
                .iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Comment as u32),
            "a `#` inside a string must not be a comment; got {toks:?}"
        );
        // A real comment still is one.
        let toks = decode_full("# real\nset x 1\n", "tcl", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Comment as u32),
            "expected a real comment token; got {toks:?}"
        );
    }

    #[test]
    fn computed_command_head_does_not_overlap() {
        // `chartV$node SetOptions …` — a command head containing a `$node`
        // substitution is a *computed* (non-static) command name, so it is not
        // painted as a single command token.  Its fragments tokenise
        // individually (`$node` as a variable) and must not overlap each other
        // (LSP clients reject overlapping semantic tokens) — issue #797.
        let toks = decode_full("chartV$node SetOptions -x {}\n", "tcl", &reg());
        for w in toks.windows(2) {
            let (l0, c0, len0, ..) = w[0];
            let (l1, c1, ..) = w[1];
            if l0 == l1 {
                assert!(
                    c1 >= c0 + len0,
                    "overlap: token at col {c1} starts before prev end {}; toks={toks:?}",
                    c0 + len0
                );
            }
        }
        // The `$node` fragment inside the head reads as a variable — the head
        // is not swallowed into one command-head token (which would hide the
        // substitution and mislabel a dynamic command as a resolved one).
        assert!(
            toks.iter()
                .any(|&(l, c, _, k, _)| l == 0 && c == 6 && k == TokenKind::Variable as u32),
            "expected a `$node` variable fragment at col 6; got {toks:?}"
        );
        // No token is a `Function` command head spanning the computed word.
        assert!(
            !toks
                .iter()
                .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
            "computed head must not emit a function command token; got {toks:?}"
        );
    }

    #[test]
    fn command_substitution_head_recurses_not_command_token() {
        // `[dict get $Pins $pin] configure -node $node` (issue #797) — the head
        // is a `[…]` command substitution, a runtime-computed command name, not
        // a resolvable command.  It must recurse into its inner script (`dict`
        // as a builtin, `get` as its subcommand, `$Pins` / `$pin` as variables)
        // rather than paint the whole `[…]` word as one function command token.
        let src = "[dict get $Pins $pin] configure -node $node\n";
        let toks = decode_full(src, "tcl", &reg());
        // `dict` inside the substitution head is a builtin function token.
        assert!(
            toks.iter().any(|&(l, c, _, k, m)| l == 0
                && c == 1
                && k == TokenKind::Function as u32
                && m == MOD_DEFAULT_LIBRARY),
            "expected `dict` builtin token at col 1; got {toks:?}"
        );
        // `$Pins` inside the head is a variable, not swallowed by a command token.
        assert!(
            toks.iter()
                .any(|&(l, c, _, k, _)| l == 0 && c == 10 && k == TokenKind::Variable as u32),
            "expected `$Pins` variable at col 10; got {toks:?}"
        );
        // Nothing paints the computed head as a function command token at col 0.
        assert!(
            !toks
                .iter()
                .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
            "the `[…]` head must not be a function command token; got {toks:?}"
        );
    }

    #[test]
    fn variable_command_head_is_a_variable() {
        // `$obj configure -node $node` — an object-handle dispatch whose class
        // is unknown.  The `$obj` head is a variable substitution, not a
        // command name, so it reads as a variable rather than a function token
        // (issue #797 / the #748 `$chart` object-dispatch shape).
        let src = "$obj configure -node $node\n";
        let toks = decode_full(src, "tcl", &reg());
        assert!(
            toks.iter()
                .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Variable as u32),
            "expected `$obj` head to read as a variable; got {toks:?}"
        );
        assert!(
            !toks
                .iter()
                .any(|&(l, c, _, k, _)| l == 0 && c == 0 && k == TokenKind::Function as u32),
            "the `$obj` head must not be a function command token; got {toks:?}"
        );
    }

    #[test]
    fn apply_lambda_body_recurses() {
        // `apply {{} { set z 3 }}` — the lambda body (list element 1) is a
        // script; `set`/`z` must tokenise rather than sit inside one string.
        let src = "apply {{} { set z 3 }}\n";
        let toks = decode_full(src, "tcl", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Function as u32),
            "expected `set` function token inside the lambda body; got {toks:?}"
        );
        assert!(
            toks.iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
            "expected `z` declared inside the lambda body; got {toks:?}"
        );
        // `apply $lambda` (a variable, not a literal) must not be recursed.
        let ks = kinds("apply $lambda a b\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Variable as u32)),
            "expected the $lambda variable token; got {ks:?}"
        );
    }

    #[test]
    fn apply_bare_arglist_param_is_a_parameter() {
        // Issue #954: `apply {dir { … }}` — the argument list is a bare,
        // unbraced single name.  Its parameter (`dir`) must highlight as a
        // `Parameter` declaration (not a `string`), and the body commands
        // must still tokenise as a script.
        let registry = reg();
        let src = "apply {dir {\n    puts $dir\n}} /tmp\n";
        assert!(
            has_token_kind(src, "tcl", &registry, "dir", TokenKind::Parameter),
            "bare arg-list param `dir` must be a Parameter declaration; got {:?}",
            decode_full(src, "tcl", &registry)
        );
        assert!(
            has_token_kind(src, "tcl", &registry, "puts", TokenKind::Function),
            "apply body command `puts` must tokenise as a Function; got {:?}",
            decode_full(src, "tcl", &registry)
        );
        // A braced arg list still emits its names as parameters.
        let braced = "apply {{a b} { expr {$a + $b} }} 1 2\n";
        assert!(
            has_token_kind(braced, "tcl", &registry, "a", TokenKind::Parameter)
                && has_token_kind(braced, "tcl", &registry, "b", TokenKind::Parameter),
            "braced arg-list params must stay parameters; got {:?}",
            decode_full(braced, "tcl", &registry)
        );
        // A computed (`$dynamic`) arg list is not painted as a parameter.
        assert!(
            !has_token_kind(
                "apply [list $al $body]\n",
                "tcl",
                &registry,
                "al",
                TokenKind::Parameter
            ),
            "a computed arg list must not be a parameter declaration"
        );
    }

    /// Issue #954, the reopened follow-up: `apply`'s lambda body is
    /// reachable indirectly through `[list apply {…} $x]`, the idiomatic way
    /// to build a deferred command around a dynamic value — most commonly a
    /// pkgIndex.tcl `package ifneeded name ver [list apply {dir {…}} $dir]`
    /// entry. TP cases: the reported repro, a namespace-qualified `::apply`,
    /// and the same idiom under a *different* enclosing Body-role command
    /// (`after idle`) to prove the fix is registry-driven (any Body-role
    /// position), not special-cased to `package ifneeded`.
    #[test]
    fn list_quoted_apply_lambda_body_recurses() {
        let registry = reg();

        // The exact reported repro: a pkgIndex.tcl-style entry.
        let pkgindex = "package ifneeded myPackage 1.0.0 [list apply {dir {\n    source [file join $dir x.tcl]\n}} $dir]\n";
        assert!(
            has_token_kind(pkgindex, "tcl", &registry, "source", TokenKind::Keyword),
            "pkgIndex-style list-quoted apply body: `source` must tokenise; got {:?}",
            decode_full(pkgindex, "tcl", &registry)
        );
        assert!(
            has_token_kind(pkgindex, "tcl", &registry, "dir", TokenKind::Parameter),
            "pkgIndex-style list-quoted apply: `dir` param must be a Parameter; got {:?}",
            decode_full(pkgindex, "tcl", &registry)
        );
        // The reconstructed command-name word itself reads as a call-site
        // reference, same as a literal head — not a plain string.
        assert!(
            has_token_kind(pkgindex, "tcl", &registry, "apply", TokenKind::Function),
            "the list-quoted `apply` word must read as a Function reference; got {:?}",
            decode_full(pkgindex, "tcl", &registry)
        );

        // A `::`-qualified spelling resolves the same way (registry `get`
        // strips a leading `::`, exactly like a direct `::apply {…}` call).
        let qualified = "package ifneeded p 1.0 [list ::apply {dir {puts $dir}} $dir]\n";
        assert!(
            has_token_kind(qualified, "tcl", &registry, "puts", TokenKind::Function),
            "qualified `::apply` list-quoted body: `puts` must tokenise; got {:?}",
            decode_full(qualified, "tcl", &registry)
        );

        // A *different* Body-role enclosing command (`after idle`) proves
        // the fix isn't specific to `package ifneeded`.
        let after_idle = "after idle [list apply {{x} {puts $x}} 5]\n";
        assert!(
            has_token_kind(after_idle, "tcl", &registry, "puts", TokenKind::Function),
            "after-idle list-quoted apply body: `puts` must tokenise; got {:?}",
            decode_full(after_idle, "tcl", &registry)
        );
        assert!(
            has_token_kind(after_idle, "tcl", &registry, "x", TokenKind::Parameter),
            "after-idle list-quoted apply: `x` param must be a Parameter; got {:?}",
            decode_full(after_idle, "tcl", &registry)
        );
    }

    /// FP guards for the `[list …]`-quoted lambda recognition: a plain data
    /// list, a list naming an unregistered head, and a dynamic `list` head
    /// must never be split as if they were an apply lambda.
    #[test]
    fn list_quoted_apply_lambda_false_positive_guards() {
        let registry = reg();

        // Ordinary data list: `list`'s own args stay whatever the default
        // classifier gives them — none of them becomes a `Parameter`
        // declaration (which only a recognised lambda-literal split emits).
        let data_list = "set data [list puts hello world]\n";
        assert!(
            !has_token_kind(data_list, "tcl", &registry, "hello", TokenKind::Parameter),
            "a plain data list must not be split as a lambda literal"
        );

        // `list`'s first argument names a command that does not carry
        // `ArgRole::LambdaLiteral` (`puts` is an ordinary command) — its
        // second argument must not be treated as a lambda body either.
        assert!(
            !has_token_kind(
                "set cb [list puts hello]\n",
                "tcl",
                &registry,
                "hello",
                TokenKind::Parameter
            ),
            "list-quoting a non-lambda-literal command must not split its trailing arg"
        );

        // An unregistered head: no crash, no spurious split.
        let unknown = "set cb [list notARealCommand {dir {source x.tcl}} $dir]\n";
        assert!(
            !has_token_kind(unknown, "tcl", &registry, "dir", TokenKind::Parameter),
            "an unresolvable list-quoted head must not be split as a lambda literal"
        );

        // A dynamic `list` head (`$cmd`) can't be resolved statically.
        let dynamic_head = "set cb [list $cmd {dir {source x.tcl}} $dir]\n";
        assert!(
            !has_token_kind(dynamic_head, "tcl", &registry, "dir", TokenKind::Parameter),
            "a dynamic list head must not be split as a lambda literal"
        );

        // `[llength $x]` — a `Cmd` substitution whose head is `llength`, not
        // `list` (no `BUILDS_COMMAND_PREFIX` trait) — must not be misread.
        assert!(
            !has_token_kind(
                "set n [llength $items]\n",
                "tcl",
                &registry,
                "items",
                TokenKind::Parameter
            ),
            "llength must never be treated as a command-quoting construct"
        );

        // Codex review of #954's follow-up: a well-formed `[list apply …]`
        // shape sitting in an *inert* argument slot (here `set`'s value,
        // which carries no `Body` / `LambdaLiteral` / `CommandPrefix` role)
        // must not be treated as a deferred invocation — `list` only ever
        // returns a value here; nothing ever invokes `apply`.
        let inert_data = "set data [list apply {x {puts $x}} value]\n";
        assert!(
            !has_token_kind(inert_data, "tcl", &registry, "x", TokenKind::Parameter),
            "an inert `[list apply …]` value must not paint its param as a \
             Parameter; got {:?}",
            decode_full(inert_data, "tcl", &registry)
        );
        assert!(
            !has_token_kind(inert_data, "tcl", &registry, "puts", TokenKind::Function),
            "an inert `[list apply …]` value must not recurse into its body \
             as executable code; got {:?}",
            decode_full(inert_data, "tcl", &registry)
        );
        assert!(
            !has_token_kind(inert_data, "tcl", &registry, "apply", TokenKind::Function),
            "an inert `[list apply …]` value's `apply` word must not read as \
             a call-site reference; got {:?}",
            decode_full(inert_data, "tcl", &registry)
        );
    }

    /// TN regression: `package ifneeded`'s script argument, when a literal
    /// braced script (no `[list …]` wrapper), is now itself recognised
    /// generically as `ArgRole::Body` — the sibling half of issue #954 (the
    /// package.ifneeded script argument carried no role at all before).
    #[test]
    fn package_ifneeded_literal_script_recurses_as_body() {
        let registry = reg();
        let src = "package ifneeded myPackage 1.0.0 {\n    source [file join $dir x.tcl]\n}\n";
        assert!(
            has_token_kind(src, "tcl", &registry, "source", TokenKind::Keyword),
            "package ifneeded literal script: `source` must tokenise; got {:?}",
            decode_full(src, "tcl", &registry)
        );
    }

    /// True if any token in `src` covers exactly `text` with kind `kind`.
    fn has_token_kind(
        src: &str,
        dialect: &str,
        registry: &CommandRegistry,
        text: &str,
        kind: TokenKind,
    ) -> bool {
        decode_full(src, dialect, registry)
            .iter()
            .any(|&(l, c, len, k, _)| {
                k == kind as u32
                    && src
                        .lines()
                        .nth(l as usize)
                        .and_then(|line| {
                            let s = line.char_indices().nth(c as usize).map(|(i, _)| i)?;
                            let e = line
                                .char_indices()
                                .nth((c + len) as usize)
                                .map_or(line.len(), |(i, _)| i);
                            line.get(s..e)
                        })
                        .is_some_and(|got| got == text)
            })
    }

    #[test]
    fn uplevel_body_recurses_as_script() {
        // Issue #837: the braced body of `uplevel ?level? {body}` runs in
        // another stack frame but is still a Tcl script — it must be
        // recursed and highlighted, not rendered as one opaque string.
        let registry = reg();

        // `uplevel 1 {body}` — literal relative level, body at arg 1.
        let src = "uplevel 1 {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "foreach", TokenKind::Keyword),
            "uplevel 1 body: `foreach` must tokenise as a keyword; got {:?}",
            decode_full(src, "tcl9.0", &registry)
        );
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "puts", TokenKind::Function),
            "uplevel 1 body: `puts` must tokenise as a function"
        );

        // `uplevel {body}` — no level, body at arg 0.
        let src = "uplevel {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "foreach", TokenKind::Keyword),
            "uplevel (no level) body: `foreach` must tokenise as a keyword"
        );

        // `uplevel #0 {body}` — absolute global level.
        let src = "uplevel #0 {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "foreach", TokenKind::Keyword),
            "uplevel #0 body: `foreach` must tokenise as a keyword"
        );

        // `uplevel $lvl {body}` — dynamic level word, body still recurses.
        let src = "uplevel $lvl {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "foreach", TokenKind::Keyword),
            "uplevel $lvl body: `foreach` must tokenise as a keyword"
        );
    }

    #[test]
    fn uplevel_dynamic_body_not_recursed() {
        // `uplevel 1 $body` — the body is a bare variable, not a braced
        // literal, so it stays a `$body` variable token (the const-lattice
        // lowering resolves it on the compiler side, not the token layer).
        let registry = reg();
        let src = "uplevel 1 $body\n";
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "$body", TokenKind::Variable),
            "uplevel 1 $body: the body variable must stay a variable token; got {:?}",
            decode_full(src, "tcl9.0", &registry)
        );
    }

    #[test]
    fn uplevel_issue_837_repro_recurses() {
        // The exact reproducer from issue #837 — a `foreach` /
        // `namespace children` / `namespace forget` body inside
        // `uplevel 1 {…}` must highlight, not sit inside one string.
        let registry = reg();
        let src = "proc forgetXyce {} {\n    uplevel 1 {foreach nameSpc [namespace children ::SpiceGenTcl::Xyce] {\n        namespace forget ${nameSpc}::*\n    }}\n}\n";
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "foreach", TokenKind::Keyword),
            "issue #837: `foreach` inside the uplevel body must be a keyword; got {:?}",
            decode_full(src, "tcl9.0", &registry)
        );
        assert!(
            has_token_kind(src, "tcl9.0", &registry, "namespace", TokenKind::Keyword)
                || has_token_kind(src, "tcl9.0", &registry, "namespace", TokenKind::Function),
            "issue #837: `namespace` inside the uplevel body must be highlighted"
        );
        // The `${nameSpc}` reference deep inside the body highlights as a
        // variable — proof the whole body was re-lexed, not stringified.
        assert!(
            decode_full(src, "tcl9.0", &registry)
                .iter()
                .any(|&(_, _, _, k, _)| k == TokenKind::Variable as u32),
            "issue #837: a variable token is expected from the recursed body"
        );
    }

    #[test]
    fn operator_command_head_classified_as_operator() {
        // `+ 3 4` — the operator head is `operator`, not `function`.
        let ks = kinds("+ 3 4\n", "tcl", &reg());
        assert_eq!(ks.first(), Some(&(TokenKind::Operator as u32)), "{ks:?}");
    }

    /// Issue #986: `is_operator_command`'s old 10-symbol hand-typed list
    /// missed every word-form comparison operator entirely — `eq 1 1` (a
    /// bare `::tcl::mathop::eq` invocation via `namespace import`) was
    /// classified as `function`, not `operator`. Also covers the TIP 461
    /// `lt`/`le`/`gt`/`ge` mathop commands (9.0+), which never had *any*
    /// classification anywhere before Phase B/D of this same unification.
    #[test]
    fn word_form_operator_command_heads_classified_as_operator() {
        for op in ["eq", "ne", "in", "ni", "lt", "le", "gt", "ge"] {
            let src = format!("{op} 1 1\n");
            let ks = kinds(&src, "tcl9.0", &reg());
            assert_eq!(
                ks.first(),
                Some(&(TokenKind::Operator as u32)),
                "{op}: {ks:?}"
            );
        }
    }

    #[test]
    fn bareword_argument_classified_as_string() {
        // `puts hello` → function head + a `string` token for the bareword
        // arg, not a dropped arg.
        let ks = kinds("puts hello\n", "tcl", &reg());
        assert_eq!(ks.len(), 2, "expected head + arg token; got {ks:?}");
        assert!(
            ks.contains(&(TokenKind::String as u32)),
            "bareword arg not classified as string; got {ks:?}"
        );
    }

    #[test]
    fn legend_includes_regexp_and_event() {
        let types = legend_token_types();
        assert_eq!(types[TokenKind::Regexp as usize], "regexp");
        assert_eq!(types[TokenKind::Event as usize], "event");
    }

    #[test]
    fn regexp_pattern_classified_as_regexp() {
        // `regexp {abc} $s` — the `{abc}` pattern argument is `regexp`,
        // not `string`.
        let ks = kinds("regexp {abc} $s\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Regexp as u32)),
            "expected a regexp token; got {ks:?}"
        );
        // `regsub -all {x+} $s y out` — option-skip finds the pattern.
        let ks = kinds("regsub -all {x+} $s y out\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Regexp as u32)),
            "expected a regexp token after -all; got {ks:?}"
        );
    }

    #[test]
    fn event_name_classified_as_event() {
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_dialect::DialectSet::IRULES);
        let ks = kinds(
            "when HTTP_REQUEST {\n  set x 1\n}\n",
            "f5-irules",
            &registry,
        );
        assert!(
            ks.contains(&(TokenKind::Event as u32)),
            "expected an event token; got {ks:?}"
        );
    }

    #[test]
    fn bigip_object_ref_token_in_irules_body() {
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_dialect::DialectSet::IRULES);
        // `pool web_pool` inside a multi-line `when` body → `object`.
        let ks = kinds(
            "when HTTP_REQUEST {\n  pool web_pool\n}\n",
            "f5-irules",
            &registry,
        );
        assert!(ks.contains(&(TokenKind::Object as u32)), "{ks:?}");
    }

    #[test]
    fn bigip_object_ref_not_emitted_in_plain_tcl() {
        // The object overlay is iRules-only.
        let ks = kinds("when HTTP_REQUEST {\n  pool web_pool\n}\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::Object as u32)), "{ks:?}");
    }

    #[test]
    fn regex_pattern_subtokenised_into_components() {
        // `(a+)+` → group `(`, literal `a`, quantifier `+`, group `)`,
        // quantifier `+`.
        let ks = kinds("regexp {(a+)+} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
        // The whole-pattern `regexp` kind is replaced by sub-tokens, but
        // the literal `a` run is still `regexp`.
        assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
    }

    #[test]
    fn regex_char_class_and_anchor_subtokens() {
        let ks = kinds("regexp {^[0-9]+$} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpAnchor as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
    }

    #[test]
    fn regex_alternation_and_escape_subtokens() {
        let ks = kinds("regexp {a\\d|b} $s\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::RegexpAlternation as u32)),
            "{ks:?}"
        );
        // `\d` is an ARE class shortcut → char class.
        assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
    }

    #[test]
    fn escape_before_multibyte_char_does_not_panic() {
        // Regression: `\<non-ASCII>` inside a string used to slice a fixed 2
        // bytes at the backslash, landing inside the multi-byte char and
        // panicking the whole semantic-tokens request. The escape must span
        // the backslash plus the full UTF-8 char.
        for src in [
            "puts \"\\é\"\n",
            "puts \"a\\你b\"\n",
            "puts \"\\€\"\n",
            "puts \"x\\é\\你\"\n",
        ] {
            let ks = kinds(src, "tcl", &reg());
            assert!(
                ks.contains(&(TokenKind::Escape as u32)),
                "expected an Escape sub-token for {src:?}, got {ks:?}",
            );
        }
    }

    #[test]
    fn scan_are_class_spans_posix_collating_equivalence_subbrackets() {
        // `[[:alpha:]]` is one bracket expression (the inner `[:alpha:]` is a
        // POSIX class whose `]` does not close the outer bracket) — the scanner
        // must span the whole thing, not stop at the first `]`.
        assert_eq!(scan_are_class(b"[[:alpha:]]", 0), Some(11));
        assert_eq!(scan_are_class(b"[[:digit:]xyz]", 0), Some(14));
        assert_eq!(scan_are_class(b"[[.ch.]]", 0), Some(8));
        assert_eq!(scan_are_class(b"[[=a=]]", 0), Some(7));
        // A plain class is unaffected; a leading literal `]` still works.
        assert_eq!(scan_are_class(b"[a-z]", 0), Some(5));
        assert_eq!(scan_are_class(b"[]a]", 0), Some(4));
        // An unterminated sub-bracket is not a token.
        assert_eq!(scan_are_class(b"[[:alpha", 0), None);
    }

    #[test]
    fn regex_posix_class_has_no_dangling_bracket_token() {
        // Before the sub-bracket fix, `[[:alpha:]]+` mis-scanned as `[[:alpha:]`
        // (char class) + a stray literal `]` + `+`. Now the whole
        // `[[:alpha:]]` is one char class and `+` its quantifier — and, per the
        // token-overlap invariant, no token may start inside another.
        let src = "regexp {[[:alpha:]]+} $s\n";
        let toks = decode_full(src, "tcl", &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::RegexpCharClass as u32),
            "expected a char-class token; got {toks:?}"
        );
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::RegexpQuantifier as u32),
            "expected the trailing `+` as a quantifier; got {toks:?}"
        );
        for w in toks.windows(2) {
            let (l0, c0, len0, _, _) = w[0];
            let (l1, c1, _, _, _) = w[1];
            if l0 == l1 {
                assert!(
                    c1 >= c0 + len0,
                    "token overlap in POSIX class; toks={toks:?}"
                );
            }
        }
    }

    #[test]
    fn regex_without_metachars_stays_single_regexp() {
        // `abc` has no metacharacters → one `regexp` token, no sub-tokens.
        let ks = kinds("regexp {abc} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
        assert!(!ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
        assert!(
            !ks.contains(&(TokenKind::RegexpQuantifier as u32)),
            "{ks:?}"
        );
    }

    #[test]
    fn classify_regex_component_maps_each_kind() {
        assert_eq!(classify_regex_component("("), TokenKind::RegexpGroup);
        assert_eq!(classify_regex_component("(?:"), TokenKind::RegexpGroup);
        assert_eq!(
            classify_regex_component("[a-z]"),
            TokenKind::RegexpCharClass
        );
        assert_eq!(classify_regex_component("\\d"), TokenKind::RegexpCharClass);
        assert_eq!(classify_regex_component("."), TokenKind::RegexpCharClass);
        assert_eq!(classify_regex_component("+"), TokenKind::RegexpQuantifier);
        assert_eq!(
            classify_regex_component("{2,3}"),
            TokenKind::RegexpQuantifier
        );
        assert_eq!(classify_regex_component("^"), TokenKind::RegexpAnchor);
        assert_eq!(classify_regex_component("\\b"), TokenKind::RegexpAnchor);
        assert_eq!(classify_regex_component("\\n"), TokenKind::RegexpEscape);
        assert_eq!(classify_regex_component("\\3"), TokenKind::RegexpBackref);
        assert_eq!(classify_regex_component("|"), TokenKind::RegexpAlternation);
    }

    #[test]
    fn sprintf_format_spec_subtokens() {
        // `format {%d}` → `%` percent, `d` spec.
        let ks = kinds("format {%d} $n\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn sprintf_flags_and_width_subtokens() {
        // `%-5.2f` → percent, `-` flag, `5` width, `.` flag, `2` width,
        // `f` spec.
        let ks = kinds("format {%-5.2f} $x\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::FormatFlag as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatWidth as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn scan_format_arg_subtokenised() {
        // `scan`'s format string is arg 2.
        let ks = kinds("scan $s {%d} a\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn format_without_specifiers_stays_string() {
        let ks = kinds("format {plain} $x\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn clock_format_subtokens() {
        // `clock format $t -format {%Y-%m-%d}` → %/letter pairs.
        let ks = kinds("clock format $t -format {%Y-%m-%d}\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
    }

    #[test]
    fn clock_locale_modifier_subtoken() {
        // `%Ey` → percent, `E` modifier, `y` spec.
        let ks = kinds("clock scan $s -format {%Ey}\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::ClockModifier as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
    }

    #[test]
    fn clock_format_without_specifiers_stays_string() {
        let ks = kinds("clock format $t -format {plain}\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn binary_format_spec_and_count_subtokens() {
        // `binary format a3 $d` (arg 2) → spec `a`, count `3`.
        let ks = kinds("binary format a3 $d\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::BinaryCount as u32)), "{ks:?}");
    }

    #[test]
    fn binary_scan_signed_modifier_and_star() {
        // `binary scan $d su r` (arg 3) → spec `s`, modifier `u`.
        let ks = kinds("binary scan $d su r\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
        // `c*` → spec `c`, `*` flag.
        let ks = kinds("binary format c* $l\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    }

    #[test]
    fn binary_signed_modifier_suppressed_in_tcl84() {
        // The `u`/`s` modifier is 8.5+, so under tcl8.4 the `u` is not a
        // binaryFlag (no signed/unsigned modifier).
        let ks = kinds("binary scan $d su r\n", "tcl8.4", &reg());
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(!ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    }

    #[test]
    fn regsub_replacement_backref_subtokens() {
        // `regsub {a} $s {\1-\&} out` → `\1` number, `\&` operator.
        let ks = kinds("regsub {a} $s {\\1-\\&} out\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::Number as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
    }

    #[test]
    fn regsub_replacement_without_backrefs_stays_string() {
        let ks = kinds("regsub {a} $s {plain} out\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
    }

    #[test]
    fn is_event_name_matches_event_shape() {
        assert!(is_event_name("HTTP_REQUEST"));
        assert!(is_event_name("CLIENT_ACCEPTED"));
        assert!(!is_event_name("lowercase"));
        assert!(!is_event_name("X")); // single char — needs 2+
    }

    #[test]
    fn semantic_tokens_are_dialect_aware_via_expand_syntax() {
        // The provider re-segments
        // under the document dialect.  In `foo {*}$x`, on 8.5+ the `{*}`
        // is the expansion operator (consumed — not a highlighted word),
        // but on 8.4 it is a literal braced string `{*}`, which adds an
        // extra `string` token.  So the packed token stream is longer on
        // 8.4.
        let src = "foo {*}$x\n";
        let on_90 = full(src, "tcl9.0", &reg()).data;
        let on_84 = full(src, "tcl8.4", &reg()).data;
        assert!(
            on_84.len() > on_90.len(),
            "8.4 keeps `{{*}}` as a highlighted string token (longer stream): \
             8.4={} 9.0={}",
            on_84.len(),
            on_90.len(),
        );
    }

    #[test]
    fn full_returns_non_empty_data_for_simple_proc() {
        let s = full("proc foo {} {}\n", "tcl", &reg());
        // Should have at least: `proc` (keyword), `foo`
        // (function), `{}` (string), `{}` (string).
        assert!(!s.data.is_empty(), "{:?}", s.data);
        // 5 ints per token.
        assert_eq!(s.data.len() % 5, 0);
    }

    #[test]
    fn diff_returns_none_for_identical_streams() {
        let a = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
        assert_eq!(diff(&a, &a), None);
    }

    #[test]
    fn diff_isolates_a_single_changed_token() {
        // Three tokens; only the middle one's type changes.
        let old = vec![
            0, 0, 4, 0, 0, /**/ 0, 5, 3, 1, 0, /**/ 1, 0, 2, 2, 0,
        ];
        let new = vec![
            0, 0, 4, 0, 0, /**/ 0, 5, 3, 4, 0, /**/ 1, 0, 2, 2, 0,
        ];
        let edit = diff(&old, &new).expect("an edit");
        // Skip the first token (5 ints), replace exactly one token.
        assert_eq!(edit.start, 5);
        assert_eq!(edit.delete_count, 5);
        assert_eq!(edit.data, vec![0, 5, 3, 4, 0]);
    }

    #[test]
    fn diff_handles_appended_token() {
        let old = vec![0, 0, 4, 0, 0];
        let new = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
        let edit = diff(&old, &new).expect("an edit");
        // Nothing deleted; one token appended after the prefix.
        assert_eq!(edit.start, 5);
        assert_eq!(edit.delete_count, 0);
        assert_eq!(edit.data, vec![0, 5, 3, 1, 0]);
    }

    #[test]
    fn diff_handles_removed_token() {
        let old = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
        let new = vec![0, 0, 4, 0, 0];
        let edit = diff(&old, &new).expect("an edit");
        // One trailing token removed, nothing spliced in.
        assert_eq!(edit.start, 5);
        assert_eq!(edit.delete_count, 5);
        assert!(edit.data.is_empty());
    }

    #[test]
    fn legend_has_expected_entries() {
        let types = legend_token_types();
        assert_eq!(types[TokenKind::Keyword as usize], "keyword");
        assert_eq!(types[TokenKind::Function as usize], "function");
        assert_eq!(types[TokenKind::Variable as usize], "variable");
        assert_eq!(types[TokenKind::String as usize], "string");
        assert_eq!(types[TokenKind::Number as usize], "number");
        assert_eq!(types[TokenKind::Comment as usize], "comment");
        assert_eq!(types[TokenKind::Namespace as usize], "namespace");
    }

    #[test]
    fn legend_modifiers_order() {
        // Order is load-bearing: `defaultLibrary` must be bit index 3.
        let mods = legend_token_modifiers();
        assert_eq!(
            mods,
            vec!["declaration", "definition", "readonly", "defaultLibrary"]
        );
        assert_eq!(MOD_DEFAULT_LIBRARY, 1 << 3);
    }

    #[test]
    fn builtin_command_head_gets_default_library_modifier() {
        // `puts` is a registry built-in classified as `function`, so its
        // head token carries the `defaultLibrary` modifier (bit 3 = 8).
        let s = full("puts hi\n", "tcl", &reg());
        assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
        assert_eq!(s.data[4], MOD_DEFAULT_LIBRARY, "{:?}", s.data);
    }

    #[test]
    fn user_proc_head_has_no_default_library_modifier() {
        // A user-defined command isn't in the registry → `function`
        // with no modifier.
        let s = full("my_custom_cmd 1 2\n", "tcl", &reg());
        assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
        assert_eq!(s.data[4], 0, "{:?}", s.data);
    }

    #[test]
    fn keyword_head_has_no_default_library_modifier() {
        // `if` is a language keyword, not a `function` — no defaultLibrary.
        let s = full("if {1} { puts hi }\n", "tcl", &reg());
        assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
        assert_eq!(s.data[4], 0, "{:?}", s.data);
    }

    #[test]
    fn keywords_classified_as_keyword() {
        let s = full("if {1} { puts hi }\n", "tcl", &reg());
        // First token's type index should be 0 (Keyword) for `if`.
        // The encoded data: [deltaLine, deltaCol, length, type, modifiers].
        assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
    }

    /// Decode the packed stream into `(line, col, len, kind)` tuples plus
    /// the covered source word (ASCII sources only — byte == utf16).
    fn decode_words(src: &str, registry: &CommandRegistry) -> Vec<(u32, u32, u32, u32, String)> {
        let st = full(src, "tcl", registry);
        let lines: Vec<&str> = src.split('\n').collect();
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::new();
        for c in st.data.chunks(5) {
            let (dl, dc, len, kind) = (c[0], c[1], c[2], c[3]);
            if dl > 0 {
                line += dl;
                col = dc;
            } else {
                col += dc;
            }
            let word = lines
                .get(line as usize)
                .and_then(|l| l.get(col as usize..(col + len) as usize))
                .unwrap_or("")
                .to_string();
            out.push((line, col, len, kind, word));
        }
        out
    }

    fn keyword_words(src: &str, registry: &CommandRegistry) -> std::collections::HashSet<String> {
        decode_words(src, registry)
            .into_iter()
            .filter(|(_, _, _, kind, _)| *kind == TokenKind::Keyword as u32)
            .map(|(_, _, _, _, word)| word)
            .collect()
    }

    #[test]
    fn if_else_elseif_are_keywords() {
        // else/elseif structural keywords highlight like `if`.
        let src = "if 1 {\n puts a\n} elseif 2 {\n puts b\n} else {\n puts c\n}";
        let kw = keyword_words(src, &reg());
        for expected in ["if", "elseif", "else"] {
            assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
        }
    }

    #[test]
    fn try_on_finally_are_keywords() {
        // try's on/trap/finally structural keywords highlight as keywords.
        let src = "try {\n set x 1\n} on error {e} {\n puts $e\n} finally {\n puts d\n}";
        let kw = keyword_words(src, &reg());
        for expected in ["try", "on", "finally"] {
            assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
        }
    }

    #[test]
    fn builtin_name_as_bareword_arg_is_string() {
        // A builtin name used as a plain dict value stays a string, not a
        // keyword — the KEYWORD role is position-aware (if/try only).
        let src = "dict set frame proc \"asasdas asd\"";
        let proc = decode_words(src, &reg())
            .into_iter()
            .find(|(_, _, _, _, word)| word == "proc")
            .expect("a `proc` token");
        assert_eq!(proc.3, TokenKind::String as u32, "{proc:?}");
    }

    #[test]
    fn quoted_structural_keyword_offsets_past_quote() {
        // A quoted `"else"` keyword marks `else`, not `"els`.
        let src = "if 0 {} \"else\" {puts ok}";
        let kw = decode_words(src, &reg())
            .into_iter()
            .find(|(_, col, _, kind, _)| *kind == TokenKind::Keyword as u32 && *col >= 8)
            .expect("a keyword token past the first word");
        assert_eq!(kw.4, "else", "{kw:?}");
    }

    #[test]
    fn comments_classified_as_comment() {
        let s = full("# this is a comment\nset x 1\n", "tcl", &reg());
        // The first token should be the comment.
        assert_eq!(s.data[3], TokenKind::Comment as u32, "{:?}", s.data);
    }

    #[test]
    fn variables_classified_as_variable() {
        let s = full("set $x 1\n", "tcl", &reg());
        // The `$x` token kind should be Variable.
        let kinds: Vec<u32> = s.data.chunks(5).map(|c| c[3]).collect();
        assert!(
            kinds.contains(&(TokenKind::Variable as u32)),
            "expected Variable in kinds; got {kinds:?}",
        );
    }

    #[test]
    fn is_number_literal_recognises_integers_and_floats() {
        assert!(is_number_literal("42"));
        assert!(is_number_literal("-7"));
        assert!(is_number_literal("3.14"));
        assert!(is_number_literal("0xff"));
        assert!(is_number_literal("0b1010"));
        assert!(!is_number_literal("abc"));
        assert!(!is_number_literal(""));
        assert!(!is_number_literal("1.2.3"));
    }

    #[test]
    fn empty_source_returns_empty_data() {
        assert!(full("", "tcl", &reg()).data.is_empty());
    }

    #[test]
    fn semantic_token_lengths_use_utf16_code_units() {
        let data = full("# 😀x\n", "tcl", &reg()).data;
        assert_eq!(
            &data[..5],
            &[0, 0, 5, TokenKind::Comment as u32, 0],
            "comment token length must count the emoji as two UTF-16 code units",
        );
    }

    #[test]
    fn many_comment_lines_do_not_drift_out_of_bounds() {
        // Regression: `push_comment_tokens` hand-incremented a byte cursor to
        // the end of each comment line while the `chars()` iterator only
        // advanced one char, so the cursor drifted past the buffer and sliced
        // out of bounds (panic) on files with several comment lines.
        use std::fmt::Write as _;
        let mut src = String::new();
        for i in 0..40 {
            let _ = writeln!(src, "# comment line number {i} with some padding text");
        }
        src.push_str("set x 1\n");
        src.push_str("# trailing comment after code, no final newline");
        let st = full(&src, "tcl", &reg()); // must not panic
        let comments = st
            .data
            .chunks(5)
            .filter(|c| c[3] == TokenKind::Comment as u32)
            .count();
        assert_eq!(comments, 41, "expected one token per comment line");
    }

    /// A `CommandHead` for a head with nothing bound about it — the written
    /// spelling is its own identity.
    fn plain_head(name: &str) -> CommandHead<'_> {
        CommandHead {
            tok: Token {
                kind: TokenType::Esc,
                span: tcl_lexer::Span::new(0, u32::try_from(name.len()).unwrap_or(0)),
                content_offset: 0,
                in_quote: false,
            },
            text: name,
            resolved: name,
            rebound: false,
        }
    }

    #[test]
    fn classify_command_head_picks_namespace_for_qualified() {
        assert_eq!(
            classify_command_head(plain_head("::myns::greet"), &reg()),
            TokenKind::Namespace,
        );
        assert_eq!(
            classify_command_head(plain_head("greet"), &reg()),
            TokenKind::Function
        );
        assert_eq!(
            classify_command_head(plain_head("if"), &reg()),
            TokenKind::Keyword
        );
    }

    /// The keyword / operator tests key off the head's *effective identity*,
    /// so a proven alias of a keyword is a keyword and a rebound head is not
    /// (issue #1185).
    #[test]
    fn classify_command_head_follows_the_effective_identity() {
        let r = reg();
        let aliased = CommandHead {
            resolved: "foreach",
            ..plain_head("myforeach")
        };
        assert_eq!(classify_command_head(aliased, &r), TokenKind::Keyword);
        let rebound = CommandHead {
            resolved: "",
            rebound: true,
            ..plain_head("foreach")
        };
        assert_eq!(classify_command_head(rebound, &r), TokenKind::Function);
    }

    // range variant

    #[test]
    fn range_filters_tokens_outside_window() {
        // Three commands on three lines.  Range covers only
        // line 1 — the line-0 and line-2 tokens should drop.
        let src = "set a 1\nset b 2\nset c 3\n";
        let full_data = full(src, "tcl", &reg());
        let line1_only = range(
            src,
            "tcl",
            crate::definition::LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 10,
            },
            &reg(),
        );
        // Each tcl line emits at least one classified token.
        // The range result must be strictly smaller than the
        // full result.
        assert!(line1_only.data.len() < full_data.data.len());
        assert!(line1_only.data.len().is_multiple_of(5));
        assert!(!line1_only.data.is_empty(), "{:?}", line1_only.data);
    }

    #[test]
    fn range_keeps_entire_document_when_range_covers_it() {
        let src = "proc foo {} { puts hi }\n";
        let full_data = full(src, "tcl", &reg());
        let wide = range(
            src,
            "tcl",
            crate::definition::LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 99,
                end_character: 0,
            },
            &reg(),
        );
        assert_eq!(wide.data, full_data.data);
    }

    #[test]
    fn range_excludes_token_at_exact_end_position() {
        // Regression: LSP ranges are
        // half-open [start, end), so a token starting exactly
        // at `end` is OUTSIDE the range.
        let src = "set a 1\nset b 2\n";
        // Range whose end exactly coincides with line 1, col 0
        // (the `set` of the second command).  That token should
        // not appear in the range result.
        let r = range(
            src,
            "tcl",
            crate::definition::LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 1,
                end_character: 0,
            },
            &reg(),
        );
        // The full document has at least one line-1 token at col
        // 0 (the `set` of `set b 2`).  The half-open range must
        // exclude it; the range data must therefore be strictly
        // shorter than the full data.
        let full_data = full(src, "tcl", &reg());
        assert!(
            r.data.len() < full_data.data.len(),
            "range data {} should drop the line-1 token; full data {}",
            r.data.len(),
            full_data.data.len(),
        );
    }

    /// Every semantic-token type the server advertises in [`legend_token_types`]
    /// must be handled by the VS Code extension — either a standard LSP type VS
    /// Code themes natively, or an explicit `semanticTokenScopes` mapping. This
    /// is the alignment guard: add a token type to the legend (e.g. from richer
    /// lexing) without wiring the editor, and this test fails instead of the
    /// token silently rendering as an unstyled default in every theme.
    /// The BIG-IP config token types: emitted only by `bigip_conf_full` for
    /// `tcl-bigip` documents (a `bigip.conf` is not Tcl), so the Tcl-family
    /// blocks are not required to map them — `tcl-bigip` is.  `object` is
    /// deliberately **not** in this set: it is shared, typing BIG-IP object
    /// references inside iRules too, so the Tcl blocks must keep mapping it.
    const BIGIP_ONLY: &[&str] = &[
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
    ];

    /// Record a failure for every token in `required` that `mapped` (a
    /// language's `semanticTokenScopes` block) does not handle.
    fn require_mapped(
        lang: &str,
        required: &[&str],
        mapped: &std::collections::BTreeSet<&str>,
        failures: &mut Vec<String>,
    ) {
        for tok in required {
            if !mapped.contains(tok) {
                failures.push(format!(
                    "language `{lang}` does not handle legend token `{tok}` \
                     (add it to contributes.semanticTokenScopes in \
                     editors/vscode/package.json)"
                ));
            }
        }
    }

    #[test]
    fn vscode_semantic_token_scopes_cover_the_server_legend() {
        // Standard LSP `SemanticTokenTypes` VS Code styles out of the box, so
        // they need no custom `semanticTokenScopes` entry.
        const STANDARD_LSP_TYPES: &[&str] = &[
            "namespace",
            "type",
            "class",
            "enum",
            "interface",
            "struct",
            "typeParameter",
            "parameter",
            "variable",
            "property",
            "enumMember",
            "event",
            "function",
            "method",
            "macro",
            "keyword",
            "modifier",
            "comment",
            "string",
            "number",
            "regexp",
            "operator",
            "decorator",
        ];

        // The Tcl-family languages that exercise the *full* legend — iRules /
        // iApps / BIG-IP code references BIG-IP `object`s and fires `event`s, so
        // these blocks must map every non-standard token type. The narrower
        // dialects (`tcl8.4`, EDA, `expect`) emit a subset and the bespoke
        // `tcl-apl` uses its own token set, so they are not required to cover
        // the whole legend here. `tcl-irule` is a superset of plain Tcl, so
        // covering it covers every token a plain `.tcl` file can emit too.
        const FULL_VOCAB: &[&str] = &["tcl", "tcl-irule", "tcl-iapp", "tcl-bigip"];

        // The `apl*` types are emitted *only* by `apl_full`, for `tcl-apl`
        // documents (APL is not Tcl). They are therefore not required of the
        // Tcl-family blocks above — but `tcl-apl` must map every one of them,
        // which is checked separately below.
        let apl_types: Vec<&str> = legend_token_types()
            .into_iter()
            .filter(|t| t.starts_with("apl"))
            .collect();
        assert!(!apl_types.is_empty(), "legend lost its apl* token types");
        for tok in BIGIP_ONLY {
            assert!(
                legend_token_types().contains(tok),
                "legend lost the BIG-IP token type `{tok}`"
            );
        }

        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let pkg = manifest.join("../../editors/vscode/package.json");
        let text = std::fs::read_to_string(&pkg)
            .unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
        let json: serde_json::Value =
            serde_json::from_str(&text).expect("package.json is valid JSON");

        let blocks = json["contributes"]["semanticTokenScopes"]
            .as_array()
            .expect("contributes.semanticTokenScopes is an array");

        let legend = legend_token_types();
        let mut failures = Vec::new();
        let mut checked_blocks = 0;

        let mut checked_apl = false;
        let mut checked_bigip = false;

        for block in blocks {
            let lang = block["language"].as_str().unwrap_or_default();
            let is_full_vocab = FULL_VOCAB.contains(&lang);
            if !is_full_vocab && lang != "tcl-apl" {
                continue;
            }
            let mapped: std::collections::BTreeSet<&str> = block["scopes"]
                .as_object()
                .map(|m| m.keys().map(String::as_str).collect())
                .unwrap_or_default();

            // `tcl-apl` owns the `apl*` types and nothing else needs them.
            if lang == "tcl-apl" {
                checked_apl = true;
                require_mapped(lang, &apl_types, &mapped, &mut failures);
                continue;
            }

            checked_blocks += 1;
            // `tcl-bigip` additionally owns the config-file types.
            if lang == "tcl-bigip" {
                checked_bigip = true;
                require_mapped(lang, BIGIP_ONLY, &mapped, &mut failures);
            }
            let shared: Vec<&str> = legend
                .iter()
                .copied()
                .filter(|t| {
                    !STANDARD_LSP_TYPES.contains(t)
                        && !apl_types.contains(t)
                        && !BIGIP_ONLY.contains(t)
                })
                .collect();
            require_mapped(lang, &shared, &mapped, &mut failures);
        }

        assert!(
            checked_blocks > 0,
            "found no tcl* semanticTokenScopes blocks to check"
        );
        assert!(
            checked_apl,
            "found no `tcl-apl` semanticTokenScopes block to check the apl* types against"
        );
        assert!(
            checked_bigip,
            "found no `tcl-bigip` semanticTokenScopes block to check the BIG-IP types against"
        );
    }

    /// The *narrow* dialects — the versioned Tcls, the EDA tools, Expect — must
    /// map the **shared** custom vocabulary too.
    ///
    /// A `.exp` / EDA / `tcl8.4` document reaches exactly the same
    /// regex / format / clock / binary / escape sub-tokenisers a plain `.tcl`
    /// file does, so a type missing from *their* scope block renders unstyled
    /// there while looking perfectly fine in `.tcl` — the failure mode this
    /// whole test family exists to prevent, just one dialect over.
    #[test]
    fn vscode_semantic_token_scopes_cover_the_narrow_dialects() {
        const FULL_VOCAB: &[&str] = &["tcl", "tcl-irule", "tcl-iapp", "tcl-bigip"];
        const STANDARD_LSP_TYPES: &[&str] = &[
            "namespace",
            "type",
            "class",
            "enum",
            "interface",
            "struct",
            "typeParameter",
            "parameter",
            "variable",
            "property",
            "enumMember",
            "event",
            "function",
            "method",
            "macro",
            "keyword",
            "modifier",
            "comment",
            "string",
            "number",
            "regexp",
            "operator",
            "decorator",
        ];

        // `object` names a BIG-IP object, which only iRules / iApps / BIG-IP
        // config reach — a plain Tcl or EDA document never emits it.
        let shared_custom: Vec<&str> = legend_token_types()
            .into_iter()
            .filter(|t| {
                !STANDARD_LSP_TYPES.contains(t)
                    && !t.starts_with("apl")
                    && !BIGIP_ONLY.contains(t)
                    && *t != "object"
            })
            .collect();
        assert!(
            !shared_custom.is_empty(),
            "legend lost its shared vocabulary"
        );

        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let pkg = manifest.join("../../editors/vscode/package.json");
        let text = std::fs::read_to_string(&pkg)
            .unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
        let json: serde_json::Value =
            serde_json::from_str(&text).expect("package.json is valid JSON");
        let blocks = json["contributes"]["semanticTokenScopes"]
            .as_array()
            .expect("contributes.semanticTokenScopes is an array");

        let mut failures = Vec::new();
        let mut checked = 0;
        for block in blocks {
            let lang = block["language"].as_str().unwrap_or_default();
            if !lang.starts_with("tcl") || FULL_VOCAB.contains(&lang) || lang == "tcl-apl" {
                continue;
            }
            checked += 1;
            let mapped: std::collections::BTreeSet<&str> = block["scopes"]
                .as_object()
                .map(|m| m.keys().map(String::as_str).collect())
                .unwrap_or_default();
            require_mapped(lang, &shared_custom, &mapped, &mut failures);
        }
        assert!(
            checked > 0,
            "found no narrow tcl* semanticTokenScopes blocks (tcl8.4 / EDA / expect) to check"
        );
        assert!(
            failures.is_empty(),
            "narrow-dialect scope gaps:\n  {}",
            failures.join("\n  ")
        );

        assert!(
            failures.is_empty(),
            "semantic-token legend not fully handled:\n  {}",
            failures.join("\n  ")
        );
    }

    // Issue #1185 — semantic tokens read command grammar from the registry,
    // so the explicitly global spellings C Tcl resolves to the same commands
    // (`namespace which -command ::format` → `::format`) are classified
    // identically to their bare forms, and a same-named user proc is not.

    /// The decoded token *kinds* of `src`, positions discarded.
    fn kinds_only(src: &str, registry: &CommandRegistry) -> Vec<u32> {
        decode_full(src, "tcl", registry)
            .into_iter()
            .map(|(_, _, _, kind, _)| kind)
            .collect()
    }

    /// Assert that qualifying a command head with a leading `::` changes
    /// nothing but the head itself: the qualified stream must *end with* the
    /// bare stream's kinds (the qualified head contributes one extra
    /// namespace-separator token at the front), so every argument is
    /// classified identically.
    fn assert_qualified_matches_bare(
        bare: &str,
        head: &str,
        qualified: &str,
        registry: &CommandRegistry,
    ) {
        let bare_kinds = kinds_only(bare, registry);
        let qualified_kinds = kinds_only(&bare.replacen(head, qualified, 1), registry);
        assert!(
            qualified_kinds.len() >= bare_kinds.len(),
            "{qualified} produced fewer tokens than {head}: \
             {qualified_kinds:?} vs {bare_kinds:?}"
        );
        let tail = &qualified_kinds[qualified_kinds.len() - bare_kinds.len()..];
        assert_eq!(
            tail,
            &bare_kinds[..],
            "{qualified} classified its arguments differently from {head}"
        );
    }

    /// The loop-variable-list scan (issue #1185 residual 2) is registry-driven,
    /// so a `::`-qualified head classifies exactly like the bare one.
    ///
    /// tclsh-proof (9.0.4): `::foreach {a b} {1 2} { puts $a }` and
    /// `::dict for {k v} {a 1} { puts $v }` run identically to their bare forms —
    /// `namespace which -command ::foreach` is `::foreach`.
    #[test]
    fn qualified_loop_headers_classify_like_the_bare_form() {
        let r = reg();
        for (bare, head, qualified) in [
            ("foreach {a b} {1 2} { puts $a }\n", "foreach", "::foreach"),
            ("lmap x {1 2} { expr {$x} }\n", "lmap", "::lmap"),
            ("dict for {k v} {a 1} { puts $v }\n", "dict", "::dict"),
            ("dict map {k v} {a 1} { set v }\n", "dict", "::dict"),
        ] {
            assert_qualified_matches_bare(bare, head, qualified, &r);
        }
    }

    /// The loop-variable binding pass reads the registry's own
    /// `ArgRole::LoopVarList` indices, so every loop header it understands is
    /// one the registry declares — not a `foreach` / `lmap` / `dict for`
    /// spelling list in the walker.
    #[test]
    fn loop_var_list_roles_come_from_the_registry() {
        let r = reg();
        let roles = |cmd: &str, args: &[&str]| {
            r.arg_indices_for_role(cmd, args, tcl_registry::ArgRole::LoopVarList)
        };
        // `foreach VARS LIST ?VARS LIST …? BODY`: the repeated (start 0,
        // stride 2) layout marks every variable-list word.
        assert_eq!(roles("foreach", &["{a b}", "$l", "{}"]), vec![0]);
        assert_eq!(
            roles("foreach", &["{a}", "$l", "{b}", "$m", "{}"]),
            vec![0, 2]
        );
        assert_eq!(roles("lmap", &["x", "$l", "{}"]), vec![0]);
        // `dict for {k v} $d body` marks its pair at index 1 (after the
        // subcommand word), and the collection at index 2 is declared a Dict —
        // which is what tells the binder this is a key/value pair rather than
        // a free variable list.
        assert_eq!(roles("dict", &["for", "{k v}", "$d", "{}"]), vec![1]);
        assert_eq!(roles("dict", &["map", "{k v}", "$d", "{}"]), vec![1]);
        assert_eq!(
            r.arg_type_hint("dict", &["for", "{k v}", "$d", "{}"], 2)
                .and_then(|h| h.expected),
            Some(tcl_registry::types::TclType::Dict)
        );
        // A `::`-qualified head resolves to the same spec, so it reports the
        // same roles — the point of issue #1185.
        assert_eq!(roles("::foreach", &["{a b}", "$l", "{}"]), vec![0]);
        assert_eq!(roles("::dict", &["for", "{k v}", "$d", "{}"]), vec![1]);
        // A command with no loop-variable list reports none.
        assert!(roles("puts", &["hi"]).is_empty());
    }

    #[test]
    fn loop_var_binding_uses_tcl_list_grammar_and_abstains_when_dynamic() {
        use tcl_compiler::segmenter::segment_commands;

        let names = |src: &str| {
            let command = segment_commands(src)
                .into_iter()
                .next()
                .expect("one command");
            static_loop_var_names(src, &command, 1)
        };

        assert_eq!(
            names("foreach {a b} $coll {}\n"),
            Some(vec![String::from("a"), String::from("b")])
        );
        assert_eq!(
            names("foreach \"a b\" $coll {}\n"),
            Some(vec![String::from("a"), String::from("b")])
        );
        assert_eq!(
            names("foreach a\\ b $coll {}\n"),
            Some(vec![String::from("a b")])
        );
        assert_eq!(names("foreach {} $coll {}\n"), Some(Vec::new()));
        assert_eq!(names("foreach $vars $coll {}\n"), None);
        assert_eq!(names("foreach [makeVars] $coll {}\n"), None);
        assert_eq!(names("foreach {a b $coll {}\n"), None);
    }

    #[test]
    fn qualified_format_family_classifies_like_the_bare_form() {
        let r = reg();
        for (bare, head, qualified) in [
            ("format {%08x} 42\n", "format", "::format"),
            ("scan $s {%d %d} a b\n", "scan", "::scan"),
            ("binary format c3 {1 2 3}\n", "binary", "::binary"),
            ("binary scan $v c3 out\n", "binary", "::binary"),
            ("clock format $t -format {%Y-%m-%d}\n", "clock", "::clock"),
            ("clock scan $s -format {%Y}\n", "clock", "::clock"),
            ("regsub -all e $s {[&]} out\n", "regsub", "::regsub"),
        ] {
            assert_qualified_matches_bare(bare, head, qualified, &r);
        }
    }

    #[test]
    fn qualified_grammar_commands_classify_like_the_bare_form() {
        let r = reg();
        for (bare, head, qualified) in [
            ("foreach {a b} {1 2} { puts $a }\n", "foreach", "::foreach"),
            ("lmap x $l { expr {$x} }\n", "lmap", "::lmap"),
            ("upvar 1 src local\n", "upvar", "::upvar"),
            ("upvar src local\n", "upvar", "::upvar"),
            ("global aa bb cc\n", "global", "::global"),
            ("variable x 1 y 2\n", "variable", "::variable"),
            (
                "oo::define C method m {args} { return $args }\n",
                "oo::define",
                "::oo::define",
            ),
            (
                "oo::objdefine $o method m {} { return }\n",
                "oo::objdefine",
                "::oo::objdefine",
            ),
            (
                "namespace upvar ::ns o1 l1 o2 l2\n",
                "namespace",
                "::namespace",
            ),
            ("dict update d k1 v1 k2 v2 { puts $v1 }\n", "dict", "::dict"),
        ] {
            assert_qualified_matches_bare(bare, head, qualified, &r);
        }
    }

    /// FP guard — a user proc in another namespace that happens to share a
    /// built-in's tail name does not inherit its grammar, and data words that
    /// merely look like format strings are not painted as ones.
    #[test]
    fn same_named_user_command_does_not_inherit_grammar() {
        let r = reg();
        // `ns::format` is a different command; its argument stays a plain
        // string, so no format-specifier sub-tokens appear.
        let user = decode_full("ns::format {%08x} 42\n", "tcl", &r);
        let builtin = decode_full("format {%08x} 42\n", "tcl", &r);
        assert!(
            builtin.len() > user.len(),
            "the built-in must produce specifier sub-tokens the user proc does not:\n\
             builtin={builtin:?}\nuser={user:?}"
        );
        // A data word that spells a format string is untouched.
        assert_eq!(
            kinds_only("puts {%08x}\n", &r),
            kinds_only("puts {plain}\n", &r)
        );
    }

    // Issue #1185 residual 1 — a head's *effective command identity* (a static
    // `interp alias`, a `rename`, a shadowing top-level `proc`) drives the
    // grammar, so calling a built-in through a proven alias classifies exactly
    // like calling it directly, and calling a name whose binding was taken
    // over does not.

    /// TP — `interp alias {} myfmt {} format` makes `myfmt %08x 42` classify
    /// exactly like `format %08x 42`.
    ///
    /// tclsh-proof (9.0.4 and 8.6.16, byte-identical): `interp alias {} myfmt
    /// {} format; myfmt %08x 42` → `0000002a`.
    #[test]
    fn a_static_interp_alias_inherits_the_targets_grammar() {
        let r = reg();
        let bind = "interp alias {} myfmt {} format\n";
        // The argument kinds of a direct `format` call — its head token is the
        // one thing an aliased or qualified spelling legitimately differs in.
        let direct = kinds_only("format {%08x} 42\n", &r);
        let direct_args = &direct[1..];
        for head in ["myfmt", "::myfmt"] {
            let call = kinds_only(&format!("{bind}{head} {{%08x}} 42\n"), &r);
            assert_eq!(
                &call[call.len() - direct_args.len()..],
                direct_args,
                "`{head}` must classify its arguments like a direct `format` call"
            );
        }
        // Without the alias the same call is an ordinary unknown command whose
        // argument stays a plain string.
        let unaliased = kinds_only("myfmt {%08x} 42\n", &r);
        assert!(
            unaliased.len() < direct.len(),
            "an unbound name must not get format sub-tokens: {unaliased:?}"
        );
    }

    /// TP — `rename foreach myforeach` moves the loop grammar (and the
    /// keyword classification) onto the new name.
    #[test]
    fn a_static_rename_moves_the_grammar_to_the_new_name() {
        let r = reg();
        let bind = "rename upvar myupvar\n";
        let direct = kinds_only("upvar 1 src local\n", &r);
        let renamed = kinds_only(&format!("{bind}myupvar 1 src local\n"), &r);
        let baseline = kinds_only(bind, &r);
        assert_eq!(
            &renamed[baseline.len()..],
            &direct[..],
            "a renamed call must classify like the original"
        );
    }

    /// FP — a name whose binding was taken over gets **no** registry grammar:
    /// after `rename format origfmt` the bare `format` is gone, and a top-level
    /// `proc format` shadows the built-in outright.
    ///
    /// tclsh-proof (9.0.4 and 8.6.16): `rename format origfmt; proc format
    /// {args} {return USER}; format x` → `USER`; `origfmt %d 7` → `7`.
    #[test]
    fn a_rebound_builtin_loses_its_grammar() {
        let r = reg();
        let direct = kinds_only("format {%08x} 42\n", &r);
        for bind in [
            "rename format origfmt\n",
            "rename format {}\n",
            "proc format {args} { return USER }\n",
            "interp alias {} format {} myformatter\n",
        ] {
            let baseline = kinds_only(bind, &r);
            let after = kinds_only(&format!("{bind}format {{%08x}} 42\n"), &r);
            assert!(
                after.len() - baseline.len() < direct.len(),
                "`{}` must strip format's specifier sub-tokens, got {after:?}",
                bind.trim()
            );
        }
    }

    /// FN guard — a binding only applies from its own statement onwards, so a
    /// call *before* the `rename` still classifies as the built-in.
    #[test]
    fn a_binding_does_not_retroactively_retag_earlier_calls() {
        let r = reg();
        let direct = kinds_only("format {%08x} 42\n", &r);
        let before = kinds_only("format {%08x} 42\nrename format origfmt\n", &r);
        let rename_only = kinds_only("rename format origfmt\n", &r);
        assert_eq!(
            &before[..direct.len()],
            &direct[..],
            "the call before the rename must keep the built-in's grammar"
        );
        assert_eq!(&before[direct.len()..], &rename_only[..]);
    }

    /// TN — a binding the analyser cannot prove states nothing, so the head
    /// keeps its literal identity rather than gaining a wrong one.
    #[test]
    fn unprovable_bindings_abstain() {
        let r = reg();
        let plain = kinds_only("myfmt {%08x} 42\n", &r);
        for bind in [
            // Dynamic alias target / name, dynamic rename source.
            "interp alias {} myfmt {} $target\n",
            "interp alias {} $n {} format\n",
            "rename $old myfmt\n",
            // Pre-bound arguments shift every index.
            "interp alias {} myfmt {} format %08x\n",
            // A child interpreter's command table is not this one's.
            "interp alias slave myfmt {} format\n",
            // Not an unconditional top-level statement.
            "if {$x} { interp alias {} myfmt {} format }\n",
        ] {
            let baseline = kinds_only(bind, &r);
            let after = kinds_only(&format!("{bind}myfmt {{%08x}} 42\n"), &r);
            assert_eq!(
                &after[baseline.len()..],
                &plain[..],
                "`{}` must not give `myfmt` a grammar",
                bind.trim()
            );
        }
    }

    /// FP — a `proc` in another namespace, or one that shadows nothing, states
    /// no fact: only a global-namespace redefinition of a built-in takes a name
    /// over (tclsh 9.0.4: inside `namespace eval ::n`, `proc format` defines
    /// `::n::format` and the global `format` is untouched).
    #[test]
    fn a_namespaced_proc_does_not_shadow_the_global_builtin() {
        let r = reg();
        let direct = kinds_only("format {%08x} 42\n", &r);
        let bind = "namespace eval ::n { proc format {a} { return 1 } }\n";
        let baseline = kinds_only(bind, &r);
        let after = kinds_only(&format!("{bind}format {{%08x}} 42\n"), &r);
        assert_eq!(&after[baseline.len()..], &direct[..]);
    }

    /// TP — `apply`'s `ArgRole::LambdaLiteral` reaches the renamed and aliased
    /// spellings too, closing the failure mode written up in the
    /// apply-lambda-body KCS note: a `[list …]`-quoted or directly-called
    /// lambda under `rename apply myapply` used to collapse into one opaque
    /// `string` token.
    ///
    /// tclsh-proof (9.0.4 / 8.6.16): `rename apply myapply; myapply {x {puts
    /// $x}} 5` prints `5`, exactly as the literal call does.
    #[test]
    fn apply_lambda_literals_survive_a_rename_or_alias() {
        let r = reg();
        let direct = kinds_only("apply {x {puts $x}} 5\n", &r);
        for bind in [
            "rename apply myapply\n",
            "interp alias {} myapply {} apply\n",
        ] {
            let baseline = kinds_only(bind, &r);
            let bound = kinds_only(&format!("{bind}myapply {{x {{puts $x}}}} 5\n"), &r);
            assert_eq!(
                &bound[baseline.len()..],
                &direct[..],
                "`{}` must keep apply's lambda-literal split",
                bind.trim()
            );
        }
    }

    /// TN — a `{*}`-expanded (dynamic) head has no resolvable identity, so no
    /// grammar is applied to its arguments.
    #[test]
    fn dynamic_head_gets_no_format_grammar() {
        let r = reg();
        let dynamic = decode_full("{*}$cmd {%08x} 42\n", "tcl", &r);
        let builtin = decode_full("format {%08x} 42\n", "tcl", &r);
        assert!(
            dynamic.len() < builtin.len(),
            "a dynamic head must not get format sub-tokens: {dynamic:?}"
        );
    }

    /// Issue #862: `set`, `lassign`, `incr`, `lappend`, `append`, `expr` (every
    /// plain builtin — `function` + `defaultLibrary`) rendered as unstyled
    /// plain text for users whose theme has no rule for the custom
    /// `support.function.tcl` scope. A `semanticTokenScopes` override was
    /// mapping `function.defaultLibrary` to that scope, which **replaces**
    /// (not supplements) VS Code's built-in cross-theme default for the
    /// standard `function`/`defaultLibrary` combo — so themes lacking that
    /// exact scope lost highlighting entirely instead of falling back to the
    /// built-in default the way every other standard type does. `operator`,
    /// `decorator` and `namespace` carried the same risk for the same reason
    /// (and `operator`'s scope, `keyword.operator.format.tcl`, was outright
    /// wrong for the general case — it covers every `expr` operator and the
    /// `regsub` `\&` backref, not just `format`). Standard LSP types get no
    /// override unless the override is either essentially universal across
    /// themes (`number`, `regexp` — near-ubiquitous `TextMate` scopes that
    /// match the grammar's own naming) or the type has no sane built-in
    /// default at all (custom types like `object`, `event`, `escape`, the
    /// `regexp*`/`format*`/`clock*`/`binary*` families).
    #[test]
    fn vscode_semantic_token_scopes_do_not_shadow_standard_defaults() {
        const MUST_NOT_OVERRIDE: &[&str] = &[
            "function.defaultLibrary",
            "operator",
            "decorator",
            "namespace",
        ];

        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let pkg = manifest.join("../../editors/vscode/package.json");
        let text = std::fs::read_to_string(&pkg)
            .unwrap_or_else(|e| panic!("reading {}: {e}", pkg.display()));
        let json: serde_json::Value =
            serde_json::from_str(&text).expect("package.json is valid JSON");

        let blocks = json["contributes"]["semanticTokenScopes"]
            .as_array()
            .expect("contributes.semanticTokenScopes is an array");

        let mut failures = Vec::new();
        for block in blocks {
            let lang = block["language"].as_str().unwrap_or_default();
            let Some(scopes) = block["scopes"].as_object() else {
                continue;
            };
            for &key in MUST_NOT_OVERRIDE {
                if scopes.contains_key(key) {
                    failures.push(format!(
                        "language `{lang}` overrides `{key}`, shadowing VS Code's \
                         built-in cross-theme default (issue #862) — remove it from \
                         contributes.semanticTokenScopes in editors/vscode/package.json"
                    ));
                }
            }
        }

        assert!(failures.is_empty(), "{}", failures.join("\n  "));
    }
}
