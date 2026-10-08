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

//! tcllib 2.0's procedures on the registry's own spec modules
//! (`docs/design/compiler/value-transfers-migration.md` § *Third-party
//! commands*, Tier 2, delivered in the Rust form): the direct routes over
//! shared cores proven against tcllib itself, and the declarations of the
//! procedures that write a caller's variable with no route.

use tcl_cmd_core::base32::{self, Alphabet};

use crate::arg_role::ArgRole;
use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{BindingKind, DependencyEvidence, EvalAnswer, TransferAnswer};
use super::builtins::{MayWriteSemantics, exact_operands, pure_outcome, result_type_transfer};
use super::const_ops::{ConstOps, ConstValue, Needs, Representation};
use super::context::Budget;
use super::decline::{DeclineReason, NoRouteReason};
use super::inputs::{AnalysisInputs, FactDomain};
use super::route::{EvalRoute, NativeEvalId};

/// The revision of the registry-owned tcllib evaluators.
const REVISION: u64 = 1;

/// `base32::encode`, `base32::decode` and their `base32::hex` twins on the
/// direct route over `tcl_cmd_core::base32`. An encoding reads its word a
/// byte per character and declines a character past U+00FF, which tcllib
/// 2.0 encodes as its UTF-8 bytes under every release from 8.5 alike
/// (`base32::encode "€"` is `4KBKY===` on tclsh 8.5.19 to 9.1.0, D352), a
/// reading the core does not model; a decoding answers a canonical encoding
/// only and declines the rest, which the package raises for or its two
/// implementations read apart. A non-ASCII word is admitted only where the
/// target decodes source as UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Base32Semantics {
    id: NativeEvalId,
    alphabet: Alphabet,
    decodes: bool,
}

/// `base32::encode string`.
pub static BASE32_ENCODE: Base32Semantics = Base32Semantics {
    id: NativeEvalId::Base32Encode,
    alphabet: Alphabet::Standard,
    decodes: false,
};

/// `base32::decode estring`.
pub static BASE32_DECODE: Base32Semantics = Base32Semantics {
    id: NativeEvalId::Base32Decode,
    alphabet: Alphabet::Standard,
    decodes: true,
};

/// `base32::hex::encode string`.
pub static BASE32_HEX_ENCODE: Base32Semantics = Base32Semantics {
    id: NativeEvalId::Base32HexEncode,
    alphabet: Alphabet::ExtendedHex,
    decodes: false,
};

/// `base32::hex::decode estring`.
pub static BASE32_HEX_DECODE: Base32Semantics = Base32Semantics {
    id: NativeEvalId::Base32HexDecode,
    alphabet: Alphabet::ExtendedHex,
    decodes: true,
};

impl Base32Semantics {
    /// The axes the core reads.
    pub const NEEDS: Needs = Needs::SOURCE_ENCODING;

    /// The type of the result: the packages' decode builds a byte array.
    const fn result_type(self) -> TclType {
        if self.decodes {
            TclType::ByteArray
        } else {
            TclType::String
        }
    }

    fn run(self, text: &str) -> Option<ConstValue> {
        if self.decodes {
            let bytes = base32::decode(text, self.alphabet).ok()?;
            let chars: String = bytes.iter().copied().map(char::from).collect();
            Some(ConstValue::bytes(
                chars.as_bytes(),
                Representation::ByteArray,
            ))
        } else {
            let bytes = text
                .chars()
                .map(|c| u8::try_from(u32::from(c)).ok())
                .collect::<Option<Vec<u8>>>()?;
            Some(ConstValue::text(&base32::encode(&bytes, self.alphabet)))
        }
    }
}

impl CommandSemantics for Base32Semantics {
    fn identity(&self) -> &'static str {
        self.id.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct { id: self.id }
    }

    fn transfer(
        &self,
        domain: FactDomain,
        _input: &dyn AnalysisInputs,
        _budget: &mut Budget,
    ) -> TransferAnswer {
        result_type_transfer(domain, self.result_type())
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let view = input.invocation();
        let values = match exact_operands(input, view.argument_offset..view.operands.len()) {
            Ok(values) => values,
            Err(answer) => return answer,
        };
        // A wrong word count is the procedure's own `wrong # args` error.
        let [word] = values.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        let mut ops = match ConstOps::admit(input.context(), budget, Self::NEEDS) {
            Ok(ops) => ops,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let target = *ops.target();
        let text = match ops.admissible_text(word) {
            Ok(text) => text,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        // Linear in the word: charged before the core runs.
        if let Err(reason) = ops.charge(u64::try_from(text.len()).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let Some(value) = self.run(&text) else {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        };
        match ops.take(value) {
            Ok(value) => pure_outcome(
                self.id,
                REVISION,
                value,
                self.result_type(),
                DependencyEvidence {
                    release: target.release,
                    ..DependencyEvidence::default()
                },
            ),
            Err(reason) => EvalAnswer::Declined(reason),
        }
    }
}

/// A tcllib procedure that computes what it writes into a caller's
/// variable from its words alone, for which no route is authored yet:
/// `base32::core::define` and `valid`, and `cmdline`'s option readers,
/// each writing scalars.
pub static WRITES_UNAUTHORED: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Unauthored,
    kind: Some(BindingKind::Scalar),
};

/// A procedure that binds a caller's variable for each element and runs
/// the expression or body the caller passes over it
/// (`math::statistics::filter`, `struct::list mapfor`): a callback, which
/// needs a declared route of its own.
pub static LOOP_CALLBACK: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Callback,
    kind: Some(BindingKind::Scalar),
};

/// A procedure whose written value the file system decides
/// (`fileutil::foreachLine`'s lines, `fileutil::test`'s message): never
/// satisfiable from the source alone.
pub static WRITES_PLATFORM: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Platform,
    kind: Some(BindingKind::Scalar),
};

/// `tie::tie arrayName …`: the array takes the data source's contents and
/// becomes externally mutable through the traces the call installs — a
/// callback, as `trace add variable` is.
pub static TIE: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Callback,
    kind: Some(BindingKind::Array),
};

/// `tie::untie arrayName ?token?`: the call removes the traces `tie::tie`
/// installed and binds nothing, so it states no kind.
pub static UNTIE: MayWriteSemantics = MayWriteSemantics {
    targets: &[ArgRole::VarWrite],
    reason: NoRouteReason::Callback,
    kind: None,
};
