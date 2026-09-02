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

//! Catalogues of the registry's enum and bitflag vocabularies.
//!
//! The studio's form is generated from [`crate::schema`], which points at the
//! catalogues here for every field whose value is one of a fixed set. Each
//! catalogue pairs the Rust variant spelling (which is also what the `.rs`
//! renderer emits) with a one-line description shown in the picker.
//!
//! ## Keeping a catalogue honest
//!
//! Every catalogue over a plain (fieldless) enum has a `covered` witness in
//! the test module: an exhaustive `match` that fails to compile the moment a
//! variant is added to the registry. Its doc comment names the catalogue to
//! update alongside, so a new `ArgRole` / hook / side-effect target cannot
//! silently go missing from the studio's picker.

use std::sync::LazyLock;

use tcl_dialect::model::SpecSurface;
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::taint::{TaintColour, TaintColourAtom};
use tcl_registry::traits::{Trait, Traits};

/// One selectable value in a picker — the Rust spelling plus a one-line
/// description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    /// Rust variant spelling, exactly as the `.rs` renderer emits it
    /// (`"VarWrite"`, `"BYTE_COMPILED"`, `"tcl9.0"`).
    pub key: &'static str,
    /// One-line description for the picker.
    pub doc: &'static str,
}

/// Shorthand for a catalogue entry.
const fn v(key: &'static str, doc: &'static str) -> Variant {
    Variant { key, doc }
}

/// [`ArgRole`] — how a consumer should treat an argument position.
pub const ARG_ROLES: &[Variant] = &[
    v("Body", "Tcl script body, recursed into by the analyser"),
    v("Expr", "expr sub-language expression"),
    v("VarWrite", "names a variable the command writes"),
    v("VarRead", "names a variable the command reads"),
    v("LoopVarList", "loop variable list (foreach / lmap)"),
    v("ParamList", "procedure parameter list"),
    v("Name", "symbolic name (proc, namespace, class)"),
    v("Pattern", "glob or regex pattern"),
    v("Option", "option flag word"),
    v("Value", "generic value — the default"),
    v("Subcommand", "ensemble subcommand keyword"),
    v("OptionTerminator", "the `--` end-of-options marker"),
    v("FormatString", "format template (format / puts style)"),
    v("ScanFormat", "scan conversion template"),
    v("Channel", "I/O channel handle"),
    v("Index", "list or string index expression"),
    v("Keyword", "fixed keyword word (`in`, `from`, `to`)"),
    v(
        "CommandPrefix",
        "callback command prefix the command appends to",
    ),
    v("CommandName", "names a command that must exist"),
    v(
        "CommandNameProbe",
        "names a command that need not exist yet",
    ),
    v("LambdaLiteral", "an `apply`-style lambda literal"),
    v(
        "NamespaceName",
        "names a namespace (`namespace children ::ns`)",
    ),
    v(
        "Boolean",
        "consumed as a boolean (`Tcl_GetBoolean` spellings)",
    ),
    v(
        "NumericOrBoolean",
        "consumed as a number or a boolean (`-validate 0`/`yes`)",
    ),
    v("Result", "becomes the command's own result (`return $w`)"),
];

/// [`TclType`] — the intrep a value carries.
pub const TCL_TYPES: &[Variant] = &[
    v("String", "pure string, no cached intrep"),
    v("Int", "integer"),
    v("Double", "double-precision float"),
    v("Boolean", "boolean"),
    v("List", "Tcl list"),
    v("Dict", "Tcl dict"),
    v("ByteArray", "byte array (binary data)"),
    v("Numeric", "abstract join of Int and Double"),
    v("Object", "TclOO object instance"),
    v("Channel", "I/O channel handle"),
];

/// [`BodyKind`] — whether a `Body` argument runs in the caller's frame.
pub const BODY_KINDS: &[Variant] = &[
    v(
        "Plain",
        "runs in the caller's frame — joins the enclosing data flow",
    ),
    v(
        "Structural",
        "runs in a separate definition / dispatch context — opts out of the enclosing data flow",
    ),
];

/// [`ScriptTiming`] — when a script-valued argument is evaluated.
pub const SCRIPT_TIMINGS: &[Variant] = &[
    v(
        "SameInvocation",
        "may run before the receiving invocation returns",
    ),
    v(
        "Deferred",
        "stored by the receiving invocation for a later callback",
    ),
    v(
        "ReferenceOnly",
        "identified for registration matching or lookup, but not invoked",
    ),
];

/// [`VariableScope`] — frame used to resolve a variable-name option value.
pub const VARIABLE_SCOPES: &[Variant] = &[
    v(
        "CurrentFrame",
        "unqualified names resolve in the invocation's current Tcl frame",
    ),
    v(
        "Global",
        "unqualified names resolve from the interpreter's global namespace",
    ),
];

