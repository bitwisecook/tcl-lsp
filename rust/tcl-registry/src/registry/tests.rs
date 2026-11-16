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

use super::{CommandRegistry, MethodDispatchKind};
use crate::ArgRole;
use crate::forms::LiteralArgumentPrefix;
use crate::spec::ArgRoleResolver;
use std::collections::BTreeSet;
use std::sync::Arc;
use tcl_dialect::DialectProfile;
use tcl_dialect::model::SpecProvider;
use tcl_dialect::model::{Family, SpecSurface, SurfaceLayer, SurfaceQuery};
use tcl_dialect::surface;

mod argument_roles;
mod body_roles;
mod construction;
mod control_arms;
mod effective_semantics;
mod invocation_completion;
mod invocation_resolution;
mod irules;
mod lookup;
mod native_registration;
mod object_model;
mod script_timing;
mod stamp_windows;
mod trait_queries;
mod variable_projection;

fn origin() -> crate::pack_origin::PackOrigin {
    crate::pack_origin::PackOrigin {
        pack: "vendor".to_owned(),
        content_hash: 7,
        vocabulary_version: "2".to_owned(),
        provenance: tcl_dialect::model::Provenance::BundledPack,
    }
}

use super::*;

// Option-value roles.

fn opt(name: &'static str, value: crate::hover::OptionValue) -> crate::hover::OptionSpec {
    crate::hover::OptionSpec {
        name,
        value,
        ..crate::hover::OptionSpec::DEFAULT
    }
}

fn opt_with_alias(
    name: &'static str,
    aliases: &'static [&'static str],
    value: crate::hover::OptionValue,
) -> crate::hover::OptionSpec {
    crate::hover::OptionSpec {
        name,
        value,
        aliases,
        ..crate::hover::OptionSpec::DEFAULT
    }
}

fn indices(options: &[crate::hover::OptionSpec], args: &[&str], role: ArgRole) -> Vec<usize> {
    let mut out = Vec::new();
    push_option_value_roles(
        &mut out,
        &options.iter().collect::<Vec<_>>(),
        args,
        0,
        &|wanted| wanted == role,
        PrefixMatching::Enabled,
    );
    out.into_iter().map(|(index, _)| index).collect()
}

/// Explicit metadata environment; this does not install a source receiver.
fn callback_metadata_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    for (package, version) in [("Tk", "9.1"), ("tcltest", "2.5.8"), ("bibtex", "0.8")] {
        registry.insert_ambient_package(package, version);
    }
    registry
}
