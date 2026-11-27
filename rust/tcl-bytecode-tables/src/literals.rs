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

//! The literal table: native string and number literals, their allocations and the compiler's actions on the object array.

use std::collections::HashMap;

// Interning tables

/// Exact absent-string numeric header produced while a native C expression
/// compiler executes a constant operator subtree. This stores the payload;
/// it grants neither expression nor command admission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeExpressionNumberLiteral {
    /// Native integer payload.
    Integer(i64),
    /// IEEE double bits, preserving signed zero and NaN payload.
    Double(u64),
    /// Exact arbitrary-precision integer magnitude in decimal notation.
    BigInteger {
        /// Sign of the integer.
        negative: bool,
        /// Cleaned decimal magnitude without a sign.
        digits: String,
    },
}

impl NativeExpressionNumberLiteral {
    /// Retain a concrete native numeric payload without invoking its updater.
    /// Non-decimal Big storage needs the producing backend's normalisation.
    #[must_use]
    pub fn from_number(number: tcl_syntax::number::Number) -> Option<Self> {
        use tcl_syntax::number::Number;
        Some(match number {
            Number::Int(value) => Self::Integer(value),
            Number::Double(value) => Self::Double(value.to_bits()),
            Number::Nan { negative, payload } => Self::Double(
                (u64::from(negative) << 63)
                    | 0x7ff8_0000_0000_0000
                    | (payload.unwrap_or(0) & 0x0007_ffff_ffff_ffff),
            ),
            Number::Big {
                negative,
                radix: tcl_syntax::number::Radix::Dec,
                digits,
            } => Self::BigInteger { negative, digits },
            Number::Big { .. } => return None,
        })
    }

    /// Recover the full scalar payload without formatting or parsing it.
    #[must_use]
    pub fn number(&self) -> tcl_syntax::number::Number {
        match self {
            Self::Integer(value) => tcl_syntax::number::Number::Int(*value),
            Self::Double(bits) => tcl_syntax::number::Number::Double(f64::from_bits(*bits)),
            Self::BigInteger { negative, digits } => tcl_syntax::number::Number::Big {
                negative: *negative,
                radix: tcl_syntax::number::Radix::Dec,
                digits: digits.clone(),
            },
        }
    }
}

/// Native allocation route for one bytecode object-array entry.
/// These recipes describe construction, not proof that a public handler is live.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum NativeLiteralAllocation {
    /// `TclRegisterLiteral` without a command namespace partition.
    #[default]
    RegisteredData,
    /// `TclRegisterLiteral`'s original command-name namespace partition.
    RegisteredCommand {
        /// Actual compiler namespace path, validated against the retained token.
        namespace: tcl_runtime_api::ByteNamespacePath,
        /// Original fully-qualified command head selects the retained root token.
        fully_qualified: bool,
    },
    /// Command registration carrying its actual interpreter/namespace incarnation.
    RegisteredNativeCommand {
        /// Retained original compilation context, independently checked on admission.
        context: tcl_runtime_api::native_command_name::NativeLiteralContext,
        /// Original selected command name is absolute.
        fully_qualified: bool,
    },
    /// Original string allocated without interpreter-global registration.
    Unshared,
    /// C9.1 `TclAddLiteralObj` retains an original constant-concat String primary.
    PrivateConcatString,
    /// `TclAddLiteralObj` retains a supplied original header without `GetString`.
    /// The concrete producer must provide that header for this ordered slot.
    PrivateOriginal,
    /// `TclAddLiteralObj` retains a fresh native Integer header without `GetString`.
    PrivateInteger(i64),
    /// Selected C8.5+ constant-expression result, retained without `GetString`.
    PrivateExpressionNumber {
        /// Actual compiler release which executed the constant subtree.
        version: tcl_dialect::TclVersion,
        /// Complete original numeric payload.
        value: NativeExpressionNumberLiteral,
    },
    /// C8.5 logical constant evaluation retains its temporary compiler's
    /// registered Boolean original through `TclAddLiteralObj`. The temporary
    /// registration is released; a preexisting global original remains shared.
    PrivateLogicalBoolean85(bool),
    /// Fresh private Dictionary manufactured by the selected native Return compiler.
    PrivateReturnOptions(tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral),
    /// `TclAddLiteralObj` retains a fresh native List and its original fresh children.
    PrivateConstantList {
        /// Original decoded native element bytes, never interpreter-interned children.
        members: Vec<Vec<u8>>,
        /// Selected native compiler's object-string recipe.
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    },
}