/// [`ArgPresentation`] — how a formatter lays an argument out.
pub const ARG_PRESENTATIONS: &[Variant] = &[
    v(
        "BlockScript",
        "expanded onto its own indented lines (the default for a body argument)",
    ),
    v(
        "InlineScript",
        "kept on the command's own line — `for`'s start / next scripts",
    ),
];

/// [`StorageType`] — the container a written variable is inferred to hold.
pub const STORAGE_TYPES: &[Variant] = &[
    v("Dict", "the target variable holds a dict"),
    v("List", "the target variable holds a list"),
    v("Array", "the target is a Tcl array"),
];

/// [`ByteArrayEffect`] — how a command transforms a binary operand (S110).
pub const BYTE_ARRAY_EFFECTS: &[Variant] = &[
    v("None", "no byte-array relationship"),
    v("Transparent", "passes bytes through unchanged"),
    v(
        "Coerces",
        "forces the operand to a string, losing the byte array",
    ),
    v("CaseFolds", "case-folds, which is lossy for binary data"),
    v(
        "Encodes",
        "re-encodes the operand into a text representation",
    ),
    v(
        "Rebinarifies { value_arg: 0 }",
        "reinstalls a byte-array rep on the operand at the given index",
    ),
];

/// [`CommandTableEffect`] — how a command rebinds command names.
pub const COMMAND_TABLE_EFFECTS: &[Variant] = &[
    v("DefinesProcedure", "defines a new procedure (`proc`)"),
    v("RenamesCommands", "moves or deletes a command (`rename`)"),
    v("CreatesAliases", "creates an alias (`interp alias`)"),
];

/// [`PatternType`] — the pattern language a pattern argument uses.
pub const PATTERN_TYPES: &[Variant] = &[
    v("Glob", "glob pattern (`string match`, `lsearch -glob`)"),
    v("Regex", "regular expression (`regexp`, `regsub`)"),
];

/// [`FormatType`] — the format-string language a format argument uses.
pub const FORMAT_TYPES: &[Variant] = &[
    v("Sprintf", "printf-style template (`format`, `scan`)"),
    v("Clock", "clock format / scan template"),
    v("Binary", "binary format / scan template"),
    v("Regsub", "regsub replacement template"),
];

/// [`FormKind`] — classification of an invocation form.
pub const FORM_KINDS: &[Variant] = &[
    v("Default", "the default form"),
    v("Getter", "read-only getter form"),
    v("Setter", "modifying setter form"),
];

/// [`DefinedSymbolKind`] — the outline category a symbol-definer binds.
pub const DEFINED_SYMBOL_KINDS: &[Variant] = &[
    v("Test", "a test case (`tcltest::test NAME …`)"),
    v(
        "Constraint",
        "a named test constraint (`tcltest::testConstraint NAME`)",
    ),
    v(
        "Matcher",
        "a custom result matcher (`tcltest::customMatch MODE command`)",
    ),
    v(
        "Event",
        "an event handler named for its event (iRules `when EVENT { … }`)",
    ),
];

/// [`SideEffectTarget`] — generated from the registry-owned names and prose.
pub static SIDE_EFFECT_TARGETS: LazyLock<Vec<Variant>> = LazyLock::new(|| {
    SideEffectTarget::ALL
        .iter()
        .map(|item| v(item.name(), item.summary()))
        .collect()
});

/// [`ConnectionSide`] — which side of an iRules connection an effect applies to.
pub const CONNECTION_SIDES: &[Variant] = &[
    v("None", "not iRules, or side-neutral"),
    v("Client", "client side"),
    v("Server", "server side"),
    v("Both", "both client and server sides"),
    v("Global", "connection-independent"),
];

/// [`LoweringHookId`] — the per-command lowering specialisation.
pub const LOWERING_HOOKS: &[Variant] = &[
    v("Expr", "expr"),
    v("Return", "return"),
    v("Set", "set"),
    v("Incr", "incr"),
    v("AppendOrLappend", "append / lappend"),
    v("Unset", "unset"),
    v("Global", "global"),
    v("Variable", "variable"),
    v("Upvar", "upvar"),
    v("Proc", "proc"),
    v("When", "iRules when"),
    v("NamespaceEval", "namespace eval"),
    v("If", "if"),
    v("Switch", "switch"),
    v("For", "for"),
    v("While", "while"),
    v("Foreach", "foreach"),
    v("Lmap", "lmap"),
    v("ForeachLine", "foreach-line reader idiom"),
    v("Catch", "catch"),
    v("Try", "try"),
    v("Dict", "dict"),
    v("Eval", "eval"),
    v("Uplevel", "uplevel"),
    v("Apply", "apply"),
    v("ArrayFor", "array for"),
];

