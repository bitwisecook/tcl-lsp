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

//! Republishing a pack set whose hook programs are unchanged keeps the
//! evaluator epoch. The language server's query database re-keys every
//! memoised lattice on that epoch, so moving it for a pack change that left
//! every hook body alone (another pack gained or lost a plain command) made
//! the server re-analyse and retain a new lattice generation per reload
//! (#1657). Its own binary: the epoch is process-wide.

use std::path::PathBuf;

use tcl_spectcl::discovery::{Origin, PackFile, Tier};

const HOOKED: &str = r"
speclib hooked 2.0 {
    command hooked::last {
        arity 0..
        arg_role_resolver {words ctx} {
            set n [llength $words]
            if {$n >= 1} { role [expr {$n - 1}] Body }
        }
        arg_role_resolver_roles Body
    }
}
";

const PLAIN: &str = r"
speclib plain 2.0 {
    command plain::noop {
        arity 1
    }
}
";

fn pack(path: &str, source: &str) -> (PackFile, String) {
    (
        PackFile {
            tier: Tier::Workspace,
            path: PathBuf::from(path),
            origin: Origin::DotDir,
            dependency_tier: None,
        },
        source.to_owned(),
    )
}

fn publish(packs: Vec<(PackFile, String)>) -> u64 {
    let set = tcl_spectcl::pack::load_in_memory(packs);
    tcl_spectcl::hooks::publish(&set);
    tcl_registry::pack_hooks::evaluator_epoch()
}

#[test]
fn an_unchanged_hook_plan_keeps_the_evaluator_epoch_1657() {
    let hooked = || pack("/ws/.tcl-lsp/hooked.tclspec", HOOKED);
    let plain = || pack("/ws/.tcl-lsp/extensions.tclspec", PLAIN);
    let first = publish(vec![hooked()]);
    assert_eq!(
        publish(vec![hooked(), plain()]),
        first,
        "a plain pack arriving changes no evaluator",
    );
    assert_eq!(
        publish(vec![hooked()]),
        first,
        "the plain pack leaving changes no evaluator",
    );
    let edited = HOOKED.replace("$n - 1", "$n - 2");
    assert!(
        publish(vec![pack("/ws/.tcl-lsp/hooked.tclspec", &edited)]) > first,
        "control: an edited hook body is a new evaluator",
    );
}