/// One ordered bytecode object-array slot and its native allocation recipe.
/// String slots retain exact native bytes; private originals retain only the
/// allocation marker here and require the concrete producer's original header.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct NativeStringLiteral {
    bytes: Vec<u8>,
    allocation: NativeLiteralAllocation,
}

impl NativeStringLiteral {
    /// Retained native construction route for this original object-array slot.
    #[must_use]
    pub const fn allocation(&self) -> &NativeLiteralAllocation {
        &self.allocation
    }

    /// Exact native string payload, without decoding or replacement.
    ///
    /// # Panics
    /// A private original has no string receipt; use [`Self::byte_payload`] or
    /// consume its concrete original header instead.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.byte_payload()
            .expect("native original object has no string payload receipt")
    }

    /// Original string bytes when this slot's producer supplied them. A private
    /// original object has no byte receipt and must use its concrete header.
    #[must_use]
    pub fn byte_payload(&self) -> Option<&[u8]> {
        (!matches!(
            self.allocation,
            NativeLiteralAllocation::PrivateOriginal
                | NativeLiteralAllocation::PrivateReturnOptions(_)
                | NativeLiteralAllocation::PrivateInteger(_)
                | NativeLiteralAllocation::PrivateExpressionNumber { .. }
        ))
        .then_some(self.bytes.as_slice())
    }

    /// Project Unicode for a string slot when its complete payload is UTF-8.
    /// A private original must first be handled through its allocation recipe.
    pub fn unicode(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.bytes())
    }

    /// Whether the native string has no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes().is_empty()
    }
}

impl PartialEq<str> for NativeStringLiteral {
    fn eq(&self, other: &str) -> bool {
        self.byte_payload() == Some(other.as_bytes())
    }
}

impl PartialEq<&str> for NativeStringLiteral {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<String> for NativeStringLiteral {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

/// Intern pool mapping exact native string bytes to object-array indices.
#[derive(Debug, Clone, Default)]
pub struct LiteralTable {
    entries: Vec<NativeStringLiteral>,
    index: HashMap<Vec<u8>, usize>,
    actions: Vec<NativeLiteralAction>,
    discarded_native_passes: Vec<LiteralTable>,
    native_compiler_replay_environment:
        Option<tcl_runtime_api::native_compiler_pass::NativeCompilerPassEnvironment>,
}

/// Native compiler actions on its object array, in original execution order.
/// Local deduplication does not suppress later cache priming actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeLiteralAction {
    /// Construct/register a newly allocated object-array entry.
    Register(usize),
    /// Retain C9.1's original Syntax message as the compiled options'
    /// `-errorinfo` member after both object-array entries exist, then perform
    /// the reached compiler result reset through actual error-variable setters.
    RetainSyntaxErrorInfo {
        /// Original private merged Dictionary slot.
        options: usize,
        /// Original compiler message slot; bytes do not replace its identity.
        message: usize,
    },
    /// Move a reached constant result's numeric primary into a registered
    /// string header only when that destination still has no primary cache.
    AdoptExpressionNumber {
        /// Registered local object-array slot.
        index: usize,
        /// Actual C expression compiler that produced the constant.
        version: tcl_dialect::TclVersion,
        /// Numeric payload retained independently of its resident spelling.
        value: NativeExpressionNumberLiteral,
    },
    /// Apply the C8.4 expression compiler's Boolean getter to the SAME
    /// registered resident literal, retaining its original spelling.
    PrimeExpressionBoolean84(usize),
    /// Install the actual compiler-selected command-name cache on this entry.
    PrimeCommandName {
        /// Existing object-array index, including an earlier data registration.
        index: usize,
        /// Retained original native lookup and compilation context.
        receipt: Box<tcl_runtime_api::native_command_name::NativeCommandNamePriming>,
    },
    /// Duplicate a local entry and withdraw its local/global hash registration.
    Hide(usize),
}

impl LiteralTable {
    /// Retain the original first-pass array actions for chronological concrete
    /// construction and release before the final compiler pass array.
    /// This metadata owns no native headers and grants no compiler admission.
    pub fn retain_discarded_native_pass(&mut self, table: Self) {
        self.discarded_native_passes.push(table);
    }

    /// Earlier compiler arrays which must be constructed and released in order.
    #[must_use]
    pub fn discarded_native_passes(&self) -> &[Self] {
        &self.discarded_native_passes
    }