/// [`CodegenHookId`] — the per-command `TclVM` bytecode emitter.
pub const CODEGEN_HOOKS: &[Variant] = &[
    v("Lassign", "lassign"),
    v("Llength", "llength"),
    v("Lrange", "lrange"),
    v("Linsert", "linsert"),
    v("Lset", "lset"),
    v("Dict", "dict"),
    v("Array", "array"),
    v("Namespace", "namespace"),
    v("Append", "append"),
    v("Lappend", "lappend"),
    v("Unset", "unset"),
    v("Tailcall", "tailcall"),
    v("Concat", "concat"),
    v("Global", "global"),
    v("Upvar", "upvar"),
];

/// [`InlineCodegenHookId`] — the value-position / catch-body emitter.
pub const INLINE_CODEGEN_HOOKS: &[Variant] = &[
    v("Expr", "expr"),
    v("Incr", "incr"),
    v("InfoExists", "info exists"),
    v("String", "string"),
    v("Lindex", "lindex"),
    v("Lrange", "lrange"),
    v("Lreplace", "lreplace"),
    v("Linsert", "linsert"),
    v("Regexp", "regexp"),
    v("List", "list"),
    v("Array", "array"),
    v("DictGet", "dict get"),
    v("Catch", "catch"),
    v("Return", "return"),
    v("Error", "error"),
    v("Break", "break"),
    v("Continue", "continue"),
    v("Try", "try"),
];

/// [`ReturnTypeHookId`] — the algorithm that types a call whose result shape
/// moves with the call.
pub const RETURN_TYPE_HOOKS: &[Variant] = &[
    v("Regexp", "regexp -inline / -about"),
    v("Lsearch", "lsearch -all / -inline / -subindices"),
    v("Regsub", "regsub with or without varName"),
    v("Scan", "scan inline or variable-writing"),
    v("Pid", "pid with or without fileId"),
];

/// [`AnalyserHookId`] — the per-command analyser handler family.
pub const ANALYSER_HOOKS: &[Variant] = &[
    v("Set", "set"),
    v("Variable", "variable"),
    v("Global", "global"),
    v("Proc", "proc"),
    v("OptProc", "argparse-style proc"),
    v("Apply", "apply"),
    v("Uplevel", "uplevel"),
    v("NamespaceEval", "namespace eval"),
    v("NamespaceEnsemble", "namespace ensemble"),
    v("NamespaceImport", "namespace import"),
    v("NamespaceExport", "namespace export"),
    v("NamespaceForget", "namespace forget"),
    v("NamespacePath", "namespace path"),
    v("NamespaceUnknown", "namespace unknown"),
    v("NamespaceUpvar", "namespace upvar"),
    v("Foreach", "foreach"),
    v("For", "for"),
    v("Switch", "switch"),
    v("Catch", "catch"),
    v("Try", "try"),
    v("Upvar", "upvar"),
    v("DictFor", "dict for"),
    v("DictUpdate", "dict update"),
    v("DictWith", "dict with"),
    v("InterpAlias", "interp alias"),
    v("InterpEval", "interp eval"),
    v("InterpCreate", "interp create"),
    v("InterpDelete", "interp delete"),
    v("InterpHide", "interp hide"),
    v("InterpExpose", "interp expose"),
    v("Rename", "rename"),
    v("OoDefine", "oo::define"),
    v("OoObjdefine", "oo::objdefine"),
    v("PackageRequire", "package require"),
    v("PackageProvide", "package provide"),
    v("PackageIfneeded", "package ifneeded"),
    v("PackagePrefer", "package prefer"),
    v("Source", "source"),
    v("Append", "append"),
    v("Lappend", "lappend"),
    v("RegexPatternCapture", "regexp capture binding"),
    v("Incr", "incr"),
    v("Load", "load"),
];

/// [`Traits`] bits, generated from the authoritative registry declaration.
///
/// A new trait cannot be absent from this catalogue: [`Trait::ALL`] supplies
/// the rows, while the exhaustive [`Trait::summary`] and [`Trait::category`]
/// methods make missing author-facing metadata a compile error.
pub static TRAITS: LazyLock<Vec<Variant>> = LazyLock::new(|| {
    Trait::ALL
        .iter()
        .map(|item| v(item.name(), item.summary()))
        .collect()
});

/// [`TaintColour`] bits, generated from the registry-owned atoms and prose.
pub static TAINT_COLOURS: LazyLock<Vec<Variant>> = LazyLock::new(|| {
    TaintColourAtom::ALL
        .iter()
        .map(|item| v(item.name(), item.summary()))
        .collect()
});

