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

use super::*;
use crate::analyser::Analyser;
use crate::analyser::types::MemberSide;
use tcl_registry::definer::DefinitionSpecialMemberKind;

#[test]
fn actual_source_members_do_not_decode_lifecycle_display_names() {
    let source = "oo::class create C {method {<constructor>} {} {next}; constructor {} {next}; destructor {next}}";
    for dialect in ["tcl", "tcl8.6"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
        let mut offsets = source
            .match_indices("next")
            .map(|(offset, _)| u32::try_from(offset).unwrap());
        let ordinary =
            original_source_member_at(&analysis, &image, config, offsets.next().unwrap()).unwrap();
        if dialect == "tcl" {
            assert_eq!(
                ordinary.1,
                SourceMemberKind::Method {
                    name: "<constructor>".to_string(),
                    side: MemberSide::Instance
                }
            );
        } else {
            let SourceMemberKind::NativeMethod { input, side } = ordinary.1 else {
                panic!("genuine Native name input")
            };
            assert_eq!(input.bytes(), b"<constructor>");
            assert_eq!(side, MemberSide::Instance);
        }
        let constructor =
            original_source_member_at(&analysis, &image, config, offsets.next().unwrap()).unwrap();
        assert_eq!(
            constructor.1,
            SourceMemberKind::Special(DefinitionSpecialMemberKind::Constructor)
        );
        let destructor =
            original_source_member_at(&analysis, &image, config, offsets.next().unwrap()).unwrap();
        assert_eq!(
            destructor.1,
            SourceMemberKind::Special(DefinitionSpecialMemberKind::Destructor)
        );
        let foreign = tcl_lexer::SourceImage::document(&format!("{source} changed"));
        assert!(original_source_member_at(&analysis, &foreign, config, 0).is_none());
    }
}