    /// Retain the original environment used to plan a compact second pass.
    /// Concrete realization must revalidate it after the first array's callbacks.
    /// This metadata grants neither native compiler admission nor cache validity.
    pub fn retain_compiler_replay_environment(
        &mut self,
        environment: tcl_runtime_api::native_compiler_pass::NativeCompilerPassEnvironment,
    ) {
        self.native_compiler_replay_environment = Some(environment);
    }

    /// Original planned replay environment, independent of installed body freshness.
    #[must_use]
    pub fn compiler_replay_environment(
        &self,
    ) -> Option<&tcl_runtime_api::native_compiler_pass::NativeCompilerPassEnvironment> {
        self.native_compiler_replay_environment.as_ref()
    }

    /// Register the original C8.4 Boolean word and retain its reached getter
    /// action separately from local/global literal deduplication.
    pub fn intern_expression_boolean84(&mut self, bytes: &[u8]) -> usize {
        let index = self.intern_bytes(bytes);
        self.actions
            .push(NativeLiteralAction::PrimeExpressionBoolean84(index));
        index
    }
    /// Register a resident constant and retain the compiler's conditional
    /// internal-representation transfer after normal literal deduplication.
    pub fn intern_expression_number(
        &mut self,
        bytes: &[u8],
        version: tcl_dialect::TclVersion,
        value: NativeExpressionNumberLiteral,
    ) -> usize {
        let index = self.intern_bytes(bytes);
        self.actions
            .push(NativeLiteralAction::AdoptExpressionNumber {
                index,
                version,
                value,
            });
        index
    }
    /// Create a new empty literal table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a deduplicated index for a Unicode value.
    pub fn intern(&mut self, value: &str) -> usize {
        self.intern_bytes(value.as_bytes())
    }

    /// Get or create a deduplicated index for exact native bytes.
    pub fn intern_bytes(&mut self, value: &[u8]) -> usize {
        self.intern_allocated(value, NativeLiteralAllocation::RegisteredData)
    }

    /// Intern a command head under its selected native namespace partition.
    /// Local-array byte deduplication retains the first registration route.
    pub fn intern_command_bytes(
        &mut self,
        value: &[u8],
        namespace: &tcl_runtime_api::ByteNamespacePath,
        fully_qualified: bool,
    ) -> usize {
        self.intern_allocated(
            value,
            NativeLiteralAllocation::RegisteredCommand {
                namespace: namespace.clone(),
                fully_qualified,
            },
        )
    }

    /// Register an original head with its actual namespace-token receipt.
    /// A previously allocated data literal retains its original allocation.
    pub fn intern_native_command_bytes(
        &mut self,
        value: &[u8],
        context: &tcl_runtime_api::native_command_name::NativeLiteralContext,
        fully_qualified: bool,
    ) -> usize {
        self.intern_allocated(
            value,
            NativeLiteralAllocation::RegisteredNativeCommand {
                context: context.clone(),
                fully_qualified,
            },
        )
    }

    /// Retain a reached native priming action even after local byte deduplication.
    /// Returns false when the receipt does not describe this original head.
    pub fn prime_native_command_name(
        &mut self,
        index: usize,
        receipt: tcl_runtime_api::native_command_name::NativeCommandNamePriming,
    ) -> bool {
        if self
            .entries
            .get(index)
            .is_none_or(|entry| entry.bytes() != receipt.original.as_bytes())
        {
            return false;
        }
        self.actions.push(NativeLiteralAction::PrimeCommandName {
            index,
            receipt: Box::new(receipt),
        });
        true
    }

    /// Retain native `TclHideLiteral` ordering and remove its local deduplication key.
    /// The runtime duplicates the complete current object before releasing its lease.
    pub fn hide_native_literal(&mut self, index: usize) -> bool {
        let Some(entry) = self.entries.get(index) else {
            return false;
        };
        if self.index.get(entry.bytes()) == Some(&index) {
            self.index.remove(entry.bytes());
        }
        self.actions.push(NativeLiteralAction::Hide(index));
        true
    }

