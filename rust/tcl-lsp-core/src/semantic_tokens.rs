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
use tcl_dialect::model::SurfaceQuery;
use tcl_registry::CommandRegistry;
use tcl_registry::definer::{DefinitionBodyGrammar, MemberKind};

mod original;

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
        for part in parts {
            original.extend(part.original.iter().cloned());
            write.absorb(&part.write, &part.ambiguous.write);
            read.absorb(&part.read, &part.ambiguous.read);
            command.absorb(&part.command, &part.ambiguous.command);
        }
        let mut result = Self::from_builders(write, read, command);
        result.original = original;
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

    for (bstart, bend) in tcl_bigip::conf_tokens::embedded_rule_bodies(source) {
        let (bstart, bend) = (bstart as usize, bend as usize);
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

/// The process-wide iRules command store, for the dialect-independent
/// retained `when EVENT` source roles — the `f5-irules`
/// environment's registry generation, resolved through the one ingress seam
/// and memoised so the per-command fallback lookup skips the generation
/// cache's mutex.
fn irules_registry() -> &'static CommandRegistry {
    static IRULES: std::sync::OnceLock<&'static CommandRegistry> = std::sync::OnceLock::new();
    IRULES.get_or_init(|| crate::registry_for_dialect("f5-irules"))
}

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
    identities: &tcl_compiler::realm::CommandBindingRealm,
) -> FxHashSet<u32> {
    let mut heads: FxHashSet<u32> =
        tcl_registry::events::top_level_when_handler_candidates_with_registry_and_head_resolver(
            source, registry, identities,
        )
        .into_iter()
        .map(|handler| handler.span.start())
        .collect();

    // Keep the dialect-independent generic-Tcl fallback for built-in iRules
    // event handlers.  The active registry carries workspace extensions, but
    // a generic registry does not include `when`; dropping the process-wide
    // iRules candidates here regresses the existing `when EVENT` colouring in
    // plain Tcl buffers.
    heads.extend(
        tcl_registry::events::top_level_when_handler_candidates_with_registry_and_head_resolver(
            source,
            irules_registry(),
            identities,
        )
        .into_iter()
        .map(|handler| handler.span.start()),
    );

    // The shared syntax boundary parser intentionally models `when`'s
    // handler-specific grammar.  Procedure declarations use the same
    // top-level lexer stream and the registry's declaration-shape owner.
    // Deliberately the iRules grammar, whatever the document's dialect: this
    // is the generic-Tcl fallback that keeps `when EVENT` colouring in a plain
    // Tcl buffer. The grammar still comes from the ingress-resolved
    // `f5-irules` environment rather than a by-name lexer preset.
    for seg in segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_file_grammar(
            crate::environment_for_dialect("f5-irules").grammar(),
        ),
    ) {
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
    CaseList(tcl_registry::CaseListSpec, bool),
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
fn insert_oo_body_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    oo_grammar: Option<&'static DefinitionBodyGrammar>,
    arg_texts: &[&str],
    dialect: Option<SurfaceQuery<'_>>,
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
/// one-liner form silently losing its name / parameters / body and getting only
/// its keyword marked.
fn insert_oo_member_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    grammar: &'static DefinitionBodyGrammar,
    head: &str,
    arg_texts: &[&str],
    base: usize,
    dialect: Option<SurfaceQuery<'_>>,
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
    // carries this role; consuming it is what stops a method name painting as
    // a plain string.
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

/// Bareword instance-command name → qualified class name,
/// built from [`AnalysisResult::instance_classes`] gated on
/// [`AnalysisResult::created_instance_commands`]. Only positive Logical
/// classification consumes this compatibility projection. Original source
/// roles and positioned receiver identity retain their separate owners.
pub type NamedInstanceMap = std::collections::HashMap<String, String>;

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
    analysis
        .instance_classes
        .iter()
        .filter(|(name, _)| analysis.created_instance_commands.contains(name.as_str()))
        .map(|(name, class)| (name.clone(), class.clone()))
        .collect()
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
#[allow(clippy::too_many_arguments)] // one object-dispatch classifier threading resolved context
fn insert_object_method_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    object_classes: &ObjectClassMap,
    object_collections: &ObjectClassMap,
    classes: Option<&ClassHierarchy>,
    document_floor: Option<crate::document_floor::DocumentFloor<'_>>,
    dialect: Option<SurfaceQuery<'_>>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let (Some(head_tok), Some(head_text), Some(method)) =
        (seg.argv.first(), seg.texts.first(), seg.texts.get(1))
    else {
        return;
    };
    // Candidate receiver classes implied by the head's shape: a `$var` object
    // handle, a direct `[Class new] …` constructor, a `[dict get $coll $k]`
    // / `[lindex $coll $i]` retrieval from an object collection,
    // or a bareword instance-command name bound by a positional create call
    // (`ttk::treeview .t` / a registry naming factory; `object_classes`
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

/// The element classes of a collection-*retrieval* command head — a
/// single-level `[dict get $coll $key]` or `[lindex $coll $idx]` — looked up in
/// the object-collection map, or `None` when the head is not such a retrieval
/// or the collection is not tracked.  Resolves the receiver of a
/// `[dict get $Pins $pin] configure -node …` dispatch.
///
/// Which calls retrieve an element, and from which argument, is registry data
/// ([`tcl_registry::types::ReturnElements::ElementOf`], read through
/// [`CommandRegistry::resolve_call`] exactly as the compiler's type inference
/// reads it) — so `::lindex` and `::dict get` resolve like their bare
/// spellings, and no command name is matched here.
fn collection_head_element_classes<'a>(
    head_text: &str,
    registry: &CommandRegistry,
    object_collections: &'a ObjectClassMap,
) -> Option<&'a std::collections::HashSet<String>> {
    use tcl_registry::types::ReturnElements;

    let (cmd, args) = tcl_compiler::value_shapes::parse_command_substitution_with_config(
        head_text,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let resolved = registry.resolve_call(&cmd, &arg_refs, None)?;
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

/// The bare handle name of a `$var` command head, matching the keys of an
/// [`ObjectClassMap`]: strips the leading `$` and any `${…}` braces, so
/// `$chart` → `chart`, `${chart}` → `chart`, `$arr(k)` → `arr(k)`.  Returns
/// `None` for a head that is not a plain variable substitution.
fn object_handle_name(head_text: &str) -> Option<&str> {
    if !head_text.starts_with('$') {
        return None;
    }
    // `var_reference` does not unwrap an unclosed `${x`, preventing a
    // malformed receiver from resolving to the real handle `x`.
    Some(tcl_syntax::naming::var_reference(head_text))
}

/// The registry class named by a direct manufacturer command-head dispatch,
/// or `None` when the head is not such a constructor call.
fn constructor_class_of_head<'r>(
    head_text: &str,
    registry: &'r CommandRegistry,
) -> Option<&'r str> {
    let (cmd, args) = tcl_compiler::value_shapes::parse_command_substitution_with_config(
        head_text,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )?;
    registry.exported_manufacturer_method(&cmd, args.first()?)?;
    registry.object_class(&cmd).map(|c| c.class_name)
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
        let implicit_self = seg
            .texts
            .first()
            .and_then(|text| object_handle_name(text))
            .is_some_and(|name| {
                matches!(name, "self" | "this")
                    && ctx
                        .oo_grammar
                        .is_some_and(|grammar| grammar.implicit_vars.contains(&name))
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
    let (cmd, args) = tcl_compiler::value_shapes::parse_command_substitution_with_config(
        head_text,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )?;
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
    resolve_class_in_hierarchy(hierarchy, &cmd)
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
/// through the selected procedure role index. A **written** target retags as a
/// `Variable` declaration; a **read** reference as a plain `Variable` (it names
/// an existing variable, not a new one).  The only remaining question is token
/// geometry.  A word that lexes as a single unquoted [`TokenType::Esc`] token —
/// a scalar (`x`), a literal array element (`arr(key)`), or a namespaced name
/// (`::ns::arr(key)`) — is retagged as one whole-word token, matching how the
/// `$arr(key)` read highlights.  A word with an inner substitution
/// (`arr($i)`, `$dynamic`) is multi-token (`single_token_word` is `false`), so
/// it is left to the default classifier and its inner `$var` sub-tokens survive.
fn retag_variable_argument(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    i: usize,
    ov: ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let Some(word) = seg.argv.get(i + 1) else {
        return;
    };
    if seg.single_token_word.get(i + 1) == Some(&true) {
        // `Str` — a brace-quoted word — is a variable *name* here just as
        // much as a bareword is: braces suppress every substitution, so
        // `set {$n} 1` declares the variable literally called `$n` and
        // `[set {$n}]` reads it (tclsh 9.0.4 / 8.6.14: `info exists {$n}`
        // → 1 while `info exists n` → 0).  Falling through would paint the
        // word as a plain `string`, hiding a declaration and inviting the
        // reader to see the `$n` inside as a substitution.
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
    // stays multi-token, and skipping it would let its literal fragments
    // fall through to the default classification and paint as `string` —
    // pervasive in Tcl's own `init.tcl` / `package.tcl`.
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
}

/// A command name passed as an argument — the registry `CommandPrefix` role
/// (`tk selection … -command`, a stub `:command_prefix`), or a proc parameter
/// the analyser inferred to be a `Command` (`$cmd` used as a head, or flowing
/// into a command-name position).  Retag the literal at the
/// call site as a `Function`, gated by the same single-token `Esc` geometry as
/// the variable retag, so `dispatch mycmd …` paints `mycmd` as a command.
fn retag_command_argument(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    i: usize,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if let Some(tok) = seg.argv.get(i + 1)
        && seg.single_token_word.get(i + 1) == Some(&true)
        && matches!(tok.kind, TokenType::Esc)
        && !tok.in_quote
    {
        overrides
            .entry(tok.span.start())
            .or_insert(ArgOverride::CommandRef);
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
    lexical: bool,
    /// The dialect's numeric-literal grammar, resolved once from `dialect` so
    /// the per-word `Number` classification does not re-walk the profile
    /// catalogue for every token.  Decides which radix prefixes exist, whether a
    /// bare leading zero is octal, and whether `_` digit separators are
    /// allowed — see [`is_number_literal`].
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
    /// Genuine source-only definition parent. Native source vocabulary does
    /// not come from rendered head/argument strings or a nominal grammar.
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
    /// The document's statically proven command-identity facts — which
    /// registry command each head spelling really names at each point in the
    /// file.  Folds together `namespace import` (`test` → `tcltest::test`),
    /// `interp alias`, static `rename`, and a top-level `proc` that shadows a
    /// built-in.  Every fact is offset-keyed, so
    /// a binding cannot retroactively re-tag an earlier call; every shape that
    /// cannot be proven leaves the head alone.  Empty for a document that
    /// binds nothing.  See [`tcl_compiler::realm`].
    head_identities: &'a tcl_compiler::realm::CommandBindingRealm,
    /// Object-handle → class-name provenance for the whole document, so a
    /// `$var method …` dispatch can resolve the method's options through the
    /// registry's object-class model.  Empty when no
    /// [`CompilationUnit`] is available or the document creates no tracked
    /// object handles.
    object_classes: &'a ObjectClassMap,
    /// Object-*collection* variable → element class, so a `[dict get $coll $k]`
    /// / `[lindex $coll $i]` retrieval used as a command head resolves the
    /// element's method.  Empty without a [`CompilationUnit`].
    object_collections: &'a ObjectClassMap,
    /// Class hierarchy, when available — the MRO + `ClassDef`s (methods +
    /// `oo::configurable` properties) used to resolve a dispatched method
    /// against a *user* class, not just a registry-modelled one.  This is the
    /// current file's hierarchy, or a workspace-merged project index so a class
    /// defined in another file resolves too.  `None` for
    /// the pure-segmentation path.
    classes: Option<&'a ClassHierarchy>,
    /// The local analysis when this request has one, used only for document
    /// package floors on lifecycle-gated registry methods.
    analysis: Option<&'a AnalysisResult>,
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
) {
    if !ctx.lexical
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
            let text = inner.get(p.start..p.end).unwrap_or_default();
            push_case_pattern(
                ctx,
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

/// The head's *effective command identity*, resolved once so every
/// registry-driven pass in [`collect_script`] keys off it.
///
/// Covers a command imported from an exported namespace (`namespace import
/// tcltest::*` → `test` = `tcltest::test`), a static `interp alias`
/// / `rename`, and a top-level `proc` that shadows a built-in.  Facts are
/// offset-keyed, so a binding never retroactively re-tags an earlier call, and
/// a head with nothing proven about it keeps its own spelling.
///
/// The overwhelmingly common document binds nothing, so the lookup is skipped
/// entirely rather than hashing every head in the file.
fn realm_head_binding_of<'a>(
    ctx: ScriptCtx<'a>,
    head_text: &'a str,
    head_tok: Token,
) -> tcl_compiler::realm::RealmBinding<'a> {
    if ctx.head_identities.is_empty() {
        return tcl_compiler::realm::RealmBinding::Command(head_text);
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
/// than a misleading command highlight.
fn emit_static_command_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    entries: &mut Vec<Entry>,
) {
    let Some(&head_tok) = seg.argv.first() else {
        return;
    };
    let kind = if ctx.original_roles.is_class_head(head_tok.span.start()) {
        TokenKind::Class
    } else if original::definition_member_head(ctx, seg) {
        TokenKind::Keyword
    } else {
        TokenKind::Function
    };
    // A missing source schema carries no built-in or namespace-name authority.
    push_token(ctx.line_index, ctx.full_source, head_tok, kind, 0, entries);
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
    let full_source = ctx.full_source;
    let definition_members = (!ctx.lexical)
        .then(|| {
            let parent = ctx.original_definition_parent?;
            let end = base_offset.checked_add(u32::try_from(text.len()).ok()?)?;
            parent.definition_member_region(ctx.generation, Span::new(base_offset, end))
        })
        .flatten();
    let ctx = ScriptCtx {
        original_definition_members: definition_members.as_ref(),
        ..ctx
    };
    for seg in
        segment_commands_with_offset_and_config(text, base_offset, ctx.config.at_depth(depth))
    {
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
        let identity = if ctx.lexical {
            realm_head_binding_of(ctx, head_text, head_tok)
        } else {
            tcl_compiler::realm::RealmBinding::Rebound
        };
        let resolved_head: &str = if ctx.lexical {
            identity.spec_name()
        } else {
            original.as_ref().map_or("", |words| words.command.as_str())
        };
        let member_head = (!ctx.lexical)
            .then(|| original::definition_member_head_word(ctx, &seg))
            .flatten()
            .and_then(|word| Some((word, word.content_span().ok()?)));
        // A complete original member word can prove static spelling even when
        // its physical escape tokens form several lexical fragments.
        let computed_head = member_head.is_none()
            && head_is_computed(&seg)
            && !original
                .as_ref()
                .is_some_and(|words| original::original_logical_head(ctx, words).is_some());
        let static_head_extent = member_head.map_or(head_tok.span, |(word, _)| word.word_span());
        if !computed_head {
            if ctx.lexical && original.is_none() {
                emit_static_command_head(ctx, &seg, entries);
            } else if let Some((_, span)) = member_head {
                push_token(
                    ctx.line_index,
                    full_source,
                    Token { span, ..head_tok },
                    TokenKind::Keyword,
                    0,
                    entries,
                );
            } else if ctx.original_roles.is_class_head(head_tok.span.start()) {
                push_token(
                    ctx.line_index,
                    full_source,
                    head_tok,
                    TokenKind::Class,
                    0,
                    entries,
                );
            } else if let Some(words) = &original {
                original::registry_head(ctx, &seg, words, entries);
            } else {
                push_token(
                    ctx.line_index,
                    full_source,
                    head_tok,
                    TokenKind::Function,
                    0,
                    entries,
                );
            }
        }

        // The command's argument words (head excluded), borrowed once as
        // `&[&str]` and shared by every registry-driven pass below — the
        // override builder and the OO-body context check both need it, and
        // the registry API takes `&[&str]`, so building it here keeps the
        // hot path to a single bridging allocation per command.
        let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();

        let member_bodies = (!ctx.lexical)
            .then(|| original::definition_member_bodies(ctx, &seg))
            .flatten();
        let original_bodies = original
            .as_ref()
            .map_or_else(Vec::new, |words| words.source_script_bodies(ctx.generation));
        let mut overrides = original.as_ref().map_or_else(FxHashMap::default, |words| {
            original::registry_overrides(ctx, &seg, words)
        });
        for body in member_bodies.iter().flatten() {
            if let [token] = body.original_container().tokens() {
                overrides.insert(token.span.start(), ArgOverride::BodyScript);
            }
        }
        if ctx.lexical {
            insert_lexical_user_overrides(ctx, &seg, resolved_head, &arg_texts, &mut overrides);
        }
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
        let produced = deferred_role
            .then(|| {
                let analysis = ctx.analysis?;
                crate::original_invocation::source_produced_command_prefix_words(
                    full_source,
                    analysis,
                    &seg,
                )
            })
            .flatten();
        if let Some(produced) = &produced {
            merge_list_quoted_command_overrides(&seg, ctx, produced, &mut overrides);
        }
        // `my method …` inside a class body resolves against the enclosing
        // class's MRO (the most common `TclOO` dispatch form).
        if !ctx.original_roles.has_head(head_tok.span.start()) {
            insert_self_method_overrides(ctx, &seg, original.as_ref(), &mut overrides);
        }
        ctx.original_roles
            .insert_overrides(ctx, &seg, &mut overrides);
        // Regex-source tracking: retag a `set` value word that feeds a regexp
        // pattern as a (substitution-aware) regex.
        if ctx.lexical {
            mark_regex_source_words(&seg, ctx.regex_sources, &mut overrides);
        }

        // Logical compatibility selects its own reported-string grammar. Native
        // source bodies instead carry a genuine parent per original operand,
        // through the shared source-body owner below.
        let outer_grammar = original.as_ref().and_then(|words| {
            words
                .with_source_schema(ctx.generation, |schema| {
                    schema.authored_source_definition_body_grammar()
                })
                .flatten()
        });
        let next_oo = if ctx.lexical {
            next_original_definition_grammar(ctx, head_text, &arg_texts, outer_grammar)
        } else {
            None
        };
        // The class whose body the recursion enters: a `oo::class create NAME`
        // (and the property-/instantiation-metaclasses) names it at argv[2], an
        // `oo::define NAME { … }` at argv[1].  Slice it from the source so it
        // outlives the walk; otherwise inherit the enclosing class (so a
        // `method …` body keeps its class).  Lets `my method …` in the body
        // resolve against that class.
        let next_class = if ctx.lexical {
            original
                .as_ref()
                .and_then(|words| original::definition_class_name(ctx, &seg, words))
                .or(ctx.enclosing_class)
        } else {
            None
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
            .map(|body| body.environment())
            .or(ctx.scoped_env);
        let body_ctx = ScriptCtx {
            oo_grammar: next_oo,
            enclosing_class: next_class,
            scoped_env: next_scoped,
            ..ctx
        };

        let mut deferred_role_starts = deferred_role_arg_starts(original.as_ref());
        if let Some(declared) = &declared {
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

        let mut expanded_roles = original
            .as_ref()
            .map_or_else(Vec::new, |words| original::expanded_role_extents(words));
        expanded_roles.extend(procedure_role_extents);
        for tok in &seg.all_tokens {
            if original::emit_expanded_role_extents(ctx, *tok, &expanded_roles, entries) {
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
                    .or_else(|| ctx.original_roles.operand_at(tok.span)),
                original_words: produced.as_ref().or(original.as_ref()),
                declared_words: declared.as_ref(),
                ..ctx
            };
            let definition_parent = if ctx.lexical {
                None
            } else if let Some(member_body) = member_bodies.iter().flatten().find(|body| {
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
}

/// User-procedure traits and entered OO source grammar are independent of
/// builtin source descriptors. They cannot recover a refused builtin head.
fn insert_lexical_user_overrides(
    ctx: ScriptCtx<'_>,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    head: &str,
    arguments: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    insert_object_method_overrides(
        command,
        ctx.registry,
        ctx.object_classes,
        ctx.object_collections,
        if ctx.original_roles.has_head(command.argv[0].span.start()) {
            None
        } else {
            ctx.classes
        },
        ctx.analysis
            .map(|analysis| crate::document_floor::DocumentFloor::new(analysis, ctx.dialect)),
        Some(ctx.context.authoring_query()),
        overrides,
    );
    insert_oo_body_overrides(
        command,
        ctx.oo_grammar,
        arguments,
        Some(ctx.context.authoring_query()),
        overrides,
    );
    let Some(procedure_roles) = ctx.proc_roles else {
        return;
    };
    for (roles, kind) in [
        (&procedure_roles.write, ArgOverride::VarDecl),
        (&procedure_roles.read, ArgOverride::VarRef),
    ] {
        if let Some(indices) = roles.get(head) {
            for &index in indices {
                retag_variable_argument(command, index as usize, kind, overrides);
            }
        }
    }
    if let Some(indices) = procedure_roles.command.get(head) {
        for &index in indices {
            retag_command_argument(command, index as usize, overrides);
        }
    }
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
    use tcl_syntax::formal_params::FormalByteArgumentBinding;
    let Some(analysis) = ctx.analysis.filter(|_| !ctx.lexical) else {
        return Vec::new();
    };
    let Some(prototype) =
        tcl_compiler::registry_invocation::source_structure::original_procedure_arguments(
            ctx.full_source,
            analysis,
            seg,
        )
    else {
        return Vec::new();
    };
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
    let mut extents = Vec::new();
    for (parameters, kind, modifier, override_kind) in [
        (
            &roles.write,
            TokenKind::Variable,
            MOD_DECLARATION,
            ArgOverride::VarDecl,
        ),
        (&roles.read, TokenKind::Variable, 0, ArgOverride::VarRef),
        (
            &roles.command,
            TokenKind::Function,
            0,
            ArgOverride::CommandRef,
        ),
    ] {
        for binding in prototype.bindings() {
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
            if !u32::try_from(parameter)
                .ok()
                .is_some_and(|index| parameters.contains(&index))
            {
                continue;
            }
            let Some(operand) = prototype.operands().get(argument).and_then(Option::as_ref) else {
                continue;
            };
            let Some(input) = operand.input() else {
                continue;
            };
            if prototype
                .arguments()
                .get(argument)
                .and_then(|value| value.literal_bytes())
                != Some(input.bytes())
            {
                continue;
            }
            if let Some(word) = operand.word() {
                if word
                    .tokens()
                    .iter()
                    .any(|token| matches!(token.kind, TokenType::Cmd | TokenType::Var))
                {
                    continue;
                }
                let Some(token) = seg.argv.iter().find(|token| token.span == operand.span()) else {
                    continue;
                };
                if matches!(token.kind, TokenType::Esc | TokenType::Str) {
                    overrides.entry(token.span.start()).or_insert(override_kind);
                }
            } else {
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
    // Logical compatibility clears its nominal member grammar here. Original
    // substitutions retain only the genuine same-source vocabulary parent;
    // a fresh region issuer checks their whole argv independently.
    let plain_ctx = ScriptCtx {
        oo_grammar: None,
        original_definition_members: None,
        declared_words: None,
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
    } else if let Some(kind) = classify_arg_token(tok, full_source, ctx.numbers) {
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
        if let Some(kind) = classify_arg_token(tok, full_source, ctx.numbers) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let math = tcl_lexer::expr_math_functions();
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
            E::Function if !et.text.is_empty() && !et.text.contains('\n') => {
                let pos = line_index
                    .position_at_utf16(u32::try_from(abs_start).unwrap_or(0), full_source);
                let mods = if ctx.lexical {
                    u32::from(math.contains(et.text.as_str())) * MOD_DEFAULT_LIBRARY
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
                    command,
                    &words,
                    &context,
                    rules,
                    object_collections,
                    object_classes,
                );
            }
            true
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
            bind_original_handle_construction(source, analysis, command, &context, object_classes);
            true
        },
    );
    let Some(walk) = crate::refactor::FrameWalk::new(source, analysis) else {
        return;
    };
    for (region, grammar) in selected_member_handle_regions(source, analysis, &context) {
        let Some(text) = source.get(region.as_range()) else {
            continue;
        };
        for root in walk.segment(text, region.start()) {
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
                && declaration.logical_source_class(analysis).is_some()
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
            true
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

/// A selected setter and current constructor graph retain independent source
/// producers; no displayed bracket text supplies a receiver or stored value.
fn bind_original_handle_construction(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_compiler::segmenter::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
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
        return;
    };
    let Some(class) = call.class_declaration().logical_source_class(analysis) else {
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
    command: &tcl_compiler::segmenter::SegmentedCommand,
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    rules: tcl_syntax::word_rules::WordValueRules,
    collections: &ObjectClassMap,
    handles: &mut ObjectClassMap,
) {
    if command.is_partial {
        return;
    }
    let Some(roles) = words.roles() else {
        return;
    };
    for &(ordinal, role) in roles {
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
        let Some(classes) = command
            .texts
            .get(*written)
            .and_then(|text| object_handle_name(text))
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
        named_instances,
        analysis,
    } = facts;
    let fresh_analysis;
    let analysis = if let Some(analysis) = analysis {
        analysis
    } else {
        let context = tcl_registry::model::context_for_profile(dialect);
        let context =
            std::sync::Arc::new(context.with_command_store(registry.snapshot().shared_registry()));
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            dialect,
            dialect,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(dialect.grammar),
        );
        fresh_analysis = tcl_compiler::analyser::Analyser::new()
            .structure_only()
            .with_resolved_input(input)
            .analyse(source, dialect.name);
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
    let lexical = analysis.allows_lexical_declaration_advice();
    let analysis = Some(analysis);
    let mut entries: Vec<Entry> = Vec::new();
    let line_index = LineIndex::new(source);
    let profile = dialect;

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
    // `rename` / built-in-shadowing `proc`.  Empty (no lookups)
    // unless the document actually binds something.
    let Some(head_identities) = analysis.and_then(AnalysisResult::retained_command_realm) else {
        return Vec::new();
    };

    // The iRules declaration overlay uses the shared top-level boundary facts
    // (including offset-resolved command identity) rather than this walk's
    // recursive depth as its placement predicate.
    let irules_top_level_declaration_heads = if lexical {
        irules_top_level_declaration_heads(source, registry, head_identities)
    } else {
        FxHashSet::default()
    };

    // Object-handle → class provenance (`set chart [ticklecharts::chart new]`
    // → `chart`), so a `$chart Xaxis -name …` dispatch resolves the method's
    // options through the registry.  Empty without a
    // `CompilationUnit` or when the document creates no tracked object handles.
    let mut object_classes: ObjectClassMap = cu.map_or_else(ObjectClassMap::default, |cu| {
        tcl_compiler::object_types::object_handle_classes(cu, registry)
    });

    // Logical object classification shares this compatibility handle map.
    // Native source roles never recover receiver identity from its labels.
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
    if lexical {
        augment_loop_var_handles(
            source,
            analysis.unwrap(),
            &object_collections,
            &mut object_classes,
        );
    }

    // snit object-handle bindings the compiler CFG doesn't surface — `install
    // NAME using TYPE` components and `set NAME [Type inst]` bare constructors —
    // via a source scan, since snit method bodies (where these live) are not
    // lowered into the CFG `object_handle_classes` reads.
    if lexical {
        augment_snit_handles(source, analysis.unwrap(), classes, &mut object_classes);
    }

    // Walk every segmented command (recursing into braced bodies, braced
    // expressions, and `[…]` command substitutions) and classify each token.
    let original_roles = original::OriginalTokenRoles::capture(source, analysis);
    let ctx = ScriptCtx {
        full_source: source,
        dialect,
        config,
        context: generation.context(),
        generation: &generation,
        operand: None,
        original_words: None,
        declared_words: None,
        lexical,
        numbers: profile.grammar.numbers,
        registry,
        line_index: &line_index,
        // The root of a document is itself a definition body when the dialect
        // says so: a `.sslictcl` document's legal top-level words are exactly
        // its document grammar's members, painted from membership like every
        // level below. An ordinary Tcl document has no such grammar.
        oo_grammar: if lexical {
            registry.document_grammar()
        } else {
            None
        },
        original_definition_parent: None,
        original_definition_members: None,
        scoped_env: None,
        regex_sources: &regex_sources,
        head_identities,
        object_classes: &object_classes,
        object_collections: &object_collections,
        classes,
        analysis,
        original_roles: &original_roles,
        proc_roles,
        enclosing_class: None,
        irules_top_level_declaration_heads: &irules_top_level_declaration_heads,
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
    if lexical && profile.is_irules() {
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
            if is_number_literal(text, numbers) {
                Some(TokenKind::Number)
            } else if text.contains("::") && tok.content_offset == 0 {
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

/// `true` when the whole of `text` is a Tcl number literal under `numbers` —
/// the release's numeric-literal grammar, which decides which radix prefixes
/// exist (`0x` always, `0o`/`0b` from 8.5, `0d` from 9.0), whether a bare
/// leading zero is octal (up to 8.6) or decimal (9.0+), and whether `_` digit
/// separators are allowed (9.0+).
///
/// This is the same question C's `ParseLexeme` asks before classifying a word
/// as a `NUMBER`, so it routes through the one shared numeral facility rather
/// than a private recogniser: a hand-rolled copy here painted `0o17` as a
/// bareword, accepted `0x_` and `--5`, and applied 9.0's `_` separators to
/// every release.
fn is_number_literal(text: &str, numbers: NumberSyntax) -> bool {
    tcl_syntax::number::is_whole_number(text, numbers)
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
            // next physical line, matching Tcl's parser.  An even
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
            // client would reject.
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
        // Tcl and the TextMate grammar.  A `;` inside a
        // string / braced literal is harmless here: the `#` it exposes is
        // already covered by that literal's tokens and suppressed above.
        if c == ';' {
            line_start = true;
            continue;
        }
        line_start = false;
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
    use tcl_dialect::model::{Family, SurfaceLayer};

    /// The plain-Tcl profile these tests tokenise under. A named helper keeps
    /// the 140-odd call sites readable — spelling the resolution out at each
    /// one buries the assertion it belongs to.
    fn tcl() -> &'static tcl_dialect::DialectProfile {
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()
    }

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn expect_reg() -> CommandRegistry {
        let mut registry = reg();
        registry.load_surface(SurfaceLayer::Package("expect"));
        registry
    }

    fn kinds(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
        registry: &CommandRegistry,
    ) -> Vec<u32> {
        full(src, dialect, registry)
            .data
            .chunks(5)
            .map(|c| c[3])
            .collect()
    }

    /// Decode the packed stream into absolute `(line, col, len)` triples.
    fn decode(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
        registry: &CommandRegistry,
    ) -> Vec<(u32, u32, u32)> {
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
        dialect: &'static tcl_dialect::DialectProfile,
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
        let toks = decode(src, tcl(), registry);
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
        // The lexer's empty-content clamp spans the opening `"` fragment over
        // `"$`, overlapping the `$x` variable token; the opening fragment must
        // shrink to just the `"`.
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
        let toks = decode("puts \"$x y\"\n", tcl(), &reg());
        // The opening `"` lands at byte/col 5 on line 0 with length 1.
        assert!(
            toks.contains(&(0, 5, 1)),
            "expected a length-1 string token at col 5, got {toks:?}",
        );
    }

    #[test]
    fn expect_canonical_and_abbreviated_regex_flags_tokenise_in_both_shapes() {
        for (label, source, flag) in [
            (
                "braced canonical",
                "expect {\n    -regexp {a+} {\n        puts matched\n    }\n}\n",
                "-regexp",
            ),
            (
                "braced abbreviation",
                "expect {\n    -re {a+} {\n        puts matched\n    }\n}\n",
                "-re",
            ),
            (
                "inline canonical",
                "expect -regexp {a+} {\n    puts matched\n    puts again\n}\n",
                "-regexp",
            ),
            (
                "inline abbreviation",
                "expect -re {a+} {\n    puts matched\n    puts again\n}\n",
                "-re",
            ),
        ] {
            let tokens = decode_full(
                source,
                tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
                &expect_reg(),
            );
            let flag_len = u32::try_from(flag.len()).unwrap();
            assert!(
                tokens.iter().any(|&(_, _, len, kind, _)| {
                    len == flag_len && kind == TokenKind::Decorator as u32
                }),
                "{label}: {flag} must be a clause decorator: {tokens:?}"
            );
            assert!(
                tokens
                    .iter()
                    .any(|&(_, _, _, kind, _)| kind == TokenKind::RegexpQuantifier as u32),
                "{label}: {flag} must enable regexp tokenisation: {tokens:?}"
            );
        }
    }

    #[test]
    fn expect_literal_comment_and_separator_patterns_recurse_into_actions() {
        for pattern in ["#", ";"] {
            let source = format!("expect {{{pattern} {{puts matched}} default {{puts other}}}}\n");
            let tokens = decode_full(
                &source,
                tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
                &expect_reg(),
            );
            let puts_col = u32::try_from(source.find("puts matched").unwrap()).unwrap();
            assert!(
                tokens.iter().any(|&(line, col, len, kind, _)| {
                    line == 0 && col == puts_col && len == 4 && kind == TokenKind::Function as u32
                }),
                "{pattern:?} is a literal list pattern, and its action must be tokenised: {tokens:?}"
            );
        }
    }

    #[test]
    fn malformed_case_lists_do_not_tokenise_action_words_as_commands() {
        let source = "switch subject {a {puts hidden} orphan}\n";
        let tokens = decode_full(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &reg(),
        );
        let puts_col = u32::try_from(source.find("puts hidden").unwrap()).unwrap();
        assert!(
            !tokens.iter().any(|&(line, col, len, kind, _)| {
                line == 0 && col == puts_col && len == 4 && kind == TokenKind::Function as u32
            }),
            "semantic tokens must abstain for an odd case list: {tokens:?}"
        );
    }

    #[test]
    fn renamed_away_expect_spelling_gets_no_case_list_overrides() {
        let source = "rename expect other\nexpect -re {a+} { puts matched }\n";
        let kinds = kinds(
            source,
            tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
            &expect_reg(),
        );
        assert!(
            !kinds.contains(&(TokenKind::RegexpQuantifier as u32)),
            "a proven rebound head must not inherit Expect overrides: {kinds:?}"
        );
    }

    #[test]
    fn known_option_classified_as_decorator() {
        // `regexp -nocase {pat} $s` — `-nocase` is a real option → decorator.
        let ks = kinds("regexp -nocase {pat} $s\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
        // `puts -foo` — `-foo` is not an option of `puts` → not a decorator.
        let ks = kinds("puts -foo\n", tcl(), &reg());
        assert!(!ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
    }

    #[test]
    fn abbreviated_option_classified_as_decorator() {
        // Tcl option parsing accepts unique prefixes: `lsort -inc` ⇒
        // `-increasing`, `lsearch -ex` ⇒ `-exact`.
        for src in ["lsort -inc {3 1 2}\n", "lsearch -ex {a b} b\n"] {
            let ks = kinds(src, tcl(), &reg());
            assert!(
                ks.contains(&(TokenKind::Decorator as u32)),
                "expected decorator for {src:?}; got {ks:?}"
            );
        }
        // An ambiguous prefix (`lsort -i` → -index/-indices/-integer/…) is not
        // a recognised option and stays a string.
        let ks = kinds("lsort -i {3 1 2}\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "ambiguous prefix must not be a decorator; got {ks:?}"
        );
    }

    #[test]
    fn lsearch_regexp_pattern_uses_the_registry_selected_language() {
        let ks = kinds("lsearch -regexp {a b} {a+}\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
        // The default glob form must not acquire regex subtokens merely
        // because lsearch also supports -regexp.
        let glob = kinds("lsearch {a b} {a+}\n", tcl(), &reg());
        assert!(!glob.contains(&(TokenKind::Regexp as u32)), "{glob:?}");
    }

    #[test]
    fn lsearch_pattern_tokens_abstain_for_invalid_profiled_option_prefixes() {
        // A dash-prefixed word inside lsearch's outer option scan is not a
        // positional list operand when it cannot resolve in that release.
        // In each invalid call the old fallback would put the regex override
        // on `{a+}`; the registry must instead abstain entirely.
        let registry = reg();
        let kinds_at_claimed_pattern = |source: &str, dialect: &str| {
            let claimed_pattern =
                u32::try_from(source.find("a+").expect("claimed pattern")).unwrap();
            let tokens = full(
                source,
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                &registry,
            );
            let mut line = 0u32;
            let mut column = 0u32;
            tokens
                .data
                .chunks(5)
                .filter_map(|chunk| {
                    if chunk[0] > 0 {
                        line += chunk[0];
                        column = chunk[1];
                    } else {
                        column += chunk[1];
                    }
                    (line == 0 && column <= claimed_pattern && claimed_pattern < column + chunk[2])
                        .then_some(chunk[3])
                })
                .collect::<Vec<_>>()
        };

        for (dialect, option) in [("tcl8.6", "-str"), ("tcl9.0", "-st"), ("tcl9.0", "--")] {
            let source = format!("lsearch -regexp {option} {{a+}} {{a b}} x\n");
            let kinds = kinds_at_claimed_pattern(&source, dialect);
            assert!(
                !kinds.contains(&(TokenKind::Regexp as u32)),
                "{dialect} {option} is invalid inside the option scan: {kinds:?}"
            );
        }

        // Tcl 9's unique -str abbreviation does consume its stride value,
        // so the real final pattern remains regex-classified.
        let source = "lsearch -regexp -str 2 {a b} {a+}\n";
        let kinds = kinds_at_claimed_pattern(source, "tcl9.0");
        assert!(
            kinds.contains(&(TokenKind::Regexp as u32)),
            "tcl9.0 must parse -str as -stride: {kinds:?}"
        );
    }

    #[test]
    fn subcommand_option_classified_as_decorator() {
        // `file delete -force filename`.  `-force`
        // is declared on the `delete` *subcommand* (not on `file` itself), so
        // it is only recognised once subcommand options are consulted.
        let ks = kinds("file delete -force filename\n", tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -force decorator; got {ks:?}"
        );
        // A subcommand option on a different subcommand: `file link -symbolic`.
        let ks = kinds("file link -symbolic a b\n", tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Decorator as u32)),
            "expected -symbolic decorator; got {ks:?}"
        );
        // A `-`-word that is not a declared option stays a plain string, even
        // on a command that has subcommand options elsewhere.
        let ks = kinds("file delete -bogus filename\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-bogus is not a real option; got {ks:?}"
        );
        // A `-$var` substitution word must never be treated as an option.
        let ks = kinds("file delete -$flag filename\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-$flag is a substitution, not an option; got {ks:?}"
        );
    }

    #[test]
    fn unknown_head_options_classified_generically() {
        // The ngspice / ticklecharts pattern: `$chart Xaxis -name
        // {v(anode), V} -type value -min 0.4` — the head `$chart` is an object
        // handle, unknown to the registry, so its `-switch value` pairs are
        // highlighted by the generic heuristic.
        let ks = kinds(
            "$chart Xaxis -name {v} -type value -min 0.4\n",
            tcl(),
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
        let ks = kinds("$obj move -5 10\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-5 is a negative number, not an option; got {ks:?}"
        );
        // A `-$var` substitution word is not an option.
        let ks = kinds("$obj configure -$flag v\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-$flag is a substitution, not an option; got {ks:?}"
        );
        // A known command keeps the strict declared-option behaviour: `puts
        // -foo` stays a string even though the generic pass exists.
        let ks = kinds("puts -foo\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "a known command's undeclared -foo must stay a string; got {ks:?}"
        );
        // A plain bareword unknown head is a (possibly user-defined) command
        // name, not a computed dispatch: `mycmd -foo bar` stays a string so a
        // user proc's argument is not mistaken for an option.
        let ks = kinds("mycmd -foo bar\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "a bareword user command's -foo must stay a string; got {ks:?}"
        );
        // Negative special-float literals are numbers, not options: `-inf`,
        // `-Inf`, `-nan` all start with a letter but Tcl parses them as values.
        let ks = kinds("$obj set -inf\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-inf is a negative float literal, not an option; got {ks:?}"
        );
        let ks = kinds("$obj set -NaN\n", tcl(), &reg());
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
        let mut deco: Vec<u32> = decode_full("$obj cfg -real 1 -- -literal\n", tcl(), &reg())
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
        let ks = kinds("lsort -index 2 $l\n", tcl(), &reg());
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
        let ks = kinds("lsort -unique $l\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::OptionValue as u32)),
            "boolean -unique must not mark a following value; got {ks:?}"
        );
        // A `$var` value keeps its variable highlight, not OptionValue.
        let ks = kinds("lsort -index $i $l\n", tcl(), &reg());
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
            tcl(),
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
        let ks = kinds("argparse -inline {d}\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::OptionValue as u32)),
            "boolean -inline must not mark a following value; got {ks:?}"
        );
    }

    #[test]
    fn subcommand_enum_value_classified_as_enum_member() {
        // `string is alnum $s` — `alnum` is a closed-set value declared on
        // the `is` subcommand → enumMember, not a plain string.
        let ks = kinds("string is alnum $s\n", tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::EnumMember as u32)),
            "expected an enumMember token; got {ks:?}"
        );
        // A value not in the set stays a string.
        let ks = kinds("string is bogusclass $s\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::EnumMember as u32)),
            "bogusclass is not a class; got {ks:?}"
        );
    }

    #[test]
    fn return_code_option_classified_as_decorator_issue_967() {
        // `return -code error "bad"` must not highlight `-code` as a plain
        // string. `-code` is a declared OptionSpec
        // on `return` (a decorator) and `error` is one of its closed-set
        // values (an enumMember); `"bad"` stays a plain string.
        let ks = kinds("return -code error \"bad\"\n", tcl(), &reg());
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
        let ks = kinds("return -level 0 \"bad\"\n", tcl(), &reg());
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
        let ks = kinds("return -options $opts\n", tcl(), &reg());
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
        let ks = kinds("concat -code error\n", tcl(), &reg());
        assert!(
            !ks.contains(&(TokenKind::Decorator as u32)),
            "-code is not a declared option of concat; got {ks:?}"
        );
    }

    #[test]
    fn info_object_class_sub_subcommand_classified_as_keyword() {
        // In `info object class $obj`, the `class` word is a
        // second-level subcommand (OBJECT INTROSPECTION), not a string. Both the
        // first-level `object` and the second-level `class` must read as
        // keywords (the `info` head itself is a Function).
        let kind_at = |src: &str, col: u32| -> u32 {
            decode_full(src, tcl(), &reg())
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

        // Unique-prefix abbreviation: `info object cl` is Tcl's
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
        let kind_at = |src: &str, dialect: &'static tcl_dialect::DialectProfile, col: u32| -> u32 {
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
        assert_eq!(
            kind_at(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                7
            ),
            TokenKind::Keyword as u32
        );
        assert_eq!(
            kind_at(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
                7
            ),
            TokenKind::String as u32,
            "`string rev` is not a subcommand in 8.4"
        );

        // `info class def`: uniquely `definition` in 8.6 (keyword), but
        // ambiguous with `definitionnamespace` in 9.0 (verified against tclsh)
        // → stays a string.  Column 11 is `def`.
        let src = "info class def ::C\n";
        assert_eq!(
            kind_at(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                11
            ),
            TokenKind::Keyword as u32,
            "`info class def` is `definition` in 8.6"
        );
        assert_eq!(
            kind_at(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                11
            ),
            TokenKind::String as u32,
            "`info class def` is ambiguous in 9.0"
        );
    }

    #[test]
    fn abbreviated_first_level_subcommand_highlights_as_keyword() {
        // `string le $s` — `le` is Tcl's unique-prefix abbreviation of
        // `length`; it must highlight as a subcommand keyword (column 7).
        let src = "string le $s\n";
        let toks = decode_full(src, tcl(), &reg());
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
        let kind = decode_full(src, tcl(), &reg())
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
        let ks = kinds("oo::define Cls method foo {} {}\n", tcl(), &reg());
        let n_kw = ks
            .iter()
            .filter(|&&k| k == TokenKind::Keyword as u32)
            .count();
        // Two keyword tokens: the `oo::define` head and the inline `method`.
        assert!(n_kw >= 2, "expected >=2 keyword tokens; got {ks:?}");
        // `oo::define Cls self method foo {} {}` — the inner keyword after
        // `self` is highlighted too.
        let ks = kinds("oo::define Cls self method foo {} {}\n", tcl(), &reg());
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
        let toks = decode_full("set x 1\n", tcl(), &reg());
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
        let ks = kinds("set x\n", tcl(), &reg());
        // No token should carry the declaration modifier here; `x` stays a
        // plain string.  (Modifier is checked in the full decode.)
        let toks = decode_full("set x\n", tcl(), &reg());
        assert!(
            !toks.iter().any(|(_, _, _, _, m)| *m == MOD_DECLARATION),
            "bare `set x` must not declare; got {toks:?} kinds {ks:?}"
        );
    }

    #[test]
    fn unset_marks_every_name_as_variable() {
        // `unset x y z` — every name is a variable, not just the first.
        let toks = decode_full("unset x y z\n", tcl(), &reg());
        let vars = toks
            .iter()
            .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
            .count();
        assert_eq!(
            vars, 3,
            "all three unset names must highlight as variables; got {toks:?}"
        );
        // Leading `-nocomplain` / `--` options are not variables.
        let toks = decode_full("unset -nocomplain -- a b\n", tcl(), &reg());
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
        let toks = decode_full("proc p {} { global a b c }\n", tcl(), &reg());
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
        let toks = decode_full("set arr($i) 1\n", tcl(), &reg());
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
        // `Variable` declaration, matching the `$arr(key)` read.
        for src in [
            "set arr(key) 1\n",
            "incr count(hits)\n",
            "append log(err) x\n",
        ] {
            let toks = decode_full(src, tcl(), &reg());
            let decl = toks
                .iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION);
            assert!(
                decl,
                "expected an array-element variable declaration; got {toks:?} for {src:?}"
            );
        }
        // The target `arr(key)` is a single token spanning the whole element.
        let toks = decode_full("set arr(key) 1\n", tcl(), &reg());
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
        let toks = decode_full("set ::ns::arr(key) 1\n", tcl(), &reg());
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
        let toks = decode_full("unset arr(key)\n", tcl(), &reg());
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
        let toks = decode_full("unset arr($i)\n", tcl(), &reg());
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
        // Falling through to the default classification would paint them as
        // `string`: `set env($lo)`, `unset UnknownPending($name)` and friends
        // all over Tcl's own library.
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
        // modifier, since a read references an existing variable.
        for src in [
            "info exists arr(key)\n",
            "info exists scalar\n",
            "array names arr\n",
            "array get arr\n",
        ] {
            let toks = decode_full(src, tcl(), &reg());
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
        let toks = decode_full("dict get $d k\n", tcl(), &reg());
        // Exactly one variable: the `$d` substitution.
        let vars = toks
            .iter()
            .filter(|(_, _, _, k, _)| *k == TokenKind::Variable as u32)
            .count();
        assert_eq!(vars, 1, "only `$d` is a variable; got {toks:?}");
    }

    #[test]
    fn stub_var_arg_highlights_array_element() {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        // A `# tcl-lsp: stub` with a `:var` argument marks that position a
        // variable-name spot, so a literal array element passed there
        // highlights like `set arr(key) …` — even on the registry-only path,
        // since stub roles are derived from the document source.
        let src = "# tcl-lsp: stubs-begin\n\
                   # tcl-lsp: stub mywrite {varName:var value}\n\
                   # tcl-lsp: stubs-end\n\
                   mywrite arr(key) 1\n";
        let toks = decode_full(src, tcl(), &reg());
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
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        // A `# tcl-lsp: stub … :var_read` argument marks a read-position
        // variable name, so a literal array element there highlights as a plain
        // `Variable` reference (no `declaration` modifier).
        let src = "# tcl-lsp: stubs-begin\n\
                   # tcl-lsp: stub myexists {varName:var_read}\n\
                   # tcl-lsp: stubs-end\n\
                   myexists arr(key)\n";
        let toks = decode_full(src, tcl(), &reg());
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
        // highlights `arr(key)` like `set arr(key) 1`.
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
        let plain = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
        assert!(
            !decl_on_call(&plain),
            "no proc-role highlight without analysis; got {plain:?}"
        );
        // With analysis, the inferred `varName` role retags `arr(key)`.
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
        let plain = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            decode_full(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
            )
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
        let toks = decode_full(src, tcl(), &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
            "expected `z` as a variable declaration inside the method body; got {toks:?}"
        );
        // constructor body too.
        let src = "oo::class create C {\n  constructor {} {\n    set q 9\n  }\n}\n";
        let toks = decode_full(src, tcl(), &reg());
        assert!(
            toks.iter()
                .any(|(_, _, _, k, m)| *k == TokenKind::Variable as u32 && *m == MOD_DECLARATION),
            "expected `q` declared inside the constructor body; got {toks:?}"
        );
    }

    #[test]
    fn option_command_value_recurses_as_body_script() {
        // `button .b -command {puts $x}` — the `-command` value is a script
        // body (`ArgRole::Body`), so it recurses: `$x` inside the braces
        // resolves as a Variable rather than one opaque string.
        let toks = decode_full(
            "button .b -command {puts $x}\n",
            crate::profile_for_dialect("tk"),
            &reg(),
        );
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
            "expected $x resolved inside the -command body; got {toks:?}"
        );
    }

    #[test]
    fn console_eval_body_recurses_into_script() {
        // `console eval {puts $x}` — the `console eval` script
        // argument is a body (ArgRole::Body via the `console` SubCommand
        // table), so it recurses: `$x` inside the braces resolves as a
        // Variable rather than the whole `{...}` staying one opaque string.
        let toks = decode_full(
            "console eval {puts $x}\n",
            crate::profile_for_dialect("tk"),
            &reg(),
        );
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Variable as u32),
            "expected $x resolved inside the `console eval` body; got {toks:?}"
        );
    }

    #[test]
    fn consoleinterp_eval_and_record_bodies_recurse_into_script() {
        // `consoleinterp eval`/`record` — both take a
        // script argument that should recurse the same way `console eval`
        // does.
        for sub in ["eval", "record"] {
            let src = format!("consoleinterp {sub} {{puts $x}}\n");
            let toks = decode_full(&src, crate::profile_for_dialect("tk"), &reg());
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
        // as an EnumMember, not a generic OptionValue.
        let toks = decode_full(
            "button .b -relief raised\n",
            crate::profile_for_dialect("tk"),
            &reg(),
        );
        assert!(
            toks.iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::EnumMember as u32),
            "expected `raised` as EnumMember; got {toks:?}"
        );
    }

    #[test]
    fn option_textvariable_value_is_variable_declaration() {
        // `entry .e -textvariable myvar` — the value names a variable the widget
        // reads/writes (`ArgRole::VarWrite`), so it is a Variable
        // declaration, not a plain `OptionValue` string.
        let toks = decode_full(
            "entry .e -textvariable myvar\n",
            crate::profile_for_dialect("tk"),
            &reg(),
        );
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
        let toks = decode_full("regexp \"abc$var.*\" $s\n", tcl(), &reg());
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
        let simple = decode("regexp \"abc$var.*\" $s\n", tcl(), &reg());
        for w in simple.windows(2) {
            let (l0, c0, len0) = w[0];
            let (l1, c1, _) = w[1];
            if l0 == l1 {
                assert!(c1 >= c0 + len0, "overlap; toks={simple:?}");
            }
        }
        // `regexp "$only" $s` — a bare-substitution pattern is just a
        // variable, not a regex anchor.
        let toks = decode_full("regexp \"$only\" $s\n", tcl(), &reg());
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
        let plain = decode_full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(
            !plain
                .iter()
                .any(|&(l, _, _, k, _)| l == 0 && k == TokenKind::RegexpQuantifier as u32),
            "no regex tokens without a CU; got {plain:?}"
        );
        // With a CU, the `.*abc` literal at the `set` reads as a regex.
        let cu = CompilationUnit::build_for(src, &registry, false);
        let st = full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        );
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
        // Root registry calls because TclOO selects the receiver's private
        // namespace at runtime, where a relative command may be shadowed.
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::class create C {\n  method m {s} {\n    ::set re \".*x\"\n    ::regexp $re $s\n  }\n}\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
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
        // The ngspice / ticklecharts pattern, end-to-end through
        // the CompilationUnit: `set chart [ticklecharts::chart new]` binds the
        // handle's class, so `$chart Xaxis -name … -type value …` resolves the
        // method and its declared options through the registry's object-class
        // model — the precise path, not the shape-based fallback.
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "set chart [ticklecharts::chart new]\n$chart Xaxis -name {v(anode), V} -type value -min 0.4 -splitLine {show True}\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let toks = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
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
        let toks = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
        let ks: Vec<u32> = toks.iter().map(|&(_, _, _, k, _)| k).collect();
        assert!(
            ks.contains(&(TokenKind::Decorator as u32))
                && ks.contains(&(TokenKind::OptionValue as u32)),
            "expected -name/-max decorators + values on direct dispatch; got {ks:?}"
        );
    }

    #[test]
    fn collection_dispatch_resolves_user_configurable_method() {
        // End-to-end: a `Pins` dict is filled with `[Pin new]`
        // handles in one method and an element is dispatched with
        // `[dict get $Pins $pin] configure -node …` in another.  The receiver
        // resolves to the user `oo::configurable` class, so `configure` is a
        // method and `-node` (a declared property) an option — not the plain
        // strings they read as without collection-element + user-class
        // resolution.
        // Root registry calls because TclOO selects the receiver's private
        // namespace at runtime, where a relative command may be shadowed.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {pin} { ::dict append Pins $pin [::Pin new] }\n\
                     method cfg {pin node} { [::dict get $Pins $pin] configure -node $node }\n\
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
        let plain = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
        assert!(
            !user_method_on_dispatch_line(&plain),
            "without analysis, no user method resolves; got {plain:?}"
        );
        // With analysis: the dynamic dispatch resolves `configure` as a method.
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
        // (the SpiceGenTcl `allNodes` / `floating` shape).
        // Root registry calls because TclOO selects the receiver's private
        // namespace at runtime, where a relative command may be shadowed.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { ::dict append Pins $p [::Pin new] }\n\
                     method dump {} { ::dict for {k pin} $Pins { ::puts [$pin configure -node] } }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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

    #[test]
    fn widget_method_tokens_follow_the_resolved_tk_lifecycle() {
        // `current` belongs to ttk::treeview only from Tk 9.1. The semantic
        // provider must consume the registry's filtered object-method table,
        // not the raw class descriptor that contains future methods.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;

        let src = "ttk::treeview .tree\n.tree current\n";
        for (dialect, expected) in [("tcl9.0", false), ("tcl9.1", true)] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let cu = CompilationUnit::build_for(src, registry, false);
            let analysis = Analyser::new().analyse(src, dialect);
            let tokens = decode_semantic(&full_with_cu_and_analysis(
                src,
                profile,
                registry,
                Some(&cu),
                Some(&analysis),
            ));
            assert_eq!(
                tokens.iter().any(|&(line, _, _, kind, _)| {
                    line == 1 && kind == TokenKind::Method as u32
                }),
                expected,
                "ttk::treeview current semantic token under {dialect}: {tokens:?}"
            );
        }
    }

    /// A bare (namespace-less) `apply {{} {...}}` runs its body in a *fresh*
    /// call frame in the global namespace by default — never the enclosing
    /// method's object namespace — so `my helper` inside it is not actually a
    /// call to the enclosing class's method at runtime (`my` isn't defined in
    /// `::`); a class-definition-body scan would raise "invalid command name
    /// my" here. The `enclosing_class` context must not leak into an
    /// `apply`-lambda body the way it correctly persists into an ordinary
    /// nested `if`/`foreach` body, or this gets painted as a resolved,
    /// legitimate method call anyway — the same fresh-frame rule call-graph
    /// namespace resolution, param traits and declarations apply.
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
    /// component, whose name appears nowhere in the call.
    ///
    /// VERIFIED against tcllib snit(n): "Given this form, `installhull` creates
    /// the hull widget, and initializes any options delegated to the hull from
    /// the Tk option database."  The second documented form, `installhull
    /// $win`, names an already-created widget and carries no static type word,
    /// so it must state nothing.
    /// `configure` / `cget` resolution must key off registry data about the
    /// metaclass, not the `oo::configurable` spelling.
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
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            // TclOO's own same-object dispatch idiom — `self`
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
            // equivalent to a bare `self`.
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
            // Tk widget instance dispatch, bareword receiver:
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
            // return value (`set lb [listbox .l]`).
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
            // `CLASS create NAME` — resolved via
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
    ///   callable, for every form the resolver supports.
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
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
        // class is not in this file (the cross-file mro_eval lever).
        use tcl_compiler::analyser::{Analyser, build_class_hierarchy};
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let lib = "oo::configurable create ::Pin { property node }\n";
        let hierarchy = build_class_hierarchy(Analyser::new().analyse(lib, "tcl9.0").all_classes);
        let user = "[::Pin new] configure -node 5\n";
        let cu = CompilationUnit::build_for(user, &registry, false);
        let toks = decode_semantic(&full_with_cu_and_classes(
            user,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
        // `getPinsNodes` / `getParams` shape).
        // Root registry calls because TclOO selects the receiver's private
        // namespace at runtime, where a relative command may be shadowed.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { ::dict append Pins $p [::Pin new] }\n\
                     method nodes {} { ::return [::dict map {k pin} $Pins {$pin configure -node}] }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
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
    fn malformed_braced_object_handle_does_not_resolve_a_method() {
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::compilation_unit::CompilationUnit;
        let registry = reg();
        let src = "oo::configurable create Pin { property node }\n\
                   set p [Pin new]\n\
                   ${p configure -node 5\n";
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert!(
            !toks
                .iter()
                .any(|&(l, _, _, k, _)| { l == 2 && k == TokenKind::Method as u32 }),
            "malformed `${{p` must not dispatch to Pin: {toks:?}"
        );
        assert_eq!(object_handle_name("${x}"), Some("x"));
        assert_eq!(object_handle_name("$x"), Some("x"));
        assert_eq!(object_handle_name("$arr(idx)"), Some("arr(idx)"));
        assert_eq!(object_handle_name("$::a:::b"), Some("::a:::b"));
        assert_eq!(object_handle_name("$foo:::"), Some("foo:::"));
        assert_eq!(object_handle_name("${x"), Some("{x"));
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
        let toks = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
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
        let toks = decode_semantic(&full_with_cu(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
        ));
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
    pub(super) fn decode_semantic(st: &SemanticTokens) -> Vec<(u32, u32, u32, u32, u32)> {
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
        let toks = decode_full(src, tcl(), &reg());
        assert!(
            !toks
                .iter()
                .any(|(_, _, _, k, _)| *k == TokenKind::Comment as u32),
            "a `#` inside a string must not be a comment; got {toks:?}"
        );
        // A real comment still is one.
        let toks = decode_full("# real\nset x 1\n", tcl(), &reg());
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
        // (LSP clients reject overlapping semantic tokens).
        let toks = decode_full("chartV$node SetOptions -x {}\n", tcl(), &reg());
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
        // `[dict get $Pins $pin] configure -node $node` — the head
        // is a `[…]` command substitution, a runtime-computed command name, not
        // a resolvable command.  It must recurse into its inner script (`dict`
        // as a builtin, `get` as its subcommand, `$Pins` / `$pin` as variables)
        // rather than paint the whole `[…]` word as one function command token.
        let src = "[dict get $Pins $pin] configure -node $node\n";
        let toks = decode_full(src, tcl(), &reg());
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
        // (the `$chart` object-dispatch shape).
        let src = "$obj configure -node $node\n";
        let toks = decode_full(src, tcl(), &reg());
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
        let toks = decode_full(src, tcl(), &reg());
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
        let ks = kinds("apply $lambda a b\n", tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Variable as u32)),
            "expected the $lambda variable token; got {ks:?}"
        );
    }

    #[test]
    fn apply_bare_arglist_param_is_a_parameter() {
        // `apply {dir { … }}` — the argument list is a bare,
        // unbraced single name.  Its parameter (`dir`) must highlight as a
        // `Parameter` declaration (not a `string`), and the body commands
        // must still tokenise as a script.
        let registry = reg();
        let src = "apply {dir {\n    puts $dir\n}} /tmp\n";
        assert!(
            has_token_kind(src, tcl(), &registry, "dir", TokenKind::Parameter),
            "bare arg-list param `dir` must be a Parameter declaration; got {:?}",
            decode_full(src, tcl(), &registry)
        );
        assert!(
            has_token_kind(src, tcl(), &registry, "puts", TokenKind::Function),
            "apply body command `puts` must tokenise as a Function; got {:?}",
            decode_full(src, tcl(), &registry)
        );
        // A braced arg list still emits its names as parameters.
        let braced = "apply {{a b} { expr {$a + $b} }} 1 2\n";
        assert!(
            has_token_kind(braced, tcl(), &registry, "a", TokenKind::Parameter)
                && has_token_kind(braced, tcl(), &registry, "b", TokenKind::Parameter),
            "braced arg-list params must stay parameters; got {:?}",
            decode_full(braced, tcl(), &registry)
        );
        // A computed (`$dynamic`) arg list is not painted as a parameter.
        assert!(
            !has_token_kind(
                "apply [list $al $body]\n",
                tcl(),
                &registry,
                "al",
                TokenKind::Parameter
            ),
            "a computed arg list must not be a parameter declaration"
        );
    }

    /// `apply`'s lambda body is reachable indirectly through
    /// `[list apply {…} $x]`, the idiomatic way
    /// to build a deferred command around a dynamic value — most commonly a
    /// pkgIndex.tcl `package ifneeded name ver [list apply {dir {…}} $dir]`
    /// entry. TP cases: the reported repro, a namespace-qualified `::apply`,
    /// and the same idiom under a *different* enclosing Body-role command
    /// (`after idle`) to prove the recognition is registry-driven (any
    /// Body-role position), not special-cased to `package ifneeded`.
    #[test]
    fn list_quoted_apply_lambda_body_recurses() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let registry = reg();

        // The exact reported repro: a pkgIndex.tcl-style entry.
        let pkgindex = "package ifneeded myPackage 1.0.0 [list apply {dir {\n    source [file join $dir x.tcl]\n}} $dir]\n";
        assert!(
            has_token_kind(pkgindex, tcl(), &registry, "source", TokenKind::Keyword),
            "pkgIndex-style list-quoted apply body: `source` must tokenise; got {:?}",
            decode_full(pkgindex, tcl(), &registry)
        );
        assert!(
            has_token_kind(pkgindex, tcl(), &registry, "dir", TokenKind::Parameter),
            "pkgIndex-style list-quoted apply: `dir` param must be a Parameter; got {:?}",
            decode_full(pkgindex, tcl(), &registry)
        );
        // The reconstructed command-name word itself reads as a call-site
        // reference, same as a literal head — not a plain string.
        assert!(
            has_token_kind(pkgindex, tcl(), &registry, "apply", TokenKind::Function),
            "the list-quoted `apply` word must read as a Function reference; got {:?}",
            decode_full(pkgindex, tcl(), &registry)
        );

        // A `::`-qualified spelling resolves the same way (registry `get`
        // strips a leading `::`, exactly like a direct `::apply {…}` call).
        let qualified = "package ifneeded p 1.0 [list ::apply {dir {puts $dir}} $dir]\n";
        assert!(
            has_token_kind(qualified, tcl(), &registry, "puts", TokenKind::Function),
            "qualified `::apply` list-quoted body: `puts` must tokenise; got {:?}",
            decode_full(qualified, tcl(), &registry)
        );

        // A *different* Body-role enclosing command (`after idle`) proves
        // the fix isn't specific to `package ifneeded`.
        let after_idle = "after idle [list apply {{x} {puts $x}} 5]\n";
        assert!(
            has_token_kind(after_idle, tcl(), &registry, "puts", TokenKind::Function),
            "after-idle list-quoted apply body: `puts` must tokenise; got {:?}",
            decode_full(after_idle, tcl(), &registry)
        );
        assert!(
            has_token_kind(after_idle, tcl(), &registry, "x", TokenKind::Parameter),
            "after-idle list-quoted apply: `x` param must be a Parameter; got {:?}",
            decode_full(after_idle, tcl(), &registry)
        );
    }

    /// FP guards for the `[list …]`-quoted lambda recognition: a plain data
    /// list, a list naming an unregistered head, and a dynamic `list` head
    /// must never be split as if they were an apply lambda.
    #[test]
    fn list_quoted_apply_lambda_false_positive_guards() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let registry = reg();

        // Ordinary data list: `list`'s own args stay whatever the default
        // classifier gives them — none of them becomes a `Parameter`
        // declaration (which only a recognised lambda-literal split emits).
        let data_list = "set data [list puts hello world]\n";
        assert!(
            !has_token_kind(data_list, tcl(), &registry, "hello", TokenKind::Parameter),
            "a plain data list must not be split as a lambda literal"
        );

        // `list`'s first argument names a command that does not carry
        // `ArgRole::LambdaLiteral` (`puts` is an ordinary command) — its
        // second argument must not be treated as a lambda body either.
        assert!(
            !has_token_kind(
                "set cb [list puts hello]\n",
                tcl(),
                &registry,
                "hello",
                TokenKind::Parameter
            ),
            "list-quoting a non-lambda-literal command must not split its trailing arg"
        );

        // An unregistered head: no crash, no spurious split.
        let unknown = "set cb [list notARealCommand {dir {source x.tcl}} $dir]\n";
        assert!(
            !has_token_kind(unknown, tcl(), &registry, "dir", TokenKind::Parameter),
            "an unresolvable list-quoted head must not be split as a lambda literal"
        );

        // A dynamic `list` head (`$cmd`) can't be resolved statically.
        let dynamic_head = "set cb [list $cmd {dir {source x.tcl}} $dir]\n";
        assert!(
            !has_token_kind(dynamic_head, tcl(), &registry, "dir", TokenKind::Parameter),
            "a dynamic list head must not be split as a lambda literal"
        );

        // `[llength $x]` — a `Cmd` substitution whose head is `llength`, not
        // `list` (no `BUILDS_COMMAND_PREFIX` trait) — must not be misread.
        assert!(
            !has_token_kind(
                "set n [llength $items]\n",
                tcl(),
                &registry,
                "items",
                TokenKind::Parameter
            ),
            "llength must never be treated as a command-quoting construct"
        );

        // A well-formed `[list apply …]` shape sitting in an *inert* argument slot (here `set`'s value,
        // which carries no `Body` / `LambdaLiteral` / `CommandPrefix` role)
        // must not be treated as a deferred invocation — `list` only ever
        // returns a value here; nothing ever invokes `apply`.
        let inert_data = "set data [list apply {x {puts $x}} value]\n";
        assert!(
            !has_token_kind(inert_data, tcl(), &registry, "x", TokenKind::Parameter),
            "an inert `[list apply …]` value must not paint its param as a \
             Parameter; got {:?}",
            decode_full(inert_data, tcl(), &registry)
        );
        assert!(
            !has_token_kind(inert_data, tcl(), &registry, "puts", TokenKind::Function),
            "an inert `[list apply …]` value must not recurse into its body \
             as executable code; got {:?}",
            decode_full(inert_data, tcl(), &registry)
        );
        assert!(
            !has_token_kind(inert_data, tcl(), &registry, "apply", TokenKind::Function),
            "an inert `[list apply …]` value's `apply` word must not read as \
             a call-site reference; got {:?}",
            decode_full(inert_data, tcl(), &registry)
        );
    }

    /// TN: `package ifneeded`'s script argument, when a literal braced script
    /// (no `[list …]` wrapper), is recognised generically as `ArgRole::Body`.
    #[test]
    fn package_ifneeded_literal_script_recurses_as_body() {
        let registry = reg();
        let src = "package ifneeded myPackage 1.0.0 {\n    source [file join $dir x.tcl]\n}\n";
        assert!(
            has_token_kind(src, tcl(), &registry, "source", TokenKind::Keyword),
            "package ifneeded literal script: `source` must tokenise; got {:?}",
            decode_full(src, tcl(), &registry)
        );
    }

    /// True if any token in `src` covers exactly `text` with kind `kind`.
    fn has_token_kind(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
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
        // The braced body of `uplevel ?level? {body}` runs in
        // another stack frame but is still a Tcl script — it must be
        // recursed and highlighted, not rendered as one opaque string.
        let registry = reg();

        // `uplevel 1 {body}` — literal relative level, body at arg 1.
        let src = "uplevel 1 {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "foreach",
                TokenKind::Keyword
            ),
            "uplevel 1 body: `foreach` must tokenise as a keyword; got {:?}",
            decode_full(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry
            )
        );
        assert!(
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "puts",
                TokenKind::Function
            ),
            "uplevel 1 body: `puts` must tokenise as a function"
        );

        // `uplevel {body}` — no level, body at arg 0.
        let src = "uplevel {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "foreach",
                TokenKind::Keyword
            ),
            "uplevel (no level) body: `foreach` must tokenise as a keyword"
        );

        // `uplevel #0 {body}` — absolute global level.
        let src = "uplevel #0 {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "foreach",
                TokenKind::Keyword
            ),
            "uplevel #0 body: `foreach` must tokenise as a keyword"
        );

        // `uplevel $lvl {body}` — dynamic level word, body still recurses.
        let src = "uplevel $lvl {foreach x $l { puts $x }}\n";
        assert!(
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "foreach",
                TokenKind::Keyword
            ),
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
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "$body",
                TokenKind::Variable
            ),
            "uplevel 1 $body: the body variable must stay a variable token; got {:?}",
            decode_full(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry
            )
        );
    }

    #[test]
    fn uplevel_issue_837_repro_recurses() {
        // A `foreach` /
        // `namespace children` / `namespace forget` body inside
        // `uplevel 1 {…}` must highlight, not sit inside one string.
        let registry = reg();
        let src = "proc forgetXyce {} {\n    uplevel 1 {foreach nameSpc [namespace children ::SpiceGenTcl::Xyce] {\n        namespace forget ${nameSpc}::*\n    }}\n}\n";
        assert!(
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "foreach",
                TokenKind::Keyword
            ),
            "issue #837: `foreach` inside the uplevel body must be a keyword; got {:?}",
            decode_full(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry
            )
        );
        assert!(
            has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "namespace",
                TokenKind::Keyword
            ) || has_token_kind(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
                "namespace",
                TokenKind::Function
            ),
            "issue #837: `namespace` inside the uplevel body must be highlighted"
        );
        // The `${nameSpc}` reference deep inside the body highlights as a
        // variable — proof the whole body was re-lexed, not stringified.
        assert!(
            decode_full(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry
            )
            .iter()
            .any(|&(_, _, _, k, _)| k == TokenKind::Variable as u32),
            "issue #837: a variable token is expected from the recursed body"
        );
    }

    #[test]
    fn operator_command_head_classified_as_operator() {
        // `+ 3 4` — the operator head is `operator`, not `function`.
        let ks = kinds("+ 3 4\n", tcl(), &reg());
        assert_eq!(ks.first(), Some(&(TokenKind::Operator as u32)), "{ks:?}");
    }

    /// A hand-typed symbol list would miss every word-form comparison operator
    /// — `eq 1 1` (a bare `::tcl::mathop::eq` invocation via `namespace
    /// import`) classifying as `function` rather than `operator`. Also covers
    /// the TIP 461 `lt`/`le`/`gt`/`ge` mathop commands (9.0+).
    #[test]
    fn word_form_operator_command_heads_classified_as_operator() {
        for op in ["eq", "ne", "in", "ni", "lt", "le", "gt", "ge"] {
            let src = format!("{op} 1 1\n");
            let ks = kinds(
                &src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &reg(),
            );
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
        let ks = kinds("puts hello\n", tcl(), &reg());
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
        let ks = kinds("regexp {abc} $s\n", tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Regexp as u32)),
            "expected a regexp token; got {ks:?}"
        );
        // `regsub -all {x+} $s y out` — option-skip finds the pattern.
        let ks = kinds("regsub -all {x+} $s y out\n", tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::Regexp as u32)),
            "expected a regexp token after -all; got {ks:?}"
        );
    }

    /// Every resolver-owned pattern layout needs the same
    /// source proof as lsearch. A dynamic leading word can be a value-taking
    /// switch at runtime, so it cannot make the later literal a regexp; the
    /// analogous regsub query must not steal the replacement as a template.
    #[test]
    fn dynamic_leading_options_do_not_claim_pattern_or_regsub_replacement_tokens() {
        let registry = reg();
        for src in [
            "regexp $mode {a+} $value\n",
            "regsub $mode {a+} $value {\\1}\n",
        ] {
            let tokens = decode_full(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
            );
            assert!(
                !tokens.iter().any(|&(_, _, _, kind, _)| {
                    matches!(
                        kind,
                        value
                            if value == TokenKind::Regexp as u32
                                || value == TokenKind::RegexpQuantifier as u32
                    )
                }),
                "{src:?}: unresolved leading option must not claim a regex word: {tokens:?}",
            );
        }

        let tokens = decode_full(
            "regsub $mode {a} $value {\\1}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(
            !tokens.iter().any(|&(_, _, _, kind, _)| {
                kind == TokenKind::Number as u32 || kind == TokenKind::Operator as u32
            }),
            "an unresolved regsub replacement is ordinary Tcl text: {tokens:?}",
        );

        for src in [
            "regsub -c {a} $value {\\1}\n",
            "regsub -command {a} $value callback\n",
        ] {
            let tokens = decode_full(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                &registry,
            );
            assert!(
                !tokens.iter().any(|&(_, _, _, kind, _)| {
                    kind == TokenKind::Number as u32 || kind == TokenKind::Operator as u32
                }),
                "{src:?}: only a valid template layout may receive regsub sub-tokens: {tokens:?}",
            );
        }

        let tokens = decode_full(
            "regsub -start $start {a} $value {\\1}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(
            tokens
                .iter()
                .any(|&(_, _, _, kind, _)| kind == TokenKind::Number as u32),
            "a known fixed-width -start option preserves the regsub replacement: {tokens:?}",
        );
    }

    #[test]
    fn event_name_classified_as_event() {
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        let ks = kinds(
            "when HTTP_REQUEST {\n  set x 1\n}\n",
            tcl_dialect::DialectProfile::irules(),
            &registry,
        );
        assert!(
            ks.contains(&(TokenKind::Event as u32)),
            "expected an event token; got {ks:?}"
        );
    }

    #[test]
    fn malformed_when_body_does_not_classify_an_event_declaration() {
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        for source in [
            "when HTTP_REQUEST bare_body\n",
            "when HTTP_REQUEST \"quoted body\"\n",
            "when HTTP_REQUEST { unterminated",
        ] {
            let ks = kinds(source, tcl_dialect::DialectProfile::irules(), &registry);
            assert!(
                !ks.contains(&(TokenKind::Event as u32)),
                "malformed declaration colored HTTP_REQUEST as an event: {source:?}; {ks:?}"
            );
        }
    }

    #[test]
    fn bigip_object_ref_token_in_irules_body() {
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        // `pool web_pool` inside a multi-line `when` body → `object`.
        for dialect in ["f5-irules", "irules", "tcl-irule"] {
            let ks = kinds(
                "when HTTP_REQUEST {\n  pool web_pool\n}\n",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                &registry,
            );
            assert!(
                ks.contains(&(TokenKind::Object as u32)),
                "{dialect}: {ks:?}"
            );
        }
    }

    #[test]
    fn bigip_object_ref_not_emitted_in_plain_tcl() {
        // The object overlay is iRules-only.
        let ks = kinds("when HTTP_REQUEST {\n  pool web_pool\n}\n", tcl(), &reg());
        assert!(!ks.contains(&(TokenKind::Object as u32)), "{ks:?}");
    }

    #[test]
    fn regex_pattern_subtokenised_into_components() {
        // `(a+)+` → group `(`, literal `a`, quantifier `+`, group `)`,
        // quantifier `+`.
        let ks = kinds("regexp {(a+)+} $s\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
        // The whole-pattern `regexp` kind is replaced by sub-tokens, but
        // the literal `a` run is still `regexp`.
        assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
    }

    #[test]
    fn regex_char_class_and_anchor_subtokens() {
        let ks = kinds("regexp {^[0-9]+$} $s\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpAnchor as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
    }

    #[test]
    fn regex_alternation_and_escape_subtokens() {
        let ks = kinds("regexp {a\\d|b} $s\n", tcl(), &reg());
        assert!(
            ks.contains(&(TokenKind::RegexpAlternation as u32)),
            "{ks:?}"
        );
        // `\d` is an ARE class shortcut → char class.
        assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
    }

    #[test]
    fn escape_before_multibyte_char_does_not_panic() {
        // Slicing a fixed 2 bytes at the backslash of a `\<non-ASCII>` escape
        // inside a string lands inside the multi-byte char and panics the whole
        // semantic-tokens request. The escape must span the backslash plus the
        // full UTF-8 char.
        for src in [
            "puts \"\\é\"\n",
            "puts \"a\\你b\"\n",
            "puts \"\\€\"\n",
            "puts \"x\\é\\你\"\n",
        ] {
            let ks = kinds(src, tcl(), &reg());
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
        // Without the sub-bracket rule `[[:alpha:]]+` scans as `[[:alpha:]`
        // (char class) + a stray literal `]` + `+`. The whole `[[:alpha:]]` is
        // one char class and `+` its quantifier — and, per the token-overlap
        // invariant, no token may start inside another.
        let src = "regexp {[[:alpha:]]+} $s\n";
        let toks = decode_full(src, tcl(), &reg());
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
        let ks = kinds("regexp {abc} $s\n", tcl(), &reg());
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
        let ks = kinds("format {%d} $n\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn format_binary_specifier_is_release_gated() {
        let old = kinds(
            "format {%b} 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
            &reg(),
        );
        assert!(!old.contains(&(TokenKind::FormatSpec as u32)), "{old:?}");
        let modern = kinds(
            "format {%b} 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &reg(),
        );
        assert!(
            modern.contains(&(TokenKind::FormatSpec as u32)),
            "{modern:?}"
        );
        let tcl9 = kinds(
            "format {%b} 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &reg(),
        );
        assert!(tcl9.contains(&(TokenKind::FormatSpec as u32)), "{tcl9:?}");
    }

    #[test]
    fn sprintf_flags_and_width_subtokens() {
        // `%-5.2f` → percent, `-` flag, `5` width, `.` flag, `2` width,
        // `f` spec.
        let ks = kinds("format {%-5.2f} $x\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::FormatFlag as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatWidth as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn scan_format_arg_subtokenised() {
        // `scan`'s format string is arg 2.
        let ks = kinds("scan $s {%d} a\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn format_without_specifiers_stays_string() {
        let ks = kinds("format {plain} $x\n", tcl(), &reg());
        assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn format_literal_percent_stays_string() {
        let ks = kinds(
            "format {100%%} 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &reg(),
        );
        assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(!ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn clock_format_subtokens() {
        // `clock format $t -format {%Y-%m-%d}` → %/letter pairs.
        let ks = kinds("clock format $t -format {%Y-%m-%d}\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
    }

    #[test]
    fn clock_locale_modifier_uses_shared_grammar() {
        // Tcl accepts `%Ey`; the shared clock grammar owns both the modifier
        // and the conversion instead of leaving an editor-only table here.
        let ks = kinds("clock scan $s -format {%Ey}\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::ClockModifier as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
    }

    #[test]
    fn unsupported_clock_g_specifier_stays_string() {
        let ks = kinds(
            "clock scan $s -format {%g}\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &reg(),
        );
        assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn clock_format_without_specifiers_stays_string() {
        let ks = kinds("clock format $t -format {plain}\n", tcl(), &reg());
        assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn binary_format_spec_and_count_subtokens() {
        // `binary format a3 $d` (arg 2) → spec `a`, count `3`.
        let ks = kinds("binary format a3 $d\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::BinaryCount as u32)), "{ks:?}");
    }

    #[test]
    fn binary_scan_signed_modifier_and_star() {
        // `binary scan $d su r` (arg 3) → spec `s`, modifier `u`.
        let ks = kinds(
            "binary scan $d su r\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &reg(),
        );
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
        // `c*` → spec `c`, `*` flag.
        let ks = kinds(
            "binary format c* $l\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &reg(),
        );
        assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    }

    #[test]
    fn binary_signed_modifier_suppressed_in_tcl84() {
        // The `u` suffix is 8.5+ (TIP 275), so under tcl8.4 the `u` is not a
        // binaryFlag.
        let ks = kinds(
            "binary scan $d su r\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
            &reg(),
        );
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(!ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    }

    #[test]
    fn regsub_replacement_backref_subtokens() {
        // `regsub {a} $s {\1-\&} out` → `\1` number, `\&` operator.
        let ks = kinds("regsub {a} $s {\\1-\\&} out\n", tcl(), &reg());
        assert!(ks.contains(&(TokenKind::Number as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
    }

    #[test]
    fn regsub_replacement_without_backrefs_stays_string() {
        let ks = kinds("regsub {a} $s {plain} out\n", tcl(), &reg());
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
        let on_90 = full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &reg(),
        )
        .data;
        let on_84 = full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
            &reg(),
        )
        .data;
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
        let s = full("proc foo {} {}\n", tcl(), &reg());
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
        let s = full("puts hi\n", tcl(), &reg());
        assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
        assert_eq!(s.data[4], MOD_DEFAULT_LIBRARY, "{:?}", s.data);
    }

    #[test]
    fn user_proc_head_has_no_default_library_modifier() {
        // A user-defined command isn't in the registry → `function`
        // with no modifier.
        let s = full("my_custom_cmd 1 2\n", tcl(), &reg());
        assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
        assert_eq!(s.data[4], 0, "{:?}", s.data);
    }

    #[test]
    fn keyword_head_has_no_default_library_modifier() {
        // `if` is a language keyword, not a `function` — no defaultLibrary.
        let s = full("if {1} { puts hi }\n", tcl(), &reg());
        assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
        assert_eq!(s.data[4], 0, "{:?}", s.data);
    }

    #[test]
    fn keywords_classified_as_keyword() {
        let s = full("if {1} { puts hi }\n", tcl(), &reg());
        // First token's type index should be 0 (Keyword) for `if`.
        // The encoded data: [deltaLine, deltaCol, length, type, modifiers].
        assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
    }

    /// Decode the packed stream into `(line, col, len, kind)` tuples plus
    /// the covered source word (ASCII sources only — byte == utf16).
    fn decode_words(src: &str, registry: &CommandRegistry) -> Vec<(u32, u32, u32, u32, String)> {
        let st = full(src, tcl(), registry);
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
        let s = full("# this is a comment\nset x 1\n", tcl(), &reg());
        // The first token should be the comment.
        assert_eq!(s.data[3], TokenKind::Comment as u32, "{:?}", s.data);
    }

    #[test]
    fn variables_classified_as_variable() {
        let s = full("set $x 1\n", tcl(), &reg());
        // The `$x` token kind should be Variable.
        let kinds: Vec<u32> = s.data.chunks(5).map(|c| c[3]).collect();
        assert!(
            kinds.contains(&(TokenKind::Variable as u32)),
            "expected Variable in kinds; got {kinds:?}",
        );
    }

    #[test]
    fn is_number_literal_recognises_integers_and_floats() {
        let n = NumberSyntax::Tcl90;
        assert!(is_number_literal("42", n));
        assert!(is_number_literal("-7", n));
        assert!(is_number_literal("3.14", n));
        assert!(is_number_literal("0xff", n));
        assert!(is_number_literal("0b1010", n));
        assert!(!is_number_literal("abc", n));
        assert!(!is_number_literal("", n));
        assert!(!is_number_literal("1.2.3", n));
    }

    /// The radix prefixes appear in the release that added them, so the
    /// `Number` classification follows the document's dialect rather than
    /// hardcoding 9.0: `0o`/`0b` from 8.5, `0d` from 9.0.  The old private
    /// recogniser knew neither `0o` nor `0d` at all.
    #[test]
    fn is_number_literal_gates_radix_prefixes_on_the_release() {
        for syntax in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
            assert!(is_number_literal("0o17", syntax), "{syntax:?}");
        }
        assert!(!is_number_literal("0o17", NumberSyntax::Tcl84));
        assert!(is_number_literal("0d99", NumberSyntax::Tcl90));
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert!(!is_number_literal("0d99", syntax), "{syntax:?}");
        }
        // `_` digit separators are 9.0+ (the old recogniser applied them to
        // every release inside a radix prefix, and to none outside one).
        assert!(is_number_literal("1_000", NumberSyntax::Tcl90));
        assert!(is_number_literal("0xff_ff", NumberSyntax::Tcl90));
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert!(!is_number_literal("1_000", syntax), "{syntax:?}");
            assert!(!is_number_literal("0xff_ff", syntax), "{syntax:?}");
        }
    }

    /// Shapes the old private recogniser mis-accepted: a body of nothing but
    /// separators, a doubled sign (it stripped signs with
    /// `trim_start_matches`), and radix-invalid digits.
    #[test]
    fn is_number_literal_rejects_malformed_numerals() {
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            for bad in ["0x_", "0x", "--5", "+-5", "0b2", "0o8", "0xZZZ"] {
                assert!(!is_number_literal(bad, syntax), "{bad} under {syntax:?}");
            }
        }
    }

    /// A bare leading zero is octal up to 8.6 and decimal from 9.0, so `08` is
    /// a bad octal numeral (a bareword) before 9.0 and plain decimal after.
    #[test]
    fn is_number_literal_follows_the_leading_zero_rule() {
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            assert!(is_number_literal("0755", syntax), "{syntax:?}");
        }
        assert!(!is_number_literal("08", NumberSyntax::Tcl85));
        assert!(is_number_literal("08", NumberSyntax::Tcl90));
    }

    /// End-to-end: an `0o17` argument paints as a `Number` under an 8.6
    /// document and not under an 8.4 one, where `0o` is not a prefix at all and
    /// the word is an ordinary bareword.
    #[test]
    fn number_tokens_follow_the_document_dialect() {
        let registry = reg();
        let modern = kinds(
            "puts 0o17\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        );
        let old = kinds(
            "puts 0o17\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
            &registry,
        );
        assert!(
            modern.contains(&(TokenKind::Number as u32)),
            "0o17 is a number from 8.5: {modern:?}"
        );
        assert!(
            !old.contains(&(TokenKind::Number as u32)),
            "0o17 is not a number under 8.4: {old:?}"
        );
    }

    #[test]
    fn empty_source_returns_empty_data() {
        assert!(full("", tcl(), &reg()).data.is_empty());
    }

    #[test]
    fn semantic_token_lengths_use_utf16_code_units() {
        let data = full("# 😀x\n", tcl(), &reg()).data;
        assert_eq!(
            &data[..5],
            &[0, 0, 5, TokenKind::Comment as u32, 0],
            "comment token length must count the emoji as two UTF-16 code units",
        );
    }

    #[test]
    fn many_comment_lines_do_not_drift_out_of_bounds() {
        // Hand-incrementing a byte cursor to the end of each comment line
        // while the `chars()` iterator advances one char drifts the cursor past
        // the buffer and slices out of bounds (panic) on files with several
        // comment lines.
        use std::fmt::Write as _;
        let mut src = String::new();
        for i in 0..40 {
            let _ = writeln!(src, "# comment line number {i} with some padding text");
        }
        src.push_str("set x 1\n");
        src.push_str("# trailing comment after code, no final newline");
        let st = full(&src, tcl(), &reg()); // must not panic
        let comments = st
            .data
            .chunks(5)
            .filter(|c| c[3] == TokenKind::Comment as u32)
            .count();
        assert_eq!(comments, 41, "expected one token per comment line");
    }

    #[test]
    fn classify_command_head_picks_namespace_for_qualified() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let rows = decode_words("::string equal A a\nunknown arg", &reg());
        assert!(
            rows.iter()
                .any(|(_, _, _, kind, text)| *kind == TokenKind::Namespace as u32 && text == "::")
        );
        assert!(
            rows.iter()
                .any(|(_, _, _, kind, text)| *kind == TokenKind::Function as u32
                    && text == "unknown")
        );
    }

    #[test]
    fn classify_command_head_follows_the_effective_identity() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source =
            "interp alias {} each {} foreach\neach v {} {}\nproc foreach args {}\nforeach v {} {}";
        let rows = decode_words(source, &reg());
        assert!(
            rows.iter()
                .any(|(_, _, _, kind, text)| *kind == TokenKind::Keyword as u32 && text == "each")
        );
        assert!(!rows.iter().any(|(line, _, _, kind, text)| *line == 3
            && *kind == TokenKind::Keyword as u32
            && text == "foreach"));
    }

    // range variant

    #[test]
    fn range_filters_tokens_outside_window() {
        // Three commands on three lines.  Range covers only
        // line 1 — the line-0 and line-2 tokens should drop.
        let src = "set a 1\nset b 2\nset c 3\n";
        let full_data = full(src, tcl(), &reg());
        let line1_only = range(
            src,
            tcl(),
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
        let full_data = full(src, tcl(), &reg());
        let wide = range(
            src,
            tcl(),
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
        // LSP ranges are
        // half-open [start, end), so a token starting exactly
        // at `end` is OUTSIDE the range.
        let src = "set a 1\nset b 2\n";
        // Range whose end exactly coincides with line 1, col 0
        // (the `set` of the second command).  That token should
        // not appear in the range result.
        let r = range(
            src,
            tcl(),
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
        let full_data = full(src, tcl(), &reg());
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

    // Semantic tokens read command grammar from the registry,
    // so the explicitly global spellings C Tcl resolves to the same commands
    // (`namespace which -command ::format` → `::format`) are classified
    // identically to their bare forms, and a same-named user proc is not.

    /// The decoded token *kinds* of `src`, positions discarded.
    fn kinds_only(src: &str, registry: &CommandRegistry) -> Vec<u32> {
        decode_full(src, tcl(), registry)
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

    /// The loop-variable-list scan is registry-driven,
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
        // same roles.
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
            static_loop_var_names(
                src,
                &command,
                1,
                tcl_syntax::word_rules::WordValueRules::default(),
            )
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
        let user = decode_full("ns::format {%08x} 42\n", tcl(), &r);
        let builtin = decode_full("format {%08x} 42\n", tcl(), &r);
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

    // A head's *effective command identity* (a static
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
    /// apply-lambda-body KCS note: without them a `[list …]`-quoted or
    /// directly-called lambda under `rename apply myapply` collapses into one
    /// opaque `string` token.
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
        let dynamic = decode_full("{*}$cmd {%08x} 42\n", tcl(), &r);
        let builtin = decode_full("format {%08x} 42\n", tcl(), &r);
        assert!(
            dynamic.len() < builtin.len(),
            "a dynamic head must not get format sub-tokens: {dynamic:?}"
        );
    }

    /// `set`, `lassign`, `incr`, `lappend`, `append`, `expr` (every plain
    /// builtin — `function` + `defaultLibrary`) render as unstyled plain text
    /// for users whose theme has no rule for the custom `support.function.tcl`
    /// scope. A `semanticTokenScopes` override mapping
    /// `function.defaultLibrary` to that scope **replaces** (not supplements)
    /// VS Code's built-in cross-theme default for the standard
    /// `function`/`defaultLibrary` combo — so themes lacking that exact scope
    /// lose highlighting entirely instead of falling back to the built-in
    /// default the way every other standard type does. `operator`, `decorator`
    /// and `namespace` carry the same risk for the same reason (and
    /// `operator`'s scope, `keyword.operator.format.tcl`, is outright wrong for
    /// the general case — it covers every `expr` operator and the `regsub` `\&`
    /// backref, not just `format`). Standard LSP types get no
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

    /// `insert_format_overrides` marks a format/clock/binary-role
    /// argument by its registry-declared *position*, independent of
    /// whether that word is literal — so a computed word (`$fmt`) reaches
    /// the sub-tokeniser exactly as a literal one does, and each
    /// sub-tokeniser already "falls back to the default classification"
    /// (its own doc comment, `ArgOverride`) when it finds no specifier in
    /// the token's own bytes. `$fmt`'s own text is `"$fmt"`, never a `%`
    /// specifier, so it renders as a plain `variable` token — a computed
    /// pattern is explained at its use through hover and inlay hints
    /// (`hover.rs`/`inlay_hints.rs`), never painted here at a token
    /// range it does not have.
    #[test]
    fn a_computed_format_word_falls_back_to_its_plain_classification() {
        let names = legend_token_types();
        let kind_of = |src: &str, needle: &str| {
            decode_words(src, &reg())
                .into_iter()
                .find(|(_, _, _, _, word)| word == needle)
                .map(|(_, _, _, kind, _)| names[kind as usize])
        };
        assert_eq!(
            kind_of("set fmt \"%-20s %d\"\nformat $fmt a 1\n", "$fmt"),
            Some("variable"),
            "a computed format argument is an ordinary variable token, not formatSpec"
        );
        assert_eq!(
            kind_of(
                "set fmt \"%Y-%m-%d\"\nclock format 0 -format $fmt\n",
                "$fmt"
            ),
            Some("variable")
        );
        assert_eq!(
            kind_of("set fmt \"a3 i\"\nbinary format $fmt foo 1\n", "$fmt"),
            Some("variable")
        );
    }
}

#[cfg(test)]
mod original_proc_role_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_roles_keep_opaque_declarations_distinct_after_ui_is_cleared() {
        let source = "proc p\\uD800 {v} {upvar 1 $v local; set local 1}\nproc p\\uD801 {v} {upvar 1 $v local; set local}\np\\uD800 written\np\\uD801 read\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let roles = VarNameArgRoles::from_analysis(&analysis);
        assert_eq!(roles.original.len(), 2);
        analysis.all_procs.clear();
        analysis.superseded_procs.clear();
        let write_at = u32::try_from(source.rfind("p\\uD800 written").unwrap()).unwrap();
        let read_at = u32::try_from(source.rfind("p\\uD801 read").unwrap()).unwrap();
        let write = roles
            .original_roles_at(&analysis, source, write_at)
            .unwrap();
        let read = roles.original_roles_at(&analysis, source, read_at).unwrap();
        assert_eq!(write.write, [0]);
        assert_eq!(read.read, [0]);
        assert!(read.write.is_empty());
        assert_ne!(write.name.slot(), read.name.slot());
        assert!(
            roles
                .original_roles_at(&analysis, &format!("{source} "), write_at)
                .is_none()
        );
    }

    #[test]
    fn original_role_lookup_requires_current_full_source_and_own_head_input() {
        let source = "proc p {value} {return $value}; p ordinary";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let roles = VarNameArgRoles::from_analysis(&analysis);
        let offset = u32::try_from(source.rfind("p ordinary").unwrap()).unwrap();
        assert!(roles.original_roles_at(&analysis, source, offset).is_some());
        assert!(
            roles
                .original_roles_at(&analysis, &format!("{source} "), offset)
                .is_none()
        );
        let invocation = analysis
            .command_invocations
            .iter_mut()
            .find(|invocation| invocation.range.start() == offset)
            .unwrap();
        let input = invocation.original_name_input.take();
        assert!(roles.original_roles_at(&analysis, source, offset).is_none());
        analysis
            .command_invocations
            .iter_mut()
            .find(|invocation| invocation.range.start() == offset)
            .unwrap()
            .original_name_input = input;
        analysis.body_lexer_config = None;
        assert!(roles.original_roles_at(&analysis, source, offset).is_none());
    }

    #[test]
    fn original_role_lookup_respects_namespace_shadow_and_ordinary_arguments() {
        let source = "proc touch {v} {upvar 1 $v local; set local 1}\nnamespace eval N {proc touch {v} {return $v}; touch ordinary}\ntouch declared\nputs touch\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let roles = VarNameArgRoles::from_analysis(&analysis);
        let local_at = u32::try_from(source.find("touch ordinary").unwrap()).unwrap();
        let global_at = u32::try_from(source.find("touch declared").unwrap()).unwrap();
        assert!(
            roles
                .original_roles_at(&analysis, source, local_at)
                .unwrap()
                .write
                .is_empty()
        );
        assert_eq!(
            roles
                .original_roles_at(&analysis, source, global_at)
                .unwrap()
                .write,
            [0]
        );
        let data_at = u32::try_from(source.rfind("touch").unwrap()).unwrap();
        assert!(
            roles
                .original_roles_at(&analysis, source, data_at)
                .is_none()
        );
    }
}

#[cfg(test)]
mod original_procedure_topology_colour_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_procedure_colours_map_formal_roles_to_their_actual_expansion_children() {
        // naming.source.original-procedure-argument-topology
        // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
        for source in [
            "proc target {fixed variable rest} {upvar 1 $variable local; set local 1}; target {*}{FIXED destination} ordinary",
            "proc target {fixed variable rest} {upvar 1 $variable local; set local 1}; interp alias {} alias {} target FIXED; alias {*}{destination} ordinary",
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            let declaration = analysis.original_procedure_declarations().next().unwrap();
            assert_eq!(
                proc_var_write_indices(declaration.metadata()),
                [1],
                "independent original formal-role prerequisite"
            );
            analysis.all_procs.clear();
            analysis.global_scope.procs.clear();
            analysis.command_invocations.clear();
            let tokens = tests::decode_semantic(&full_with_cu_and_analysis(
                source,
                analysis.resolved_profile().unwrap(),
                analysis.resolved_registry().unwrap(),
                None,
                Some(&analysis),
            ));
            let index = LineIndex::new(source);
            let expected = index.position_at_utf16(
                u32::try_from(source.rfind("destination").unwrap()).unwrap(),
                source,
            );
            assert!(
                tokens
                    .iter()
                    .any(|&(line, column, _, kind, mods)| line == expected.line
                        && column == expected.character.get()
                        && kind == TokenKind::Variable as u32
                        && mods == MOD_DECLARATION)
            );
            let ordinary = index.position_at_utf16(
                u32::try_from(source.rfind("ordinary").unwrap()).unwrap(),
                source,
            );
            assert!(
                !tokens
                    .iter()
                    .any(|&(line, column, _, kind, mods)| line == ordinary.line
                        && column == ordinary.character.get()
                        && kind == TokenKind::Variable as u32
                        && mods == MOD_DECLARATION)
            );
        }
    }
}

#[cfg(test)]
mod original_realm_consumer_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_semantic_consumer_declines_missing_realm_without_nominal_recapture() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "set value 1; proc p {} {return $value}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let profile = analysis.resolved_profile().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let entries = collect_entries(
            source,
            profile,
            registry,
            None,
            WorkspaceTokenFacts {
                analysis: Some(&analysis),
                ..WorkspaceTokenFacts::default()
            },
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.3 == TokenKind::Variable && entry.4 & MOD_DECLARATION != 0)
        );
        let mut unowned = AnalysisResult::default();
        unowned.resolved_input = analysis.resolved_input.clone();
        unowned.body_lexer_config = analysis.body_lexer_config;
        unowned.all_procs.clone_from(&analysis.all_procs);
        unowned.all_variables.clone_from(&analysis.all_variables);
        assert!(unowned.retained_command_realm().is_none());
        assert!(
            collect_entries(
                source,
                profile,
                registry,
                None,
                WorkspaceTokenFacts {
                    analysis: Some(&unowned),
                    ..WorkspaceTokenFacts::default()
                }
            )
            .is_empty()
        );
    }
}

#[cfg(test)]
mod original_definition_vocabulary_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn retained_tokens(source: &str, dialect: &str) -> Vec<(u32, u32, u32, u32, u32)> {
        let analysis = Analyser::new().analyse(source, dialect);
        tests::decode_semantic(&full_with_cu_and_analysis(
            source,
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
            None,
            Some(&analysis),
        ))
    }

    fn token_kind_at(
        source: &str,
        tokens: &[(u32, u32, u32, u32, u32)],
        offset: usize,
    ) -> Option<u32> {
        let position =
            LineIndex::new(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
        tokens.iter().find_map(|&(line, column, _, kind, _)| {
            (line == position.line && column == position.character.get()).then_some(kind)
        })
    }

    #[test]
    fn original_definition_colours_use_genuine_wrappers_and_clear_ordinary_bodies() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = "oo::class create C {\nself self {method café {} {puts wrapper}}\nif 1 {method café {} {puts immediate}}\nmethod plain {} {method ordinary {} {puts inert}}\nproc p {} {method ordinary {} {puts inert}}\napply {{} {method ordinary {} {puts inert}}}\n}";
        let tokens = retained_tokens(source, "tcl8.6");
        for (offset, _) in source.match_indices("method café") {
            assert_eq!(
                token_kind_at(source, &tokens, offset),
                Some(TokenKind::Keyword as u32)
            );
        }
        for word in ["puts wrapper", "puts immediate"] {
            assert_eq!(
                token_kind_at(source, &tokens, source.find(word).unwrap()),
                Some(TokenKind::Function as u32),
                "{word}: {tokens:?}"
            );
        }
        for (offset, _) in source.match_indices("method ordinary") {
            assert_eq!(
                token_kind_at(source, &tokens, offset),
                Some(TokenKind::Function as u32)
            );
        }
        for (offset, _) in source.match_indices("puts inert") {
            assert_ne!(
                token_kind_at(source, &tokens, offset),
                Some(TokenKind::Function as u32)
            );
        }
    }

    #[test]
    fn original_definition_keyword_colours_preserve_whole_escaped_source_extent() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        let source = r"oo::class create C {metho\u0064 café {} {puts visible}}";
        let tokens = retained_tokens(source, "tcl8.6");
        let start = source.find("metho").unwrap();
        let position =
            LineIndex::new(source).position_at_utf16(u32::try_from(start).unwrap(), source);
        let head = tokens
            .iter()
            .find(|&&(line, column, _, kind, _)| {
                line == position.line
                    && column == position.character.get()
                    && kind == TokenKind::Keyword as u32
            })
            .unwrap();
        assert_eq!(
            head.2, 11,
            "the raw escape stays within the one original word"
        );
        assert_eq!(
            token_kind_at(source, &tokens, source.find("puts").unwrap()),
            Some(TokenKind::Function as u32)
        );
        assert!(
            !tokens
                .iter()
                .any(|&(line, column, _, _, _)| line == position.line
                    && position.character.get() < column
                    && column < position.character.get() + head.2)
        );
    }

    #[test]
    fn original_definition_colours_keep_keyword_body_and_release_applicability_separate() {
        // naming.core.original-command-source-schema
        // docs/design/analysis/name-resolution-proofs/original-command-source-schema.md
        for (source, dialect, keyword, body) in [
            (
                "oo::class create C {method café $params {puts hidden}}",
                "tcl8.6",
                true,
                false,
            ),
            (
                "oo::class create C {method café -private {} {puts visible}}",
                "tcl9.0",
                true,
                true,
            ),
            (
                "oo::class create C {method café -private {} {puts hidden}}",
                "tcl8.6",
                true,
                false,
            ),
            (
                r#"oo::class create C {method café {} "\u0070uts hidden"}"#,
                "tcl8.6",
                true,
                false,
            ),
            (
                "oo::class create C {property p -get {puts hidden}}",
                "tcl8.6",
                false,
                false,
            ),
            (
                "oo::class create C {property p -get {puts visible}}",
                "tcl9.1",
                true,
                true,
            ),
        ] {
            let tokens = retained_tokens(source, dialect);
            let offset = source
                .find("method")
                .or_else(|| source.find("property"))
                .unwrap();
            assert_eq!(
                token_kind_at(source, &tokens, offset) == Some(TokenKind::Keyword as u32),
                keyword,
                "{dialect}: {source}: {tokens:?}"
            );
            if let Some(offset) = source.find("puts") {
                assert_eq!(
                    token_kind_at(source, &tokens, offset) == Some(TokenKind::Function as u32),
                    body,
                    "{dialect}: {source}: {tokens:?}"
                );
            } else {
                assert!(
                    !tokens.iter().any(|&(line, column, _, kind, _)| {
                        let begin = source.find("\\u0070uts").unwrap();
                        let position = LineIndex::new(source)
                            .position_at_utf16(u32::try_from(begin).unwrap(), source);
                        line == position.line
                            && column == position.character.get()
                            && kind == TokenKind::Function as u32
                    }),
                    "cooked source has no original executable body extent"
                );
            }
        }
    }
}

#[cfg(test)]
mod selected_source_token_tests {
    use super::*;

    #[test]
    fn original_loop_handle_advice_keeps_actual_availability_captures_and_inert_data() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let mut registry = CommandRegistry::build_default();
        let mut each = registry.get("foreach").unwrap().clone();
        each.name = "selected_each";
        each.surface = Some(tcl_registry::model::SpecSurface::TCL90_PLUS);
        registry.insert(each);
        let registry = std::sync::Arc::new(registry);
        let source = "interp alias {} each {} selected_each {left right}
set data {selected_each bogus $items {}}
each $items {}";
        let current = crate::refactor::test_logical_analysis(source, "tcl9.0", registry.clone());
        let collections = ObjectClassMap::from([(
            String::from("items"),
            std::collections::HashSet::from([String::from("::Element")]),
        )]);
        let mut handles = ObjectClassMap::default();
        augment_loop_var_handles(source, &current, &collections, &mut handles);
        assert!(handles.contains_key("left"));
        assert!(handles.contains_key("right"));
        assert!(!handles.contains_key("bogus"));
        let old = crate::refactor::test_logical_analysis(source, "tcl8.6", registry.clone());
        handles.clear();
        augment_loop_var_handles(source, &old, &collections, &mut handles);
        assert!(handles.is_empty());
        let replaced = "proc selected_each args {}
selected_each hidden $items {}";
        let analysis = crate::refactor::test_logical_analysis(replaced, "tcl9.0", registry);
        augment_loop_var_handles(replaced, &analysis, &collections, &mut handles);
        assert!(handles.is_empty());
    }

    #[test]
    fn original_member_handle_advice_uses_the_genuine_factory_member_body() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "snit::type Engine {method run {} {}}
snit::type Wrapper {constructor {} {install item using Engine $win.a; install unknown using $type instance; proc nested {} {install borrowed using Engine instance}}}
set inert {install leaked using Engine instance}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let regions = selected_member_handle_regions(source, &analysis, &context);
        assert!(regions.iter().any(|(span, _)| {
            source[span.as_range()].starts_with("install item using Engine $win.a")
        }));
        let mut handles = ObjectClassMap::default();
        augment_snit_handles(
            source,
            &analysis,
            Some(analysis.class_hierarchy()),
            &mut handles,
        );
        assert!(handles.contains_key("item"));
        assert!(!handles.contains_key("leaked"));
        assert!(!handles.contains_key("unknown"));
        assert!(!handles.contains_key("borrowed"));
        let changed = source.replace("Engine instance", "Engine other");
        handles.clear();
        augment_snit_handles(
            &changed,
            &analysis,
            Some(analysis.class_hierarchy()),
            &mut handles,
        );
        assert!(handles.is_empty());
    }

    #[test]
    fn original_semantic_tokens_withdraw_stale_grammar_foreign_and_missing_source_owner() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let source = "set x 1
puts $x";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let registry = CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let tokens = |source: &str, analysis: &AnalysisResult| {
            full_with_cu_and_analysis(source, profile, &registry, None, Some(analysis))
        };
        assert!(!tokens(source, &analysis).data.is_empty());
        assert!(
            tokens(
                "set y 1
puts $y",
                &analysis
            )
            .data
            .is_empty()
        );
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(tokens(source, &analysis).data.is_empty());
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::resolve_environment("tcl8.4").default_context_registry(),
            config,
        ));
        assert!(tokens(source, &analysis).data.is_empty());
        analysis.resolved_input = None;
        assert!(tokens(source, &analysis).data.is_empty());
    }
}
