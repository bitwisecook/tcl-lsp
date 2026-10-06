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

//! Native dictionary argument preparation, separate from variable publication.

use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

/// Argument preprocessing selected by the actual dictionary command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDictionaryAppendInputs {
    /// Append original operands in sequence to one prepared dictionary member.
    PerOperand,
    /// Concatenate original arguments before the one member append operation.
    Concatenate(crate::native_object_append::NativeObjectCatProtocol),
}

/// Construction of a dictionary member absent at the native lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDictionaryMissingAppendMember {
    /// Tcl 8 appends into a fresh empty object rather than adopting its source.
    FreshEmptyReceiver,
    /// Tcl 9 adopts the original source or the prepared concatenation result.
    AdoptPreparedInput,
}

/// Publication of an existing member when the caller supplies no append input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDictionaryEmptyAppend {
    /// Select the member's native copy and put it into the prepared root.
    CopyAndPut,
    /// Keep the member and root representation; still publish the variable.
    StoreOnly,
}

/// Actual C dictionary append behavior, independent of ordinary variable append.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeDictionaryAppendPolicy {
    /// Preparation of supplied original argument objects.
    pub inputs: NativeDictionaryAppendInputs,
    /// Initial member object selected after a missing key.
    pub missing: NativeDictionaryMissingAppendMember,
    /// Existing-member operation for an empty argument batch.
    pub empty: NativeDictionaryEmptyAppend,
}

/// Initial generic dictionary variable lookup, before its read callbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDictionaryVariableLookup {
    /// Existing roots only; create a missing element shell in an existing array.
    ExistingRootWithElementCreation,
}

impl crate::InvocationDialect {
    /// Select the actual Tcl 9 array-default command independently of the
    /// source grammar or a vendor's compatibility version.
    #[must_use]
    pub fn native_array_default_protocol(
        self,
    ) -> Option<tcl_runtime_api::NativeArrayDefaultProtocol> {
        match self.native_string_protocol()? {
            NativeStringProtocol::C(TclVersion::V9_0 | TclVersion::V9_1) => {
                Some(tcl_runtime_api::NativeArrayDefaultProtocol::Tcl9)
            }
            NativeStringProtocol::C(TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6)
            | NativeStringProtocol::Jim084 => None,
        }
    }

    /// Select the native primitive GET frontier. Scripted Jim wrappers own
    /// their normal Tcl reads rather than receiving this C primitive receipt.
    #[must_use]
    pub fn native_dictionary_variable_lookup(self) -> Option<NativeDictionaryVariableLookup> {
        match self.native_string_protocol()? {
            NativeStringProtocol::C(
                TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1,
            ) => Some(NativeDictionaryVariableLookup::ExistingRootWithElementCreation),
            NativeStringProtocol::C(TclVersion::V8_4) | NativeStringProtocol::Jim084 => None,
        }
    }

    /// Select C primitive member ownership. Jim dictionary append is an actual
    /// scripted procedure and grants no borrowed-member primitive capability.
    #[must_use]
    pub fn native_dictionary_append_policy(self) -> Option<NativeDictionaryAppendPolicy> {
        let inputs = self.native_dictionary_append_inputs()?;
        let (missing, empty) = match self.native_string_protocol()? {
            NativeStringProtocol::C(TclVersion::V8_5 | TclVersion::V8_6) => (
                NativeDictionaryMissingAppendMember::FreshEmptyReceiver,
                NativeDictionaryEmptyAppend::CopyAndPut,
            ),
            NativeStringProtocol::C(TclVersion::V9_0 | TclVersion::V9_1) => (
                NativeDictionaryMissingAppendMember::AdoptPreparedInput,
                NativeDictionaryEmptyAppend::StoreOnly,
            ),
            NativeStringProtocol::C(TclVersion::V8_4) | NativeStringProtocol::Jim084 => {
                return None;
            }
        };
        Some(NativeDictionaryAppendPolicy {
            inputs,
            missing,
            empty,
        })
    }
    /// Select the actual dictionary command's preprocessing owner. Vendor
    /// simulation and C8.4 command absence provide no dictionary capability.
    #[must_use]
    pub fn native_dictionary_append_inputs(self) -> Option<NativeDictionaryAppendInputs> {
        match self.native_string_protocol()? {
            NativeStringProtocol::C(TclVersion::V8_4) => None,
            NativeStringProtocol::C(TclVersion::V8_5 | TclVersion::V8_6)
            | NativeStringProtocol::Jim084 => Some(NativeDictionaryAppendInputs::PerOperand),
            NativeStringProtocol::C(TclVersion::V9_0 | TclVersion::V9_1) => Some(
                NativeDictionaryAppendInputs::Concatenate(self.native_object_cat_protocol()?),
            ),
        }
    }
}