    fn intern_allocated(&mut self, value: &[u8], allocation: NativeLiteralAllocation) -> usize {
        if let Some(&idx) = self.index.get(value) {
            return idx;
        }
        let idx = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: value.to_owned(),
            allocation,
        });
        self.index.insert(value.to_owned(), idx);
        self.actions.push(NativeLiteralAction::Register(idx));
        idx
    }

    /// Always append a Unicode value (no deduplication).
    pub fn register(&mut self, value: &str) -> usize {
        self.register_bytes(value.as_bytes())
    }

    /// Always append exact native bytes (no deduplication).
    pub fn register_bytes(&mut self, value: &[u8]) -> usize {
        self.register_unshared(value)
    }

    /// Append an original string without local or interpreter-global deduplication.
    pub fn register_unshared(&mut self, value: &[u8]) -> usize {
        let idx = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: value.to_owned(),
            allocation: NativeLiteralAllocation::Unshared,
        });
        self.actions.push(NativeLiteralAction::Register(idx));
        idx
    }

    /// Retain the C9.1 constant-concat String original without global registration.
    pub fn register_private_concat_string(&mut self, value: &[u8]) -> usize {
        let index = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: value.to_owned(),
            allocation: NativeLiteralAllocation::PrivateConcatString,
        });
        self.actions.push(NativeLiteralAction::Register(index));
        index
    }

    /// Reserve one ordered `TclAddLiteralObj` slot without materialising its
    /// original object. The concrete emitter retains the supplied header; this
    /// table neither allocates it nor grants a string or compilation receipt.
    pub fn register_private_original(&mut self) -> usize {
        let idx = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: Vec::new(),
            allocation: NativeLiteralAllocation::PrivateOriginal,
        });
        self.actions.push(NativeLiteralAction::Register(idx));
        idx
    }

    /// Reserve a chronological private native Return-options object slot.
    pub fn register_private_return_options(
        &mut self,
        recipe: tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral,
    ) -> usize {
        let index = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: Vec::new(),
            allocation: NativeLiteralAllocation::PrivateReturnOptions(recipe),
        });
        self.actions.push(NativeLiteralAction::Register(index));
        index
    }

    /// Record the selected C9.1 Syntax options' original message member.
    /// The backend validates the actual release and both original slots.
    pub fn retain_syntax_error_info(&mut self, options: usize, message: usize) {
        self.actions
            .push(NativeLiteralAction::RetainSyntaxErrorInfo { options, message });
    }

    /// Append the fresh native Integer supplied to `TclAddLiteralObj`.
    /// No interpreter-global String registration or original byte receipt is issued.
    pub fn register_private_integer(&mut self, value: i64) -> usize {
        let index = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: Vec::new(),
            allocation: NativeLiteralAllocation::PrivateInteger(value),
        });
        self.actions.push(NativeLiteralAction::Register(index));
        index
    }

    /// Retain a selected compiler's absent-string constant-expression result.
    /// Consumers validate the compiler release before constructing its header.
    pub fn register_private_expression_number(
        &mut self,
        version: tcl_dialect::TclVersion,
        value: NativeExpressionNumberLiteral,
    ) -> usize {
        let index = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: Vec::new(),
            allocation: NativeLiteralAllocation::PrivateExpressionNumber { version, value },
        });
        self.actions.push(NativeLiteralAction::Register(index));
        index
    }

    /// Append the C8.5 constant logical evaluator's original Boolean result.
    /// Unlike ordinary registration, this always creates a new local slot.
    pub fn register_private_logical_boolean85(&mut self, value: bool) -> usize {
        let index = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: if value { b"1" } else { b"0" }.to_vec(),
            allocation: NativeLiteralAllocation::PrivateLogicalBoolean85(value),
        });
        self.actions.push(NativeLiteralAction::Register(index));
        index
    }

    /// Append a native constant List object and retain its original fresh elements.
    /// The emitter separately proves the actual compiler selects this allocation.
    pub fn register_private_constant_list(
        &mut self,
        members: &[impl AsRef<[u8]>],
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> usize {
        use tcl_syntax::list_result::NativeListResultSerialization as Render;
        let renderer = match protocol {
            tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) => {
                Render::Tcl84
            }
            tcl_syntax::native_string::NativeStringProtocol::C(_) => Render::Tcl85Plus,
            tcl_syntax::native_string::NativeStringProtocol::Jim084 => Render::Jim084,
        };
        let idx = self.entries.len();
        self.entries.push(NativeStringLiteral {
            bytes: renderer.render(members),
            allocation: NativeLiteralAllocation::PrivateConstantList {
                members: members
                    .iter()
                    .map(|member| member.as_ref().to_vec())
                    .collect(),
                protocol,
            },
        });
        self.actions.push(NativeLiteralAction::Register(idx));
        idx
    }

    /// Return all interned native byte entries in order.
    #[must_use]
    pub fn entries(&self) -> &[NativeStringLiteral] {
        &self.entries
    }

    /// Ordered allocation/cache actions retained by the original compiler.
    #[must_use]
    pub fn native_actions(&self) -> &[NativeLiteralAction] {
        &self.actions
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