/// Canonical dialect name ↔ its surface rows — the vocabulary
/// [`DIALECTS`] and [`dialect_surface`] both read.
///
/// The names are the catalogue's own dialect ids, so a draft round-trips
/// through the registry's vocabulary rather than a parallel spelling. The EDA
/// shells are deliberately absent, being a base Tcl version plus
/// package-gated command libraries rather than surfaces of their own.
const DIALECT_SURFACES: &[(&str, &[SpecSurface])] = &[
    ("tcl8.4", SpecSurface::TCL84),
    ("tcl8.5", SpecSurface::TCL85),
    ("tcl8.6", SpecSurface::TCL86),
    ("tcl9.0", SpecSurface::TCL90),
    ("tcl9.1", SpecSurface::TCL91),
    ("f5-irules", SpecSurface::IRULES),
    ("f5-iapps", SpecSurface::IAPPS),
    ("tk", SpecSurface::TK),
    ("expect", SpecSurface::EXPECT),
    ("bpf", SpecSurface::BPF),
    ("f5-tmsh", SpecSurface::TMSH),
    ("f5-bigip", SpecSurface::BIGIP),
    ("spectcl", SpecSurface::SPECTCL),
    ("sslictcl", SpecSurface::SSLICTCL),
];

/// Labels for the bits with no profile in the dialect catalogue. `tk` is the
/// only one: it is a library pin rather than a selectable profile, so
/// `DialectProfile::find` never resolves it.
const BIT_ONLY_LABELS: &[(&str, &str)] = &[("tk", "Tk")];

/// Surface rows, by canonical dialect name.
///
/// Labelled from the dialect catalogue, so a profile's display name is the one
/// the studio's picker shows and renaming a dialect renames it here too.
pub static DIALECTS: LazyLock<Vec<Variant>> = LazyLock::new(|| {
    DIALECT_SURFACES
        .iter()
        .map(|(name, _)| {
            let label = crate::environment::catalogue_profile_for_dialect(name).map_or_else(
                || {
                    BIT_ONLY_LABELS
                        .iter()
                        .find(|(bit, _)| bit == name)
                        .map_or(*name, |(_, label)| *label)
                },
                |profile| profile.display_name,
            );
            v(name, label)
        })
        .collect()
});

/// The surface rows for a canonical dialect name from
/// [`DIALECTS`].
#[must_use]
pub fn dialect_surface(name: &str) -> Option<&'static [SpecSurface]> {
    DIALECT_SURFACES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
}

/// The [`Traits`] bit for a catalogue key from [`TRAITS`].
///
/// Resolved against the registry's own flag list, so there is no second table
/// here to drift out of step with it.
#[must_use]
pub fn trait_bit(key: &str) -> Option<Traits> {
    Trait::ALL
        .iter()
        .find(|t| t.name() == key)
        .map(|t| Traits::of(&[*t]))
}

/// The catalogue key for each [`Traits`] flag the spec carries, in
/// declaration order.
///
/// Each flag is a distinct enum variant, so a name maps to exactly one bit and
/// a bit to exactly one name, and no deduplication by bit value is needed. Two
/// flags sharing a bit would render a spec claiming a trait its author never
/// set; deriving each bit from an enum discriminant makes that collision
/// unrepresentable.
#[must_use]
pub fn trait_keys(traits: Traits) -> Vec<&'static str> {
    traits.iter_names().collect()
}

/// The [`TaintColour`] bit for a catalogue key from [`TAINT_COLOURS`].
#[must_use]
pub fn taint_bit(key: &str) -> Option<TaintColour> {
    TAINT_BITS.iter().find(|(n, _)| *n == key).map(|(_, b)| *b)
}

/// The catalogue keys for each [`TaintColour`] bit `colour` carries.
#[must_use]
pub fn taint_keys(colour: TaintColour) -> Vec<&'static str> {
    TAINT_BITS
        .iter()
        .filter(|(_, bit)| colour.contains(*bit))
        .map(|(name, _)| *name)
        .collect()
}

/// Key ↔ bit table backing [`taint_bit`] / [`taint_keys`].
const TAINT_BITS: &[(&str, TaintColour)] = &[
    ("TAINTED", TaintColour::TAINTED),
    ("PATH_PREFIXED", TaintColour::PATH_PREFIXED),
    ("NON_DASH_PREFIXED", TaintColour::NON_DASH_PREFIXED),
    ("CRLF_FREE", TaintColour::CRLF_FREE),
    ("SHELL_ATOM", TaintColour::SHELL_ATOM),
    ("LIST_CANONICAL", TaintColour::LIST_CANONICAL),
    ("REGEX_LITERAL", TaintColour::REGEX_LITERAL),
    ("PATH_NORMALISED", TaintColour::PATH_NORMALISED),
    ("PATH_BOUNDED", TaintColour::PATH_BOUNDED),
    ("HEADER_TOKEN_SAFE", TaintColour::HEADER_TOKEN_SAFE),
    ("HTML_ESCAPED", TaintColour::HTML_ESCAPED),
    ("URL_ENCODED", TaintColour::URL_ENCODED),
    ("IP_ADDRESS", TaintColour::IP_ADDRESS),
    ("PORT", TaintColour::PORT),
    ("FQDN", TaintColour::FQDN),
    ("PATH_JOINED", TaintColour::PATH_JOINED),
    ("CHANNEL", TaintColour::CHANNEL),
];