use crate::hooks::LoweringHookId;
use crate::native_compilation::{
    NativeBodyCompilation, NativeCompilationContext, NativeCompilationFrame,
    NativeCompilationGrammar, NativeCompilationGuard, NativeCompilationSelection,
    NativeCompilationSpec, NativeCompilationWordShape, NativeCompilerImplementationLookup,
    NativeNamedInvocationProtocol,
};
use crate::{IntrinsicId, InvocationWords, SemanticOperationId};

/// Original C dictionary compiler registration. This is compiler metadata;
/// it does not attest a current interpreter slot or an evaluated handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NativeDictionaryCommand {
    /// Append string values to a dictionary member.
    Append,
    /// Construct a dictionary from key/value pairs.
    Create,
    /// Test a dictionary key path.
    Exists,
    /// Iterate dictionary key/value pairs.
    For,
    /// Fetch a dictionary key path.
    Get,
    /// Increment a dictionary member.
    Incr,
    /// Enumerate dictionary keys.
    Keys,
    /// Append one list value to a dictionary member.
    Lappend,
    /// Collect dictionary iteration results.
    Map,
    /// Merge dictionary values.
    Merge,
    /// Remove dictionary keys.
    Remove,
    /// Replace dictionary members.
    Replace,
    /// Store a dictionary key path.
    Set,
    /// Read the dictionary size.
    Size,
    /// Remove a key path in a dictionary variable.
    Unset,
    /// Bind dictionary members while executing a body.
    Update,
    /// Enumerate dictionary values.
    Values,
    /// Bind dictionary contents while executing a body.
    With,
    /// Report dictionary implementation information.
    Info,
    /// Fetch a key path with a default, using the short spelling.
    GetDefault,
    /// Fetch a key path with a default, using the long spelling.
    GetWithDefault,
}

macro_rules! dictionary_lookup {
    ($member:literal) => {
        NativeCompilerImplementationLookup {
            ensemble: "::dict",
            member: $member,
            slot: concat!("::tcl::dict::", $member),
            command: "dict",
            prepended: &[$member],
        }
    };
}

const IMPLEMENTATIONS: [NativeCompilerImplementationLookup; 21] = [
    dictionary_lookup!("append"),
    dictionary_lookup!("create"),
    dictionary_lookup!("exists"),
    dictionary_lookup!("for"),
    dictionary_lookup!("get"),
    dictionary_lookup!("incr"),
    dictionary_lookup!("keys"),
    dictionary_lookup!("lappend"),
    dictionary_lookup!("map"),
    dictionary_lookup!("merge"),
    dictionary_lookup!("remove"),
    dictionary_lookup!("replace"),
    dictionary_lookup!("set"),
    dictionary_lookup!("size"),
    dictionary_lookup!("unset"),
    dictionary_lookup!("update"),
    dictionary_lookup!("values"),
    dictionary_lookup!("with"),
    dictionary_lookup!("info"),
    dictionary_lookup!("getdef"),
    dictionary_lookup!("getwithdefault"),
];

