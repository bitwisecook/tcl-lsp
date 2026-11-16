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
use crate::model::{BuildProfileId, DialectPoint, Release};

#[test]
fn snapshot_keys_distinguish_release_build_and_policies_under_one_name() {
    const EXAMPLE: &[PackageFloor<'static>] = &[PackageFloor::named("Example")];
    const EXAMPLE_2: &[PackageFloor<'static>] = &[PackageFloor::at("Example", "2.0")];
    let profile = DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        DialectPoint::canonical(Release::JIM_0_80),
    );
    let mut changed = profile.clone();
    changed.core_point = Some(DialectPoint::canonical(Release::JIM_0_84));
    assert_ne!(profile.cache_key(), changed.cache_key());
    changed = profile.clone();
    changed.core_point = Some(DialectPoint::new(
        Release::JIM_0_80,
        BuildProfileId::Unknown,
    ));
    assert_ne!(profile.cache_key(), changed.cache_key());
    changed = profile.clone();
    changed.grammar.numbers = crate::NumberSyntax::Tcl90;
    assert_ne!(profile.cache_key(), changed.cache_key());
    changed = profile.clone();
    changed.leading_zero_is_octal = Ternary::Yes;
    assert_ne!(profile.cache_key(), changed.cache_key());
    changed = profile.clone();
    changed.surface_packages = EXAMPLE;
    assert_ne!(profile.cache_key(), changed.cache_key());
    assert!(std::ptr::eq(profile.intern(), profile.clone().intern()));
    assert!(!std::ptr::eq(profile.intern(), changed.intern()));
    let without_floor = changed.cache_key();
    changed.surface_packages = EXAMPLE_2;
    assert_ne!(without_floor, changed.cache_key());
    changed = profile.clone();
    changed.evaluation_evidence = EvaluationEvidence::MeasuredFork {
        note: "explicit test evidence",
    };
    assert_ne!(profile.cache_key(), changed.cache_key());
    assert_eq!(
        changed.cache_key().profile().evaluation_evidence,
        changed.evaluation_evidence,
    );
}

#[test]
fn canonical_snapshots_keep_catalogue_handle_identity() {
    for profile in DialectProfile::all()
        .iter()
        .chain([DialectProfile::plain_tcl(), DialectProfile::tk()])
    {
        assert!(std::ptr::eq(profile, profile.clone().intern()));
    }
}
