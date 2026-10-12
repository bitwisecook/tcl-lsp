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

//! Independent, byte-controlled scriptd and tmsh observations.

use super::{
    BigIpExecutionContext, CommandPresence, EmbeddedRuntimeEvidence, GlobalValue, ProbeSetId,
    RuntimeFact, appliance, row, surface,
};

const SCRIPT_D: &str = include_str!(
    "../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l/scriptd-results.txt"
);
const CLI: &str = include_str!(
    "../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006l/cli-run.txt"
);
const PROVENANCE: super::EvidenceProvenance = appliance(
    ProbeSetId::RESOLUTION_CONTEXTS,
    "scripts/dev/bigip-probes/resolution-2286/BIGIP_RESULTS.md",
    false,
);

fn field(line: &'static str, name: &str) -> Option<&'static str> {
    line.split('|').find_map(|part| part.strip_prefix(name))
}

pub(super) fn rows() -> Vec<EmbeddedRuntimeEvidence> {
    let mut rows = Vec::new();
    for transcript in [SCRIPT_D, CLI] {
        for line in transcript.lines() {
            let Some(case) = field(line, "case=") else {
                continue;
            };
            let context = match line.split('|').nth(2) {
                Some("IAppImplementation") => BigIpExecutionContext::IAppImplementation,
                Some("ICallScript") => BigIpExecutionContext::ICallScript,
                Some("TmshCliScript") => BigIpExecutionContext::TmshCliScript,
                _ => panic!("unidentified embedded runtime transcript context"),
            };
            rows.push(row(
                context,
                RuntimeFact::ParserCase {
                    case,
                    source_hex: field(line, "source_hex=").expect("original source bytes"),
                    code: field(line, "rc=")
                        .expect("completion code")
                        .parse()
                        .expect("Tcl completion code"),
                    value_hex: field(line, "value_hex=").expect("completion result bytes"),
                },
                PROVENANCE,
            ));
        }
    }
    let context = BigIpExecutionContext::ICallScript;
    rows.extend([
        row(
            context,
            RuntimeFact::ReportedPatchlevel {
                info_patchlevel: "8.4.6",
                tcl_patch_level_global: GlobalValue::Present("8.4.6"),
            },
            PROVENANCE,
        ),
        row(context, RuntimeFact::CommandCount(95), PROVENANCE),
        row(
            context,
            RuntimeFact::TmshVersion(Some("21.1.0.1")),
            PROVENANCE,
        ),
        surface(context, "exec", CommandPresence::Present, PROVENANCE),
    ]);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::f5::evidence::{BigIpBuild, RuntimeFactKind, measured_fact};

    #[test]
    fn original_context_scripts_keep_independent_byte_attested_results() {
        let rows = rows();
        let mut seen = std::collections::HashSet::new();
        for context in [
            BigIpExecutionContext::ICallScript,
            BigIpExecutionContext::IAppImplementation,
            BigIpExecutionContext::TmshCliScript,
        ] {
            let mut count = 0;
            for record in rows.iter().filter(|r| r.context == context) {
                let RuntimeFact::ParserCase {
                    case,
                    source_hex,
                    code,
                    value_hex,
                } = record.fact
                else {
                    continue;
                };
                assert!(seen.insert((context, case)), "{context}/{case}");
                for hex in [source_hex, value_hex] {
                    assert_eq!(hex.len() % 2, 0);
                    assert!(hex.bytes().all(|b| b.is_ascii_hexdigit()));
                }
                assert!(code <= 4);
                assert_eq!(
                    measured_fact(context, record.build, RuntimeFactKind::ParserCase(case)),
                    Some(record)
                );
                let other_build = BigIpBuild {
                    release: "17.1.0",
                    build: "0.0.1",
                };
                assert!(
                    measured_fact(context, other_build, RuntimeFactKind::ParserCase(case))
                        .is_none()
                );
                count += 1;
            }
            assert_eq!(count, 92, "{context}");
        }
        assert!(
            measured_fact(
                BigIpExecutionContext::TmmIRule,
                BigIpBuild::MEASURED_21_1_0_1,
                RuntimeFactKind::ParserCase("g_if_chain")
            )
            .is_none()
        );
        assert!(
            measured_fact(
                BigIpExecutionContext::ICallScript,
                BigIpBuild::MEASURED_21_1_0_1,
                RuntimeFactKind::TclPlatform
            )
            .is_none()
        );
    }

    #[test]
    fn icall_runtime_facts_are_backed_by_its_own_report_and_entered_commands() {
        let report = SCRIPT_D
            .lines()
            .find(|line| line.starts_with("R2286DEEP|r2286l|ICallScript|REPORTED|"))
            .unwrap();
        assert_eq!(field(report, "patchlevel="), Some("8.4.6"));
        assert_eq!(field(report, "tcl_patchLevel="), Some("8.4.6"));
        assert_eq!(field(report, "ncommands="), Some("95"));
        assert_eq!(field(report, "tmshversion="), Some("21.1.0.1"));
        let entered_exec = SCRIPT_D
            .lines()
            .find(|line| line.starts_with("R2286DEEP|r2286l|ICallScript|case=p_exec|"))
            .unwrap();
        assert_eq!(
            field(entered_exec, "source_hex="),
            Some("65786563202f62696e2f74727565")
        );
        assert_eq!(field(entered_exec, "rc="), Some("0"));
    }
}