impl NativeDictionaryCommand {
    /// Retain original dictionary compiler visits, including a declined prefix.
    /// Missing geometry is distinct from a compiler decline.
    pub fn original_preparations(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<
        Result<
            Vec<crate::native_control_compilation::NativeControlPreparationStep>,
            crate::native_dictionary_compilation::NativeDictionaryCompilationUnavailable,
        >,
    > {
        use crate::native_dictionary_compilation::NativeDictionaryCompilationUnavailable as Error;
        if matches!(self, Self::Update | Self::With) {
            return Some(
                crate::native_dictionary_scope_compilation::compile_native_dictionary_scope(
                    self,
                    words,
                    operand_from,
                    version,
                    context,
                )
                .map(|recipe| recipe.preparations)
                .map_err(|_| Error::Unavailable),
            );
        }
        if matches!(
            self,
            Self::Set | Self::Unset | Self::Append | Self::Lappend | Self::Incr
        ) {
            return Some(
                crate::native_dictionary_compilation::compile_native_dictionary_mutation(
                    self,
                    words,
                    operand_from,
                    version,
                    context,
                )
                .map(|recipe| recipe.preparations),
            );
        }
        None
    }

    /// Select actual original dictionary scope and mutation geometry.
    #[must_use]
    pub fn select_original_compilation(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        ensemble: bool,
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> Option<NativeCompilationSelection> {
        use crate::native_control_compilation::NativeControlOutcome as Outcome;
        let from = operand_from.checked_add(usize::from(ensemble))?;
        let selected = if matches!(self, Self::Update | Self::With) {
            match crate::native_dictionary_scope_compilation::compile_native_dictionary_scope(
                self, words, from, version, context,
            ) {
                Ok(recipe) => Some(matches!(recipe.outcome, Outcome::Inline(_))),
                Err(_) => return Some(NativeCompilationSelection::Unknown),
            }
        } else if matches!(
            self,
            Self::Set | Self::Unset | Self::Append | Self::Lappend | Self::Incr
        ) {
            match crate::native_dictionary_compilation::compile_native_dictionary_mutation(
                self, words, from, version, context,
            ) {
                Ok(recipe) => Some(matches!(recipe.outcome, Outcome::Inline(_))),
                Err(_) => return Some(NativeCompilationSelection::Unknown),
            }
        } else {
            None
        };
        let Some(inline) = selected else {
            return self.select_original_lookup(words, operand_from, ensemble, version);
        };
        if inline {
            return Some(self.inline());
        }
        if version < self.hook_from() {
            return Some(NativeCompilationSelection::Generic);
        }
        let Ok(projected) =
            crate::native_compiler_word_projection::project_native_compiler_words(words, version)
        else {
            return Some(NativeCompilationSelection::Unknown);
        };
        let operands = projected.get(from..)?;
        if operands
            .iter()
            .any(|word| word.shape == NativeCompilationWordShape::Expanded)
            || !self.valid_operand_count(operands.len())
        {
            return Some(NativeCompilationSelection::Generic);
        }
        Some(if self == Self::With {
            self.named(from)
        } else {
            self.local_fallback(from, version)
        })
    }

    /// Select value-independent dictionary compilers from original native words.
    /// Opaque literal values remain original operands; no Unicode or getter is
    /// required merely to establish native lookup arity and stack geometry.
    #[must_use]
    pub fn select_original_lookup(
        self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        operand_from: usize,
        ensemble: bool,
        version: TclVersion,
    ) -> Option<NativeCompilationSelection> {
        use crate::native_dictionary_compilation::{
            NativeDictionaryCompilationUnavailable, compile_native_dictionary_lookup,
        };
        let from = operand_from.checked_add(usize::from(ensemble))?;
        if matches!(
            self,
            Self::Get | Self::Exists | Self::GetDefault | Self::GetWithDefault
        ) {
            return Some(
                match compile_native_dictionary_lookup(self, words, from, version) {
                    Ok(_) => self.inline(),
                    Err(NativeDictionaryCompilationUnavailable::Generic) => {
                        NativeCompilationSelection::Generic
                    }
                    Err(NativeDictionaryCompilationUnavailable::Unavailable) => {
                        NativeCompilationSelection::Unknown
                    }
                },
            );
        }
        if !matches!(self, Self::Keys | Self::Values | Self::Size | Self::Info) {
            return None;
        }
        if version < self.hook_from() {
            return Some(NativeCompilationSelection::Generic);
        }
        let Ok(projected) =
            crate::native_compiler_word_projection::project_native_compiler_words(words, version)
        else {
            return Some(NativeCompilationSelection::Unknown);
        };
        let Some(operands) = projected.get(from..) else {
            return Some(NativeCompilationSelection::Unknown);
        };
        if operands
            .iter()
            .any(|word| word.shape == NativeCompilationWordShape::Expanded)
        {
            return Some(NativeCompilationSelection::Generic);
        }
        let accepted = match self {
            Self::Keys | Self::Values => (1..=2).contains(&operands.len()),
            _ => operands.len() == 1,
        };
        Some(if accepted {
            self.named(usize::from(ensemble))
        } else {
            NativeCompilationSelection::Generic
        })
    }
    /// Exact original public mapping and private compiler implementation slot.
    #[must_use]
    pub const fn lookup(self) -> NativeCompilerImplementationLookup {
        IMPLEMENTATIONS[self as usize]
    }

    /// First C release with this compiler, independently of handler availability.
    #[must_use]
    pub const fn hook_from(self) -> TclVersion {
        match self {
            Self::Append
            | Self::For
            | Self::Get
            | Self::Incr
            | Self::Lappend
            | Self::Set
            | Self::Update => TclVersion::V8_5,
            Self::Create | Self::Exists | Self::Map | Self::Merge | Self::Unset | Self::With => {
                TclVersion::V8_6
            }
            Self::GetDefault | Self::GetWithDefault => TclVersion::V9_0,
            Self::Keys | Self::Remove | Self::Replace | Self::Size | Self::Values | Self::Info => {
                TclVersion::V9_1
            }
        }
    }

    /// Compiler specification for an original ensemble or private worker call.
    #[must_use]
    pub const fn spec(self, ensemble: bool) -> NativeCompilationSpec {
        let operation = match self {
            Self::Get => SemanticOperationId::Intrinsic(IntrinsicId::DictGet),
            Self::Set => SemanticOperationId::Intrinsic(IntrinsicId::DictSet),
            Self::Unset => SemanticOperationId::Intrinsic(IntrinsicId::DictUnset),
            Self::Incr => SemanticOperationId::Intrinsic(IntrinsicId::DictIncr),
            Self::Append => SemanticOperationId::Intrinsic(IntrinsicId::DictAppend),
            Self::Lappend => SemanticOperationId::Intrinsic(IntrinsicId::DictListAppend),
            Self::For | Self::Update | Self::With => {
                SemanticOperationId::StructuredLowering(LoweringHookId::Dict)
            }
            _ => SemanticOperationId::Invoke,
        };
        NativeCompilationSpec {
            grammar: NativeCompilationGrammar::Dictionary {
                command: self,
                ensemble,
            },
            operation,
            body: NativeBodyCompilation::Inherit,
        }
    }

    /// Select using original post-head words, their retained source shapes, and
    /// the actual C compiler environment. Missing source/value evidence abstains.
    #[must_use]
    pub fn select(
        self,
        ensemble: bool,
        words: InvocationWords<'_>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        use NativeCompilationSelection as Selection;
        if version < self.hook_from() {
            return Selection::Generic;
        }
        let from = usize::from(ensemble);
        if shapes.len() != words.arguments().len()
            || shapes.len() < from
            || shapes.contains(&NativeCompilationWordShape::Opaque)
        {
            return Selection::Unknown;
        }
        if shapes.contains(&NativeCompilationWordShape::Expanded) {
            // TclParseCommand can flatten a literal expanded list before a
            // dictionary compiler sees it. Unprojected source expansion does
            // not prove that the original compiler declined.
            return Selection::Unknown;
        }
        if ensemble && !simple(shapes[0]) {
            return Selection::Generic;
        }
        let operands = &shapes[from..];
        let args = words.arguments().slice_from(from);
        let count = operands.len();
        if !self.valid_operand_count(count) {
            return Selection::Generic;
        }
        if matches!(self, Self::Info | Self::Keys | Self::Size | Self::Values) {
            return self.named(from);
        }
        // The pinned C9.1 replace compiler checks `% 1`, not pair parity.
        // An unmatched pair has no safe authored opcode recipe here.
        if self == Self::Replace && count.is_multiple_of(2) {
            return Selection::Unknown;
        }
        if matches!(
            self,
            Self::Append | Self::Lappend | Self::Set | Self::Unset | Self::Incr | Self::Update
        ) {
            if context.frame == NativeCompilationFrame::Unknown {
                return Selection::Unknown;
            }
            if simple(operands[0]) && args.literal_at(0).is_none() {
                return Selection::Unknown;
            }
            let local = context.frame == NativeCompilationFrame::ProcedureCode
                && simple(operands[0])
                && args.literal_at(0).is_some_and(local_scalar);
            if !local {
                return self.local_fallback(from, version);
            }
        }
        if self == Self::Incr
            && count == 3
            && let Some(selection) = self.select_increment_amount(from, args, operands[2], version)
        {
            return selection;
        }
        if matches!(self, Self::For | Self::Map) {
            return self.select_iteration(from, args.literal_at(0), operands, version, context);
        }
        if self == Self::Update
            && (!simple(*operands.last().expect("validated update arity"))
                || (2..count - 1).step_by(2).any(|index| {
                    !simple(operands[index]) || !args.literal_at(index).is_some_and(local_scalar)
                }))
        {
            return self.local_fallback(from, version);
        }
        if self == Self::Merge && count >= 2 {
            match context.frame {
                NativeCompilationFrame::Unknown => return Selection::Unknown,
                NativeCompilationFrame::ScriptCode => return self.named(from),
                NativeCompilationFrame::ProcedureCode => {}
            }
        }
        if self == Self::With {
            if !simple(operands[count - 1]) {
                return self.named(from);
            }
            if context.frame != NativeCompilationFrame::ProcedureCode {
                // TclIsEmptyToken also accepts comments and whitespace. A
                // nonempty value needs that lexer-owned test, not trim().
                match args.literal_at(count - 1) {
                    Some("") => {}
                    _ => return Selection::Unknown,
                }
            }
        }
        self.inline()
    }

    fn select_increment_amount(
        self,
        from: usize,
        args: crate::InvocationArguments<'_>,
        shape: NativeCompilationWordShape,
        version: TclVersion,
    ) -> Option<NativeCompilationSelection> {
        if shape == NativeCompilationWordShape::BackslashLiteral {
            return Some(NativeCompilationSelection::Unknown);
        }
        let Some(amount) = args.literal_at(2) else {
            return Some(self.local_fallback(from, version));
        };
        // Tcl_GetIntFromObj uses native numeric syntax; do not borrow Rust
        // integer parsing for a compiler constant whose conversion is unproved.
        if !simple(shape)
            || !amount.bytes().all(|byte| byte.is_ascii_digit())
            || (amount.len() > 1 && amount.starts_with('0'))
        {
            return Some(NativeCompilationSelection::Unknown);
        }
        if amount.parse::<i32>().is_err() {
            return Some(self.local_fallback(from, version));
        }
        None
    }

    fn valid_operand_count(self, count: usize) -> bool {
        if u32::try_from(count).is_err() {
            return false;
        }
        match self {
            Self::Get | Self::Exists | Self::Unset | Self::Remove | Self::With => count >= 2,
            Self::GetDefault | Self::GetWithDefault | Self::Set | Self::Replace => count >= 3,
            Self::Append => (3..=99).contains(&count),
            Self::Lappend | Self::For | Self::Map => count == 3,
            Self::Incr => (2..=3).contains(&count),
            Self::Create => count.is_multiple_of(2),
            Self::Update => count >= 4 && count.is_multiple_of(2),
            Self::Info | Self::Size => count == 1,
            Self::Keys | Self::Values => (1..=2).contains(&count),
            Self::Merge => true,
        }
    }

    fn inline(self) -> NativeCompilationSelection {
        NativeCompilationSelection::Inline {
            operation: self.spec(false).operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    }

    fn named(self, from: usize) -> NativeCompilationSelection {
        NativeCompilationSelection::NamedInvocation {
            lookup: &IMPLEMENTATIONS[self as usize],
            arguments_from: from,
            protocol: NativeNamedInvocationProtocol::Direct,
        }
    }

    fn local_fallback(self, from: usize, version: TclVersion) -> NativeCompilationSelection {
        if self == Self::Set || version == TclVersion::V8_5 {
            NativeCompilationSelection::Generic
        } else {
            self.named(from)
        }
    }

    fn select_iteration(
        self,
        from: usize,
        variables: Option<&str>,
        shapes: &[NativeCompilationWordShape],
        version: TclVersion,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        if context.frame == NativeCompilationFrame::Unknown {
            return NativeCompilationSelection::Unknown;
        }
        if context.frame != NativeCompilationFrame::ProcedureCode
            || !simple(shapes[2])
            || (!simple(shapes[0]) && version >= TclVersion::V8_6)
        {
            return self.local_fallback(from, version);
        }
        let Some(variables) = variables else {
            return self.local_fallback(from, version);
        };
        if variables.as_bytes().contains(&0) {
            return NativeCompilationSelection::Unknown;
        }
        let Ok(names) = tcl_syntax::list::split_list(variables) else {
            return self.local_fallback(from, version);
        };
        if names.len() != 2 || !names.iter().all(|name| local_scalar(name)) {
            return self.local_fallback(from, version);
        }
        self.inline()
    }
}

fn simple(shape: NativeCompilationWordShape) -> bool {
    matches!(
        shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}

fn local_scalar(name: &str) -> bool {
    !tcl_syntax::naming::is_qualified(name.as_bytes())
        && tcl_syntax::naming::split_element_ref(name).is_none()
}

#[cfg(test)]
mod compiler_tests {
    use super::*;

    fn procedure() -> NativeCompilationContext {
        NativeCompilationContext {
            frame: NativeCompilationFrame::ProcedureCode,
            ..NativeCompilationContext::default()
        }
    }

    #[test]
    fn compiler_versions_are_independent_of_dictionary_handler_presence() {
        for command in [
            NativeDictionaryCommand::Create,
            NativeDictionaryCommand::With,
        ] {
            assert_eq!(command.hook_from(), TclVersion::V8_6);
            assert_eq!(
                command.select(
                    true,
                    InvocationWords::literals("dict", &[command.lookup().member]),
                    &[NativeCompilationWordShape::Literal],
                    TclVersion::V8_5,
                    procedure()
                ),
                NativeCompilationSelection::Generic
            );
        }
        assert_eq!(
            NativeDictionaryCommand::GetDefault.hook_from(),
            TclVersion::V9_0
        );
        assert_eq!(NativeDictionaryCommand::Keys.hook_from(), TclVersion::V9_1);
    }

    #[test]
    fn local_dictionary_fallback_retains_the_original_private_worker() {
        let command = NativeDictionaryCommand::Incr;
        let values = ["incr", "::d", "key"];
        let words = InvocationWords::literals("dict", &values);
        let shapes = [NativeCompilationWordShape::Literal; 3];
        assert_eq!(
            command.select(true, words, &shapes, TclVersion::V8_5, procedure()),
            NativeCompilationSelection::Generic
        );
        let NativeCompilationSelection::NamedInvocation {
            lookup,
            arguments_from,
            protocol,
        } = command.select(true, words, &shapes, TclVersion::V8_6, procedure())
        else {
            panic!("native named worker fallback");
        };
        assert_eq!(lookup.slot, "::tcl::dict::incr");
        assert_eq!(arguments_from, 1);
        assert_eq!(protocol, NativeNamedInvocationProtocol::Direct);
        let inline = command.select(
            false,
            InvocationWords::literals("::tcl::dict::incr", &["d", "key"]),
            &[NativeCompilationWordShape::Literal; 2],
            TclVersion::V8_6,
            procedure(),
        );
        assert!(matches!(
            inline,
            NativeCompilationSelection::Inline {
                operation: SemanticOperationId::Intrinsic(IntrinsicId::DictIncr),
                ..
            }
        ));
    }

    #[test]
    fn unprojected_expansion_and_unproved_numeric_syntax_remain_unknown() {
        assert_eq!(
            NativeDictionaryCommand::Create.select(
                true,
                InvocationWords::structured(
                    crate::InvocationWord::Literal("dict"),
                    &[
                        crate::InvocationWord::Literal("create"),
                        crate::InvocationWord::Expanded
                    ]
                ),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Expanded
                ],
                TclVersion::V9_1,
                procedure()
            ),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            NativeDictionaryCommand::Incr.select(
                false,
                InvocationWords::literals("::tcl::dict::incr", &["d", "key", "010"]),
                &[NativeCompilationWordShape::Literal; 3],
                TclVersion::V8_6,
                procedure()
            ),
            NativeCompilationSelection::Unknown
        );
    }
}
