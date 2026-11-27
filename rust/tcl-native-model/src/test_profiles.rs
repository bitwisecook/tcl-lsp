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

//! Dialect profiles for this crate's tests, from the compiled environment
//! catalogue: a catalogue environment's own profile, a profile projected from
//! the point of an environment with a ladder but no catalogue row (`jim`),
//! Tk's additive profile, and the permissive fallback for everything else.
//! The registry's environment ingress hands a unit the same profiles.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tcl_dialect::DialectProfile;
use tcl_dialect::model::{EnvironmentDefinition, EnvironmentRegistry, LENIENT_ENVIRONMENT_ID};

/// One resolved environment of the compiled catalogue.
pub(crate) struct TestEnvironment {
    definition: Arc<EnvironmentDefinition>,
}

impl TestEnvironment {
    /// The analyser's profile for this environment.
    pub(crate) fn analyser_profile(&self) -> &'static DialectProfile {
        let id = self.definition.id.as_str();
        if let Some(profile) = DialectProfile::find(id) {
            return profile;
        }
        if id != "tk"
            && id != LENIENT_ENVIRONMENT_ID
            && let Some(point) = self.definition.point()
        {
            static PROJECTED: OnceLock<Mutex<HashMap<String, &'static DialectProfile>>> =
                OnceLock::new();
            let leak = |s: &str| -> &'static str { Box::leak(s.to_owned().into_boxed_str()) };
            let mut projected = PROJECTED.get_or_init(Default::default).lock().unwrap();
            return projected.entry(id.to_owned()).or_insert_with(|| {
                let aliases: Vec<&'static str> =
                    self.definition.aliases.iter().map(|a| leak(a)).collect();
                DialectProfile::projected_from_point(
                    leak(id),
                    Box::leak(aliases.into_boxed_slice()),
                    leak(&self.definition.display_name),
                    point,
                )
                .intern()
            });
        }
        DialectProfile::plain_tcl()
    }

    /// The profile a compilation unit for this environment is built for.
    pub(crate) fn unit_profile(&self) -> &'static DialectProfile {
        if self.definition.id.as_str() == "tk" {
            DialectProfile::tk()
        } else {
            self.analyser_profile()
        }
    }
}

/// The environment `name` names, or the lenient `tcl` environment.
pub(crate) fn resolve_environment(name: &str) -> TestEnvironment {
    resolve_known_environment(name).unwrap_or_else(|| {
        resolve_known_environment(LENIENT_ENVIRONMENT_ID)
            .expect("the compiled catalogue seeds the lenient `tcl` environment")
    })
}

/// The environment `name` names, when it names one.
pub(crate) fn resolve_known_environment(name: &str) -> Option<TestEnvironment> {
    let definition = EnvironmentRegistry::compiled().resolve(name)?;
    Some(TestEnvironment { definition })
}

/// The unit profile of a known native fixture provider.
pub(crate) fn profile(provider: &str) -> &'static DialectProfile {
    resolve_known_environment(provider)
        .expect("native fixture requires a known provider environment")
        .unit_profile()
}
