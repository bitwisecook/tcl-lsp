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

#[test]
fn rename_association_requires_a_unique_declared_base_and_same_host_root() {
    let document = Path::new("/project/sub/main.tcl");
    let old = Path::new("/project/old.tcl");
    let one = project_source_association(document, &[], "../old.tcl", old).unwrap();
    assert_eq!(
        one.renamed_literal(Path::new("/project/new.tcl"))
            .as_deref(),
        Some("../new.tcl")
    );
    assert!(
        project_source_association(
            document,
            &[PathBuf::from("/project/other")],
            "../old.tcl",
            old
        )
        .is_none()
    );
    let duplicate = project_source_association(
        document,
        &[PathBuf::from("/project/sub")],
        "../old.tcl",
        old,
    )
    .unwrap();
    assert_eq!(duplicate, one);
    assert!(project_source_association(document, &[], "other.tcl", old).is_none());
    assert!(
        project_source_association(document, &[PathBuf::from("relative")], "../old.tcl", old)
            .is_none()
    );
    assert!(one.renamed_literal(Path::new("relative/new.tcl")).is_none());
    assert!(relative_source_path(Path::new("relative/new.tcl"), Path::new("relative")).is_none());
    assert!(project_source_association(document, &[], "~/old.tcl", old).is_none());
    assert!(project_source_association(document, &[], "~other/old.tcl", old).is_none());
    assert!(project_source_association(document, &[], "", old).is_none());
    #[cfg(unix)]
    {
        assert!(
            project_source_association(Path::new("//project/sub/main.tcl"), &[], "../old.tcl", old)
                .is_none()
        );
        assert!(project_source_association(document, &[], "//project/old.tcl", old).is_none());
        assert!(
            one.renamed_literal(Path::new("//project/new.tcl"))
                .is_none()
        );
    }
}
