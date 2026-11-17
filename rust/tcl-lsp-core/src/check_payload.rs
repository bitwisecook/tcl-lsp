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

//! Versioned compiler-check payload transport for LSP, CLI and MCP consumers.
//!
//! Names travel as byte arrays. Decoding checks the version, the code and the
//! code's payload family; it never rebuilds a compiler payload or source sink.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tcl_compiler::compiler_checks::{CheckPayload, CheckPayloadKind, Diagnostic};
use tcl_compiler::irules_checks::IrulesCheckPayload;
use tcl_core_types::DiagCode;
use tcl_registry::types::TclType;

const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RepresentationDescription {
    String,
    Int,
    Double,
    Boolean,
    List,
    Dict,
    ByteArray,
    Numeric,
    Object,
    Channel,
}

impl From<TclType> for RepresentationDescription {
    fn from(original: TclType) -> Self {
        match original {
            TclType::String => Self::String,
            TclType::Int => Self::Int,
            TclType::Double => Self::Double,
            TclType::Boolean => Self::Boolean,
            TclType::List => Self::List,
            TclType::Dict => Self::Dict,
            TclType::ByteArray => Self::ByteArray,
            TclType::Numeric => Self::Numeric,
            TclType::Object => Self::Object,
            TclType::Channel => Self::Channel,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PayloadDescription {
    ConstantBranch {
        value: bool,
    },
    Redundancy {
        first_start: u32,
        first_end: u32,
        expression: Vec<u8>,
    },
    Representation {
        variable: Vec<u8>,
        from_type: RepresentationDescription,
        to_type: RepresentationDescription,
        command: Vec<u8>,
        in_loop: bool,
    },
    Oscillation {
        variable: Vec<u8>,
        first_type: RepresentationDescription,
        second_type: RepresentationDescription,
    },
    SharedCopy {
        variable: Vec<u8>,
        shared_with: Vec<u8>,
        command: Vec<u8>,
    },
    Taint {
        variable: Vec<u8>,
        sink_command: Vec<u8>,
    },
    PathConcatenation {
        variable: Vec<u8>,
    },
    UnnormalisedGetter {
        command: Vec<u8>,
    },
    UnguardedDrop {
        command: Vec<u8>,
    },
    UnguardedDnsReturn,
    EventWithoutCollect {
        event: Vec<u8>,
        side: Vec<u8>,
        collect_commands: Vec<Vec<u8>>,
    },
    PayloadWithoutCollect {
        protocol: Vec<u8>,
        side: Vec<u8>,
    },
    CollectWithoutRelease {
        protocol: Vec<u8>,
        side: Vec<u8>,
    },
    ReleaseWithoutCollect {
        protocol: Vec<u8>,
        side: Vec<u8>,
    },
    ContextAfterResponse {
        command: Vec<u8>,
        event: Vec<u8>,
    },
    RepeatedResponse {
        command: Vec<u8>,
        event: Vec<u8>,
    },
    GenericStaticName {
        source_name: Vec<u8>,
        pattern_name: Vec<u8>,
    },
    HoistableAssignment {
        variable: Vec<u8>,
    },
}

fn bytes(name: &str) -> Vec<u8> {
    name.as_bytes().to_vec()
}

impl From<&CheckPayload> for PayloadDescription {
    fn from(original: &CheckPayload) -> Self {
        match original {
            CheckPayload::ConstantBranch { value } => Self::ConstantBranch { value: *value },
            CheckPayload::Redundancy {
                first_span,
                expression,
            } => Self::Redundancy {
                first_start: first_span.start(),
                first_end: first_span.end(),
                expression: bytes(expression),
            },
            CheckPayload::Representation {
                variable,
                from_type,
                to_type,
                command,
                in_loop,
            } => Self::Representation {
                variable: bytes(variable),
                from_type: (*from_type).into(),
                to_type: (*to_type).into(),
                command: bytes(command),
                in_loop: *in_loop,
            },
            CheckPayload::Oscillation {
                variable,
                first_type,
                second_type,
            } => Self::Oscillation {
                variable: bytes(variable),
                first_type: (*first_type).into(),
                second_type: (*second_type).into(),
            },
            CheckPayload::SharedCopy {
                variable,
                shared_with,
                command,
            } => Self::SharedCopy {
                variable: bytes(variable),
                shared_with: bytes(shared_with),
                command: bytes(command),
            },
            CheckPayload::Taint(subject) => Self::Taint {
                variable: bytes(subject.variable()),
                sink_command: bytes(subject.sink_command()),
            },
            CheckPayload::PathConcatenation { variable } => Self::PathConcatenation {
                variable: bytes(variable),
            },
            CheckPayload::Irules(payload) => Self::from(payload),
        }
    }
}

impl From<&IrulesCheckPayload> for PayloadDescription {
    fn from(original: &IrulesCheckPayload) -> Self {
        match original {
            IrulesCheckPayload::UnnormalisedGetter { command } => Self::UnnormalisedGetter {
                command: bytes(command),
            },
            IrulesCheckPayload::UnguardedDrop { command } => Self::UnguardedDrop {
                command: bytes(command),
            },
            IrulesCheckPayload::UnguardedDnsReturn => Self::UnguardedDnsReturn,
            IrulesCheckPayload::EventWithoutCollect {
                event,
                side,
                collect_commands,
            } => Self::EventWithoutCollect {
                event: bytes(event),
                side: bytes(side),
                collect_commands: collect_commands.iter().map(|name| bytes(name)).collect(),
            },
            IrulesCheckPayload::PayloadWithoutCollect { protocol, side } => {
                Self::PayloadWithoutCollect {
                    protocol: bytes(protocol),
                    side: bytes(side),
                }
            }
            IrulesCheckPayload::CollectWithoutRelease { protocol, side } => {
                Self::CollectWithoutRelease {
                    protocol: bytes(protocol),
                    side: bytes(side),
                }
            }
            IrulesCheckPayload::ReleaseWithoutCollect { protocol, side } => {
                Self::ReleaseWithoutCollect {
                    protocol: bytes(protocol),
                    side: bytes(side),
                }
            }
            IrulesCheckPayload::ContextAfterResponse { command, event } => {
                Self::ContextAfterResponse {
                    command: bytes(command),
                    event: bytes(event),
                }
            }
            IrulesCheckPayload::RepeatedResponse { command, event } => Self::RepeatedResponse {
                command: bytes(command),
                event: bytes(event),
            },
            IrulesCheckPayload::GenericStaticName {
                source_name,
                pattern_name,
            } => Self::GenericStaticName {
                source_name: bytes(source_name),
                pattern_name: bytes(pattern_name),
            },
            IrulesCheckPayload::HoistableAssignment { variable } => Self::HoistableAssignment {
                variable: bytes(variable),
            },
        }
    }
}

impl PayloadDescription {
    const fn kind(&self) -> CheckPayloadKind {
        match self {
            Self::ConstantBranch { .. } => CheckPayloadKind::ConstantBranch,
            Self::Redundancy { .. } => CheckPayloadKind::Redundancy,
            Self::Representation { .. } => CheckPayloadKind::Representation,
            Self::Oscillation { .. } => CheckPayloadKind::Oscillation,
            Self::SharedCopy { .. } => CheckPayloadKind::SharedCopy,
            Self::Taint { .. } => CheckPayloadKind::Taint,
            Self::PathConcatenation { .. } => CheckPayloadKind::PathConcatenation,
            Self::UnnormalisedGetter { .. } => CheckPayloadKind::UnnormalisedGetter,
            Self::UnguardedDrop { .. } => CheckPayloadKind::UnguardedDrop,
            Self::UnguardedDnsReturn => CheckPayloadKind::UnguardedDnsReturn,
            Self::EventWithoutCollect { .. } => CheckPayloadKind::EventWithoutCollect,
            Self::PayloadWithoutCollect { .. } => CheckPayloadKind::PayloadWithoutCollect,
            Self::CollectWithoutRelease { .. } => CheckPayloadKind::CollectWithoutRelease,
            Self::ReleaseWithoutCollect { .. } => CheckPayloadKind::ReleaseWithoutCollect,
            Self::ContextAfterResponse { .. } => CheckPayloadKind::ContextAfterResponse,
            Self::RepeatedResponse { .. } => CheckPayloadKind::RepeatedResponse,
            Self::GenericStaticName { .. } => CheckPayloadKind::GenericStaticName,
            Self::HoistableAssignment { .. } => CheckPayloadKind::HoistableAssignment,
        }
    }
}

/// Serialised compiler-check payload, shared by every protocol adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckPayloadData {
    version: u32,
    code: String,
    payload: PayloadDescription,
    /// Process-local source-advice handle, when the issuer retained one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_advice: Option<Value>,
}

impl CheckPayloadData {
    /// Project the issuer's typed payload; the message is never read.
    #[must_use]
    pub fn from_diagnostic(diagnostic: &Diagnostic) -> Self {
        Self {
            version: VERSION,
            code: diagnostic.code.as_str().to_owned(),
            payload: PayloadDescription::from(&diagnostic.payload),
            source_advice: None,
        }
    }

    /// Attach the process-local source-advice handle.
    #[must_use]
    pub fn with_source_advice(mut self, handle: Value) -> Self {
        self.source_advice = (!handle.is_null()).then_some(handle);
        self
    }

    /// Decode data for `code`. A future version, a different code, or a
    /// payload family the contract does not assign to `code` stays unknown.
    #[must_use]
    pub fn from_value(value: &Value, code: &str) -> Option<Self> {
        let data: Self = serde_json::from_value(value.clone()).ok()?;
        let expected = code.parse::<DiagCode>().ok()?;
        (data.version == VERSION && data.code == code && data.payload.kind().accepts(expected))
            .then_some(data)
    }

    /// Encode for an adapter.
    #[must_use]
    pub fn to_value(&self) -> Option<Value> {
        serde_json::to_value(self).ok()
    }

    /// Diagnostic code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Payload family.
    #[must_use]
    pub const fn kind(&self) -> CheckPayloadKind {
        self.payload.kind()
    }

    /// Process-local source-advice handle, if one was attached.
    #[must_use]
    pub const fn source_advice(&self) -> Option<&Value> {
        self.source_advice.as_ref()
    }

    /// Whether this is exactly the issuer payload's projection.
    #[must_use]
    pub fn describes(&self, payload: &CheckPayload) -> bool {
        self.payload == PayloadDescription::from(payload)
    }
}

/// One compiler-check diagnostic per payload family, with quoted, spaced,
/// qualified, control-character and non-ASCII names. Adapter crates use it to
/// test their wire output against [`CheckPayloadData`].
#[cfg(any(test, feature = "transport-fixtures"))]
#[must_use]
pub fn family_fixtures() -> Vec<Diagnostic> {
    use tcl_compiler::compiler_checks::{Severity, TaintDiagnosticSubject};
    use tcl_lexer::Span;
    const QUOTED: &str = "it's \"q\" {b} [c] $d";
    const SPACED: &str = "::a b::c(d e)";
    const NON_ASCII: &str = "ñame_é\u{ff}\u{1f600}";
    const CONTROL: &str = "nul\u{0}tab\tnl\n";
    let mut payloads = vec![
        (DiagCode::O100, CheckPayload::ConstantBranch { value: true }),
        (
            DiagCode::O106,
            CheckPayload::Redundancy {
                first_span: Span::new(3, 9),
                expression: format!("llength ${QUOTED}"),
            },
        ),
        (
            DiagCode::S101,
            CheckPayload::Representation {
                variable: NON_ASCII.to_owned(),
                from_type: TclType::String,
                to_type: TclType::Int,
                command: SPACED.to_owned(),
                in_loop: true,
            },
        ),
        (
            DiagCode::S102,
            CheckPayload::Oscillation {
                variable: CONTROL.to_owned(),
                first_type: TclType::List,
                second_type: TclType::String,
            },
        ),
        (
            DiagCode::S103,
            CheckPayload::SharedCopy {
                variable: QUOTED.to_owned(),
                shared_with: NON_ASCII.to_owned(),
                command: "lappend".to_owned(),
            },
        ),
        (
            DiagCode::T100,
            CheckPayload::Taint(TaintDiagnosticSubject::new(
                SPACED.to_owned(),
                NON_ASCII.to_owned(),
            )),
        ),
        (
            DiagCode::W201,
            CheckPayload::PathConcatenation {
                variable: QUOTED.to_owned(),
            },
        ),
    ];
    payloads.extend(irules_fixture_payloads());
    payloads
        .into_iter()
        .enumerate()
        .map(|(index, (code, payload))| {
            let start = u32::try_from(index).expect("fixture index fits u32");
            Diagnostic {
                span: Span::new(start, start + 1),
                code,
                category: "fixture".to_owned(),
                severity: Severity::Warning,
                message: format!("presentation {QUOTED}"),
                replacement: None,
                fixes: Vec::new(),
                source_context: None,
                payload,
            }
        })
        .collect()
}

#[cfg(any(test, feature = "transport-fixtures"))]
fn irules_fixture_payloads() -> Vec<(DiagCode, CheckPayload)> {
    const QUOTED: &str = "it's \"q\" {b} [c] $d";
    const SPACED: &str = "::a b::c(d e)";
    const NON_ASCII: &str = "ñame_é\u{ff}\u{1f600}";
    const CONTROL: &str = "nul\u{0}tab\tnl\n";
    vec![
        (
            DiagCode::Irule3102,
            CheckPayload::Irules(IrulesCheckPayload::UnnormalisedGetter {
                command: "HTTP::path".to_owned(),
            }),
        ),
        (
            DiagCode::Irule5002,
            CheckPayload::Irules(IrulesCheckPayload::UnguardedDrop {
                command: QUOTED.to_owned(),
            }),
        ),
        (
            DiagCode::Irule5004,
            CheckPayload::Irules(IrulesCheckPayload::UnguardedDnsReturn),
        ),
        (
            DiagCode::Irule1005,
            CheckPayload::Irules(IrulesCheckPayload::EventWithoutCollect {
                event: NON_ASCII.to_owned(),
                side: "request".to_owned(),
                collect_commands: vec!["HTTP::collect".to_owned(), SPACED.to_owned()],
            }),
        ),
        (
            DiagCode::Irule1006,
            CheckPayload::Irules(IrulesCheckPayload::PayloadWithoutCollect {
                protocol: QUOTED.to_owned(),
                side: "response".to_owned(),
            }),
        ),
        (
            DiagCode::Irule1007,
            CheckPayload::Irules(IrulesCheckPayload::CollectWithoutRelease {
                protocol: "HTTP".to_owned(),
                side: CONTROL.to_owned(),
            }),
        ),
        (
            DiagCode::Irule1008,
            CheckPayload::Irules(IrulesCheckPayload::ReleaseWithoutCollect {
                protocol: NON_ASCII.to_owned(),
                side: "request".to_owned(),
            }),
        ),
        (
            DiagCode::Irule1201,
            CheckPayload::Irules(IrulesCheckPayload::ContextAfterResponse {
                command: SPACED.to_owned(),
                event: "HTTP_REQUEST".to_owned(),
            }),
        ),
        (
            DiagCode::Irule1202,
            CheckPayload::Irules(IrulesCheckPayload::RepeatedResponse {
                command: "HTTP::respond".to_owned(),
                event: QUOTED.to_owned(),
            }),
        ),
        (
            DiagCode::Irule4002,
            CheckPayload::Irules(IrulesCheckPayload::GenericStaticName {
                source_name: format!("static::{NON_ASCII}"),
                pattern_name: format!("{NON_ASCII}(x y)"),
            }),
        ),
        (
            DiagCode::Irule4004,
            CheckPayload::Irules(IrulesCheckPayload::HoistableAssignment {
                variable: CONTROL.to_owned(),
            }),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{CheckPayloadData, family_fixtures};
    use tcl_compiler::compiler_checks::{CheckPayload, CheckPayloadKind};

    /// Implementation contract: every family's payload survives an encode,
    /// a JSON text round trip and a decode, byte for byte.
    #[test]
    fn every_family_round_trips_through_json_text() {
        let fixtures = family_fixtures();
        let mut kinds = std::collections::HashSet::new();
        for diagnostic in &fixtures {
            let value = CheckPayloadData::from_diagnostic(diagnostic)
                .to_value()
                .unwrap();
            let text = serde_json::to_string(&value).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
            let decoded = CheckPayloadData::from_value(&parsed, diagnostic.code.as_str())
                .unwrap_or_else(|| panic!("{} did not decode: {text}", diagnostic.code));
            assert!(decoded.describes(&diagnostic.payload), "{text}");
            assert_eq!(decoded.kind(), diagnostic.payload.kind());
            kinds.insert(decoded.kind());
        }
        assert_eq!(kinds.len(), 18, "one fixture per payload family");
    }

    #[test]
    fn names_travel_as_their_exact_bytes() {
        let fixture = family_fixtures()
            .into_iter()
            .find(|d| matches!(d.payload, CheckPayload::Taint(_)))
            .unwrap();
        let value = CheckPayloadData::from_diagnostic(&fixture)
            .to_value()
            .unwrap();
        let CheckPayload::Taint(subject) = &fixture.payload else {
            unreachable!()
        };
        assert_eq!(
            value["payload"]["variable"],
            serde_json::json!(subject.variable().as_bytes())
        );
        assert_eq!(
            value["payload"]["sinkCommand"],
            serde_json::json!(subject.sink_command().as_bytes())
        );
    }

    #[test]
    fn rewording_changes_no_payload_and_equal_messages_keep_distinct_payloads() {
        let fixtures = family_fixtures();
        let mut reworded = fixtures[0].clone();
        reworded.message = "entirely different 'wording'".to_owned();
        assert_eq!(
            CheckPayloadData::from_diagnostic(&reworded),
            CheckPayloadData::from_diagnostic(&fixtures[0])
        );
        assert_eq!(fixtures[1].message, fixtures[2].message);
        assert_ne!(
            CheckPayloadData::from_diagnostic(&fixtures[1]),
            CheckPayloadData::from_diagnostic(&fixtures[2])
        );
    }

    #[test]
    fn mismatched_future_or_malformed_data_stays_unknown() {
        let fixture = &family_fixtures()[0];
        let value = CheckPayloadData::from_diagnostic(fixture)
            .to_value()
            .unwrap();
        assert!(CheckPayloadData::from_value(&value, "O105").is_none());
        let mut forged = value.clone();
        forged["code"] = serde_json::json!("W201");
        assert!(CheckPayloadData::from_value(&forged, "W201").is_none());
        let mut future = value.clone();
        future["version"] = serde_json::json!(2);
        assert!(CheckPayloadData::from_value(&future, "O100").is_none());
        let mut extra = value.clone();
        extra["payload"]["guess"] = serde_json::json!("x");
        assert!(CheckPayloadData::from_value(&extra, "O100").is_none());
        assert!(CheckPayloadData::from_value(&serde_json::json!({}), "O100").is_none());
        assert!(CheckPayloadData::from_value(&value, "not-a-code").is_none());
        assert_eq!(
            CheckPayloadData::from_value(&value, "O100").unwrap().kind(),
            CheckPayloadKind::ConstantBranch
        );
    }
}
