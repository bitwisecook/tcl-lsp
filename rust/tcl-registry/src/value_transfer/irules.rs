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

//! The pure iRules functions on the direct route
//! (`docs/design/compiler/value-transfers-migration.md` § *Third-party
//! commands*, Tier 1): each a registry-owned evaluator over its shared core,
//! `tcl_cmd_core::irules::call`, which the iRule test simulator registers as
//! the same command. A core answers only what F5's published reference
//! states, and the route declines everything else as unsupported.

use tcl_cmd_core::irules::{self, Output};

use crate::types::TclType;

use super::CommandSemantics;
use super::answers::{DependencyEvidence, EvalAnswer, TransferAnswer};
use super::builtins::{exact_operands, pure_outcome, result_type_transfer};
use super::const_ops::{ConstOps, ConstValue, Needs, Representation};
use super::context::Budget;
use super::decline::DeclineReason;
use super::inputs::{AnalysisInputs, FactDomain};
use super::route::{EvalRoute, NativeEvalId};

/// The revision of the registry-owned iRules evaluators.
const REVISION: u64 = 1;

/// One pure iRules function on the direct route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrulesFunctionSemantics {
    command: &'static str,
    id: NativeEvalId,
}

/// `b64encode string`.
pub static B64ENCODE: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "b64encode",
    id: NativeEvalId::Base64Encode,
};
/// `b64decode string`.
pub static B64DECODE: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "b64decode",
    id: NativeEvalId::Base64Decode,
};
/// `crc32 string`.
pub static CRC32: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "crc32",
    id: NativeEvalId::Crc32Checksum,
};
/// `md5 string`.
pub static MD5: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "md5",
    id: NativeEvalId::Md5Digest,
};
/// `sha1 string`.
pub static SHA1: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "sha1",
    id: NativeEvalId::Sha1Digest,
};
/// `sha256 string`.
pub static SHA256: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "sha256",
    id: NativeEvalId::Sha256Digest,
};
/// `sha384 string`.
pub static SHA384: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "sha384",
    id: NativeEvalId::Sha384Digest,
};
/// `sha512 string`.
pub static SHA512: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "sha512",
    id: NativeEvalId::Sha512Digest,
};
/// `findstr string search_string ?skip_count ?terminator??`.
pub static FINDSTR: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "findstr",
    id: NativeEvalId::FindString,
};
/// `getfield string split field_number`.
pub static GETFIELD: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "getfield",
    id: NativeEvalId::StringField,
};
/// `substr string skip_count ?terminator?`.
pub static SUBSTR: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "substr",
    id: NativeEvalId::Substring,
};
/// `domain string count`.
pub static DOMAIN: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "domain",
    id: NativeEvalId::DomainLabels,
};
/// `URI::basename uri`.
pub static URI_BASENAME: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::basename",
    id: NativeEvalId::UriBasename,
};
/// `URI::path uri ?depth | start ?end??`.
pub static URI_PATH: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::path",
    id: NativeEvalId::UriPath,
};
/// `URI::query uri ?name?`.
pub static URI_QUERY: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::query",
    id: NativeEvalId::UriQuery,
};
/// `URI::host uri`.
pub static URI_HOST: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::host",
    id: NativeEvalId::UriHost,
};
/// `URI::port uri`.
pub static URI_PORT: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::port",
    id: NativeEvalId::UriPort,
};
/// `URI::protocol uri`.
pub static URI_PROTOCOL: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::protocol",
    id: NativeEvalId::UriProtocol,
};
/// `URI::decode uri`.
pub static URI_DECODE: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::decode",
    id: NativeEvalId::UriDecode,
};
/// `URI::encode uri`.
pub static URI_ENCODE: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::encode",
    id: NativeEvalId::UriEncode,
};
/// `URI::compare uri1 uri2`.
pub static URI_COMPARE: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "URI::compare",
    id: NativeEvalId::UriCompare,
};
/// `IP::addr addr1[/mask] equals addr2[/mask]`.
pub static IP_ADDR: IrulesFunctionSemantics = IrulesFunctionSemantics {
    command: "IP::addr",
    id: NativeEvalId::IpAddrEquals,
};

impl IrulesFunctionSemantics {
    /// The iRules command the core runs.
    #[must_use]
    pub const fn command(self) -> &'static str {
        self.command
    }

    /// The catalogued evaluator.
    #[must_use]
    pub const fn evaluator(self) -> NativeEvalId {
        self.id
    }

    /// The type of the function's result.
    #[must_use]
    pub const fn result_type(self) -> TclType {
        match self.id {
            NativeEvalId::Crc32Checksum
            | NativeEvalId::UriPort
            | NativeEvalId::UriCompare
            | NativeEvalId::IpAddrEquals => TclType::Int,
            NativeEvalId::Base64Decode
            | NativeEvalId::Md5Digest
            | NativeEvalId::Sha1Digest
            | NativeEvalId::Sha256Digest
            | NativeEvalId::Sha384Digest
            | NativeEvalId::Sha512Digest => TclType::ByteArray,
            _ => TclType::String,
        }
    }
}

impl CommandSemantics for IrulesFunctionSemantics {
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
        let mut ops = match ConstOps::admit(input.context(), budget, Needs::NONE) {
            Ok(ops) => ops,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        let target = *ops.target();
        let mut words = Vec::with_capacity(values.len());
        for value in &values {
            match ops.text_of(value) {
                Ok(text) => words.push(text),
                Err(reason) => return EvalAnswer::Declined(reason),
            }
        }
        // Every core is linear in its input: charge it before it runs.
        let input_bytes = words.iter().map(|word| word.len()).sum::<usize>();
        if let Err(reason) = ops.charge(u64::try_from(input_bytes).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let words: Vec<&str> = words.iter().map(AsRef::as_ref).collect();
        let value = match irules::call(self.command, &words) {
            Ok(Output::Text(text)) => ConstValue::text(&text),
            Ok(Output::Int(value)) => ConstValue::int(value),
            Ok(Output::Bytes(bytes)) => {
                let text: String = bytes.iter().copied().map(char::from).collect();
                ConstValue::bytes(text.as_bytes(), Representation::ByteArray)
            }
            // An answer outside the reference is not the analysis's to give.
            Err(irules::Unmodelled) => return EvalAnswer::Declined(DeclineReason::Unsupported),
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
