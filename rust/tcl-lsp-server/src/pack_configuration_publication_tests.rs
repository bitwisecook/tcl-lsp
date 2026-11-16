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

#[tokio::test]
async fn configured_receipt_requires_real_selected_sources_and_same_installed_generation() {
    let backend = crate::tests::test_backend();
    let source_path = PathBuf::from("/workspace/requested-pack.tclspec");
    let options = tcl_spectcl::DiscoveryOptions {
        configured: vec![source_path.clone()],
        ..tcl_spectcl::DiscoveryOptions::default()
    };
    let packs = Arc::new(tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: source_path.clone(),
            origin: tcl_spectcl::discovery::Origin::Setting,
            dependency_tier: None,
        },
        "speclib requested 1.0 {\ncommand requested::call {\narity 0\n}\n}\n".to_owned(),
    )]));
    assert_eq!(
        packs.load_errors,
        [] as [(std::path::PathBuf, tcl_spectcl::LoadError); 0]
    );
    let mut publication = PackConfigurationPublication {
        options,
        configured_sources: vec![vec![source_path]],
        seq: 1,
        packs,
        studio_snapshot: backend.studio_sources.lock().await.snapshot(),
        loader_inputs: tcl_spectcl::pack::LoadedSourceInputs::default(),
        overlays: Vec::new(),
    };
    assert!(publication.has_configured_contributions());
    let mut installed = PublishedPackSet {
        seq: publication.seq,
        packs: Arc::clone(&publication.packs),
    };
    assert!(publication.matches_installed(&publication.options, &installed));
    installed.seq += 1;
    assert!(!publication.matches_installed(&publication.options, &installed));
    installed.seq = publication.seq;
    installed.packs = Arc::new((*publication.packs).clone());
    assert_eq!(installed.packs.key, publication.packs.key);
    assert!(!publication.matches_installed(&publication.options, &installed));
    installed.packs = Arc::clone(&publication.packs);
    let mut unrelated = publication.options.clone();
    unrelated.configured = vec![PathBuf::from("/workspace/old.tclspec")];
    assert!(!publication.matches_installed(&unrelated, &installed));
    publication.configured_sources[0].clear();
    assert!(!publication.has_configured_contributions());
    publication.configured_sources[0].push(PathBuf::from("/workspace/unrelated.tclspec"));
    assert!(!publication.has_configured_contributions());
    assert!(backend.spec_pack_configuration_report().await.is_none());
}