/// The Rust variant spelling of a fieldless registry enum value.
///
/// Every enum reached here derives `Debug`, whose output for a fieldless
/// variant *is* its Rust spelling — so seeding a draft from a live spec needs
/// no parallel name table, only the catalogues above (which exist for the
/// picker).
pub fn variant_name<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// [`variant_name`] prefixed with its enum's type path, for a field the studio
/// edits as a raw Rust expression rather than a picker.
///
/// A field whose editor is a picker gets its type prefix from the schema at
/// render time, but a `RustExpr` field is emitted verbatim — so a seeded value
/// has to carry the path itself, or the rendered spec fails to compile with a
/// bare `ListifiesDictValue`. Struct-variant payloads survive too, since
/// `Debug` prints them in Rust literal form:
/// `VarElementsEffect::AppendsListElements { values_from: 1 }`.
pub fn qualified_variant<T: std::fmt::Debug>(type_name: &str, value: &T) -> String {
    format!("{type_name}::{}", variant_name(value))
}

/// [`qualified_variant`] wrapped in `Some(…)`, for an `Option`-typed field the
/// studio edits as a raw Rust expression.
///
/// A `RustExpr` field is emitted exactly as written, so the `Some` has to be
/// part of the value — the renderer has no type information to add it.
pub fn optional_variant<T: std::fmt::Debug>(type_name: &str, value: &T) -> String {
    format!("Some({})", qualified_variant(type_name, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::arg_role::ArgRole;
    use tcl_registry::body_kind::BodyKind;
    use tcl_registry::byte_array_effect::ByteArrayEffect;
    use tcl_registry::command_table::CommandTableEffect;
    use tcl_registry::hooks::{AnalyserHookId, CodegenHookId, InlineCodegenHookId, LoweringHookId};
    use tcl_registry::hover::{FormKind, ScriptTiming, VariableScope};
    use tcl_registry::patterns::{FormatType, PatternType};
    use tcl_registry::side_effects::{ConnectionSide, SideEffectTarget, StorageType};
    use tcl_registry::symbol_def::DefinedSymbolKind;
    use tcl_registry::types::TclType;

    /// Witness that [`ARG_ROLES`] covers every [`ArgRole`].
    ///
    /// Adding a variant breaks this match — add it to `ARG_ROLES` too.
    fn covered_arg_role(role: ArgRole) -> bool {
        match role {
            ArgRole::Body
            | ArgRole::Expr
            | ArgRole::VarWrite
            | ArgRole::VarRead
            | ArgRole::LoopVarList
            | ArgRole::ParamList
            | ArgRole::Name
            | ArgRole::Pattern
            | ArgRole::Option
            | ArgRole::Value
            | ArgRole::Subcommand
            | ArgRole::OptionTerminator
            | ArgRole::FormatString
            | ArgRole::ScanFormat
            | ArgRole::Channel
            | ArgRole::Index
            | ArgRole::Keyword
            | ArgRole::CommandPrefix
            | ArgRole::CommandName
            | ArgRole::CommandNameProbe
            | ArgRole::LambdaLiteral
            | ArgRole::NamespaceName
            | ArgRole::Boolean
            | ArgRole::NumericOrBoolean
            | ArgRole::Result => true,
        }
    }

    /// Witness that [`TCL_TYPES`] covers every [`TclType`].
    fn covered_tcl_type(t: TclType) -> bool {
        match t {
            TclType::String
            | TclType::Int
            | TclType::Double
            | TclType::Boolean
            | TclType::List
            | TclType::Dict
            | TclType::ByteArray
            | TclType::Numeric
            | TclType::Object
            | TclType::Channel => true,
        }
    }

    /// Witness that [`SIDE_EFFECT_TARGETS`] covers every [`SideEffectTarget`].
    fn covered_side_effect_target(t: SideEffectTarget) -> bool {
        match t {
            SideEffectTarget::Variable
            | SideEffectTarget::SessionTable
            | SideEffectTarget::PersistenceTable
            | SideEffectTarget::DataGroup
            | SideEffectTarget::HttpHeader
            | SideEffectTarget::HttpBody
            | SideEffectTarget::HttpStatus
            | SideEffectTarget::HttpUri
            | SideEffectTarget::HttpCookie
            | SideEffectTarget::HttpMethod
            | SideEffectTarget::Http2State
            | SideEffectTarget::ResponseCommit
            | SideEffectTarget::ConnectionControl
            | SideEffectTarget::TcpState
            | SideEffectTarget::SslState
            | SideEffectTarget::UdpState
            | SideEffectTarget::PoolSelection
            | SideEffectTarget::NodeSelection
            | SideEffectTarget::SnatSelection
            | SideEffectTarget::FileIo
            | SideEffectTarget::NetworkIo
            | SideEffectTarget::LogIo
            | SideEffectTarget::StreamProfile
            | SideEffectTarget::DnsState
            | SideEffectTarget::ClassificationState
            | SideEffectTarget::Dosl7State
            | SideEffectTarget::FlowState
            | SideEffectTarget::LsnState
            | SideEffectTarget::FtpState
            | SideEffectTarget::IcapState
            | SideEffectTarget::MessageState
            | SideEffectTarget::IStats
            | SideEffectTarget::ApmState
            | SideEffectTarget::AsmState
            | SideEffectTarget::BigipConfig
            | SideEffectTarget::ProcDefinition
            | SideEffectTarget::NamespaceState
            | SideEffectTarget::InterpState
            | SideEffectTarget::Process
            | SideEffectTarget::ChannelIo
            | SideEffectTarget::EventControl
            | SideEffectTarget::Unknown => true,
        }
    }

    /// Witnesses for the small enums, grouped so one test covers them all.
    fn covered_small_enums() -> bool {
        // `BodyKind` is `#[non_exhaustive]`, so the wildcard is required and
        // this witness cannot compile-gate a new variant. The catalogue test
        // below still checks the two spellings it does list.
        fn body(k: BodyKind) -> bool {
            matches!(k, BodyKind::Plain | BodyKind::Structural)
        }
        fn script_timing(timing: ScriptTiming) -> bool {
            match timing {
                ScriptTiming::SameInvocation
                | ScriptTiming::Deferred
                | ScriptTiming::ReferenceOnly => true,
            }
        }
        fn variable_scope(scope: VariableScope) -> bool {
            match scope {
                VariableScope::CurrentFrame | VariableScope::Global => true,
            }
        }
        fn storage(k: StorageType) -> bool {
            match k {
                StorageType::Dict | StorageType::List | StorageType::Array => true,
            }
        }
        fn byte_array(k: ByteArrayEffect) -> bool {
            match k {
                ByteArrayEffect::None
                | ByteArrayEffect::Transparent
                | ByteArrayEffect::Coerces
                | ByteArrayEffect::CaseFolds
                | ByteArrayEffect::Encodes
                | ByteArrayEffect::Rebinarifies { .. } => true,
            }
        }
        fn command_table(k: CommandTableEffect) -> bool {
            match k {
                CommandTableEffect::DefinesProcedure
                | CommandTableEffect::RenamesCommands
                | CommandTableEffect::CreatesAliases => true,
            }
        }
        fn pattern(k: PatternType) -> bool {
            match k {
                PatternType::Glob | PatternType::Regex => true,
            }
        }
        fn format(k: FormatType) -> bool {
            match k {
                FormatType::Sprintf
                | FormatType::Clock
                | FormatType::Binary
                | FormatType::Regsub => true,
            }
        }
        fn form(k: FormKind) -> bool {
            match k {
                FormKind::Default | FormKind::Getter | FormKind::Setter => true,
            }
        }
        fn symbol(k: DefinedSymbolKind) -> bool {
            match k {
                DefinedSymbolKind::Test
                | DefinedSymbolKind::Constraint
                | DefinedSymbolKind::Matcher
                | DefinedSymbolKind::Event => true,
            }
        }
        fn side(k: ConnectionSide) -> bool {
            match k {
                ConnectionSide::None
                | ConnectionSide::Client
                | ConnectionSide::Server
                | ConnectionSide::Both
                | ConnectionSide::Global => true,
            }
        }
        body(BodyKind::Plain)
            && script_timing(ScriptTiming::SameInvocation)
            && variable_scope(VariableScope::CurrentFrame)
            && storage(StorageType::List)
            && byte_array(ByteArrayEffect::None)
            && command_table(CommandTableEffect::DefinesProcedure)
            && pattern(PatternType::Glob)
            && format(FormatType::Sprintf)
            && form(FormKind::Default)
            && symbol(DefinedSymbolKind::Test)
            && side(ConnectionSide::None)
    }

    /// Witnesses that the hook catalogues cover every hook variant.
    ///
    /// One exhaustive `match` per hook family: adding a variant to the
    /// registry breaks the match, and the fix is to add it to the matching
    /// catalogue above as well.
    /// Every [`LoweringHookId`] variant is catalogued in `LOWERING_HOOKS`.
    fn covered_lowering(h: LoweringHookId) -> bool {
        match h {
            LoweringHookId::Expr
            | LoweringHookId::Return
            | LoweringHookId::Set
            | LoweringHookId::Incr
            | LoweringHookId::AppendOrLappend
            | LoweringHookId::Unset
            | LoweringHookId::Global
            | LoweringHookId::Variable
            | LoweringHookId::Upvar
            | LoweringHookId::Proc
            | LoweringHookId::When
            | LoweringHookId::NamespaceEval
            | LoweringHookId::If
            | LoweringHookId::Switch
            | LoweringHookId::For
            | LoweringHookId::While
            | LoweringHookId::Foreach
            | LoweringHookId::Lmap
            | LoweringHookId::ForeachLine
            | LoweringHookId::Catch
            | LoweringHookId::Try
            | LoweringHookId::Dict
            | LoweringHookId::Eval
            | LoweringHookId::Uplevel
            | LoweringHookId::Apply
            | LoweringHookId::ArrayFor => true,
        }
    }

    /// Every [`CodegenHookId`] variant is catalogued in `CODEGEN_HOOKS`.
    fn covered_codegen(h: CodegenHookId) -> bool {
        match h {
            CodegenHookId::Lassign
            | CodegenHookId::Llength
            | CodegenHookId::Lrange
            | CodegenHookId::Linsert
            | CodegenHookId::Lset
            | CodegenHookId::Dict
            | CodegenHookId::Array
            | CodegenHookId::Namespace
            | CodegenHookId::Append
            | CodegenHookId::Lappend
            | CodegenHookId::Unset
            | CodegenHookId::Tailcall
            | CodegenHookId::Concat
            | CodegenHookId::Global
            | CodegenHookId::Upvar => true,
        }
    }

    /// Every [`InlineCodegenHookId`] variant is catalogued in `INLINE_CODEGEN_HOOKS`.
    fn covered_inline(h: InlineCodegenHookId) -> bool {
        match h {
            InlineCodegenHookId::Expr
            | InlineCodegenHookId::Incr
            | InlineCodegenHookId::InfoExists
            | InlineCodegenHookId::String
            | InlineCodegenHookId::Lindex
            | InlineCodegenHookId::Lrange
            | InlineCodegenHookId::Lreplace
            | InlineCodegenHookId::Linsert
            | InlineCodegenHookId::Regexp
            | InlineCodegenHookId::List
            | InlineCodegenHookId::Array
            | InlineCodegenHookId::DictGet
            | InlineCodegenHookId::Catch
            | InlineCodegenHookId::Return
            | InlineCodegenHookId::Error
            | InlineCodegenHookId::Break
            | InlineCodegenHookId::Continue
            | InlineCodegenHookId::Try => true,
        }
    }

    /// Every [`AnalyserHookId`] variant is catalogued in `ANALYSER_HOOKS`.
    fn covered_analyser(h: AnalyserHookId) -> bool {
        match h {
            AnalyserHookId::Set
            | AnalyserHookId::Variable
            | AnalyserHookId::Global
            | AnalyserHookId::Proc
            | AnalyserHookId::OptProc
            | AnalyserHookId::Apply
            | AnalyserHookId::Uplevel
            | AnalyserHookId::NamespaceEval
            | AnalyserHookId::NamespaceEnsemble
            | AnalyserHookId::NamespaceImport
            | AnalyserHookId::NamespaceExport
            | AnalyserHookId::NamespaceForget
            | AnalyserHookId::NamespacePath
            | AnalyserHookId::NamespaceUnknown
            | AnalyserHookId::NamespaceUpvar
            | AnalyserHookId::Foreach
            | AnalyserHookId::For
            | AnalyserHookId::Switch
            | AnalyserHookId::Catch
            | AnalyserHookId::Try
            | AnalyserHookId::Upvar
            | AnalyserHookId::DictFor
            | AnalyserHookId::DictUpdate
            | AnalyserHookId::DictWith
            | AnalyserHookId::InterpAlias
            | AnalyserHookId::InterpEval
            | AnalyserHookId::InterpCreate
            | AnalyserHookId::InterpDelete
            | AnalyserHookId::InterpHide
            | AnalyserHookId::InterpExpose
            | AnalyserHookId::Rename
            | AnalyserHookId::OoDefine
            | AnalyserHookId::OoObjdefine
            | AnalyserHookId::PackageRequire
            | AnalyserHookId::PackageProvide
            | AnalyserHookId::PackageIfneeded
            | AnalyserHookId::PackagePrefer
            | AnalyserHookId::Source
            | AnalyserHookId::Append
            | AnalyserHookId::Lappend
            | AnalyserHookId::RegexPatternCapture
            | AnalyserHookId::Incr
            | AnalyserHookId::Load => true,
        }
    }

    #[test]
    fn enum_witnesses_compile_and_hold() {
        assert!(covered_arg_role(ArgRole::Value));
        assert!(covered_tcl_type(TclType::String));
        assert!(covered_side_effect_target(SideEffectTarget::Unknown));
        assert!(covered_small_enums());
        assert!(covered_lowering(LoweringHookId::Expr));
        assert!(covered_codegen(CodegenHookId::Dict));
        assert!(covered_inline(InlineCodegenHookId::Expr));
        assert!(covered_analyser(AnalyserHookId::Set));
    }

    #[test]
    fn catalogue_keys_are_unique() {
        for cat in [
            ARG_ROLES,
            TCL_TYPES,
            BODY_KINDS,
            SCRIPT_TIMINGS,
            VARIABLE_SCOPES,
            ARG_PRESENTATIONS,
            STORAGE_TYPES,
            BYTE_ARRAY_EFFECTS,
            COMMAND_TABLE_EFFECTS,
            PATTERN_TYPES,
            FORMAT_TYPES,
            FORM_KINDS,
            DEFINED_SYMBOL_KINDS,
            SIDE_EFFECT_TARGETS.as_slice(),
            CONNECTION_SIDES,
            LOWERING_HOOKS,
            CODEGEN_HOOKS,
            INLINE_CODEGEN_HOOKS,
            ANALYSER_HOOKS,
            &TRAITS,
            TAINT_COLOURS.as_slice(),
            DIALECTS.as_slice(),
        ] {
            let mut seen: Vec<&str> = Vec::new();
            for entry in cat {
                assert!(
                    !seen.contains(&entry.key),
                    "duplicate catalogue key {}",
                    entry.key
                );
                assert!(!entry.doc.is_empty(), "{} has no description", entry.key);
                seen.push(entry.key);
            }
        }
    }

    #[test]
    fn traits_catalogue_matches_the_registry_flags() {
        // The catalogue's keys must be exactly the registry's declared flags,
        // in the same order — a new trait shows up in the studio without a
        // hand-maintained table to update.
        let cat: Vec<&str> = TRAITS.iter().map(|v| v.key).collect();
        let declared: Vec<&str> = Trait::ALL.iter().map(|t| t.name()).collect();
        assert_eq!(cat, declared, "TRAITS must match the registry's flags");
        for t in Trait::ALL {
            let bit = Traits::of(&[*t]);
            assert_eq!(trait_bit(t.name()), Some(bit));
            assert_eq!(trait_keys(bit), vec![t.name()], "one flag, one name");
        }
    }

    /// Every flag owns a distinct bit.
    ///
    /// `SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` were both `1 << 61` under
    /// `bitflags`, which cannot reject a duplicate value — aliasing is a
    /// supported feature there. The registry now assigns bits from enum
    /// discriminants, so this is a property of the type rather than something
    /// a test can meaningfully defend; the assertion below documents the fixed
    /// bug for anyone who finds issue #1031.
    #[test]
    fn every_trait_has_its_own_bit() {
        assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);
        let mut seen = Traits::empty();
        for t in Trait::ALL {
            let bit = Traits::of(&[*t]);
            assert!(!seen.intersects(bit), "{} reuses a bit", t.name());
            seen |= bit;
        }
        assert_eq!(seen.len() as usize, Trait::ALL.len());
    }

    #[test]
    fn taint_catalogue_matches_bits() {
        let cat: Vec<&str> = TAINT_COLOURS.iter().map(|v| v.key).collect();
        let bits: Vec<&str> = TAINT_BITS.iter().map(|(n, _)| *n).collect();
        assert_eq!(cat, bits, "TAINT_COLOURS and TAINT_BITS must stay in step");
        for (name, bit) in TAINT_BITS {
            assert_eq!(taint_bit(name), Some(*bit));
        }
    }

    #[test]
    fn dialect_catalogue_round_trips_through_its_names() {
        for entry in DIALECTS.iter() {
            let rows = dialect_surface(entry.key).expect("catalogued dialect has a surface");
            assert_eq!(
                tcl_registry::model::surface::dialect_names_for_rows(rows),
                vec![entry.key.to_owned()],
                "{} must name exactly itself",
                entry.key
            );
        }
    }

    #[test]
    fn dialect_labels_come_from_the_profile_catalogue() {
        for entry in DIALECTS.iter() {
            let expected = crate::environment::catalogue_profile_for_dialect(entry.key)
                .map_or_else(
                    || {
                        BIT_ONLY_LABELS
                            .iter()
                            .find(|(bit, _)| *bit == entry.key)
                            .unwrap_or_else(|| {
                                panic!("{} has neither a profile nor a label", entry.key)
                            })
                            .1
                    },
                    |profile| profile.display_name,
                );
            assert_eq!(entry.doc, expected, "{} is mislabelled", entry.key);
        }
    }

    #[test]
    fn variant_name_matches_the_catalogue_spelling() {
        assert_eq!(variant_name(&ArgRole::VarWrite), "VarWrite");
        assert_eq!(variant_name(&TclType::ByteArray), "ByteArray");
        assert_eq!(
            variant_name(&LoweringHookId::AppendOrLappend),
            "AppendOrLappend"
        );
        assert_eq!(
            qualified_variant("TclType", &TclType::List),
            "TclType::List",
            "a RustExpr-edited field must carry its type path"
        );
        assert_eq!(
            optional_variant("TclType", &TclType::List),
            "Some(TclType::List)",
            "an Option-typed RustExpr field must carry its own Some"
        );
        for entry in ARG_ROLES {
            assert!(
                ARG_ROLES.iter().any(|e| e.key == entry.key),
                "catalogue is self-consistent"
            );
        }
    }
}
