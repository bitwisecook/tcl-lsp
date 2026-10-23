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

use std::collections::HashMap;
use tcl_compiler::alias::{CommandAliasMap, resolve_alias};
use tcl_compiler::auto_path_eval::PathConstantLookup;
use tcl_compiler::auto_path_eval::{
    constant_path_assignments, fold_constant_assignments, fold_constant_assignments_with_imports,
};
use tcl_compiler::command_binding::SourceCommandBindings;
use tcl_dialect::TclVersion;
use tcl_registry::model::ingress::resolve_environment;

#[test]
fn alias_selection_obeys_command_order_before_inspecting_aliases() {
    let aliases = CommandAliasMap::from([
        ("::pick".into(), ("list".into(), vec!["global".into()])),
        ("::ns::pick".into(), ("list".into(), vec!["current".into()])),
        ("::ns::".into(), ("list".into(), vec!["empty".into()])),
        ("::via::pick".into(), ("list".into(), vec!["path".into()])),
        (
            "::local".into(),
            ("list".into(), vec!["global_alias".into()]),
        ),
    ]);
    assert_eq!(
        resolve_alias("pick", &aliases, "::ns").unwrap().1,
        ["current"]
    );
    assert_eq!(
        resolve_alias("::ns:::pick", &aliases, "::").unwrap().1,
        ["current"]
    );
    assert_eq!(
        resolve_alias("::ns:::", &aliases, "::").unwrap().1,
        ["empty"]
    );
    for version in TclVersion::ALL {
        let context = tcl_registry::model::ingress::static_context_for(version.dialect_name());
        let registry = context.commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let mut cases = vec![(
            "interp alias {} ::pick {} list global; namespace eval other {proc pick {} {return ordinary}; pick}",
            "pick",
            "::other::pick",
            None,
        )];
        if version >= TclVersion::V8_5 {
            cases.extend([
                ("namespace eval via {}; interp alias {} ::pick {} list global; interp alias {} ::via::pick {} list path; namespace eval caller {namespace path ::via; pick}", "pick", "::list", Some("path")),
                ("namespace eval via {proc local {} {return ordinary_path}}; interp alias {} ::local {} list global_alias; namespace eval caller {namespace path ::via; local}", "local", "::via::local", None),
            ]);
        }
        for (source, head, expected, prefix) in cases {
            let bindings = SourceCommandBindings::analyse(source, config, registry);
            let offset = u32::try_from(source.rfind(head).unwrap()).unwrap();
            let binding = bindings.invocation_at_source(head, offset);
            let target = binding
                .proved_target()
                .expect("original source lookup is closed");
            assert_eq!(target.command, expected, "{version:?}: {source}");
            assert_eq!(
                target
                    .prepended
                    .first()
                    .and_then(|word| word.as_registry_word().literal()),
                prefix
            );
        }
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let result = tcl_test_support::run_script(
            &tclsh.path,
            include_bytes!("data/name-consumers/aliases.tcl"),
        )
        .expect("run C Tcl alias oracle")
        .strict_text()
        .expect("alias oracle succeeds");
        let expected = if version == TclVersion::V8_4 {
            "current\ncurrent\nempty\nordinary"
        } else {
            "current\ncurrent\nempty\nordinary\npath\nordinary_path"
        };
        assert_eq!(result, expected, "{}", tclsh.patchlevel);
    }
}

#[test]
fn path_constants_preserve_written_variable_tails_and_constructed_namespace_segments() {
    let source = r"namespace eval cfg {}
set ::cfg:::dir /colon-run
set ::cfg:: /empty-tail
namespace eval : {
    variable dir /colon-namespace
    variable selected $dir
    namespace eval inner {
        variable dir /nested-colon-namespace
        variable selected $dir
    }
}";
    let authored = HashMap::from([
        ("::cfg::".to_owned(), "/empty-tail".to_owned()),
        (":::::selected".to_owned(), "/colon-namespace".to_owned()),
    ]);
    assert_eq!(
        authored.path_constant("::cfg::").as_deref(),
        Some("/empty-tail")
    );
    assert_eq!(authored.path_constant(":::::selected"), None);
    for version in TclVersion::ALL {
        let dialect = format!("tcl{}", version.version_string());
        let profile = resolve_environment(&dialect).analyser_profile();
        let writes = constant_path_assignments(source, profile);
        let constants = fold_constant_assignments(&writes, None);
        assert_eq!(
            constants.get("::cfg::dir").map(String::as_str),
            Some("/colon-run")
        );
        assert_eq!(
            constants.get("::cfg::").map(String::as_str),
            Some("/empty-tail")
        );
        assert!(!constants.contains_key("::cfg"));
        assert_eq!(
            constants.get(":::::selected").map(String::as_str),
            Some("/colon-namespace")
        );
        assert_eq!(
            constants.get(":::::inner::selected").map(String::as_str),
            Some("/nested-colon-namespace")
        );
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let result = tcl_test_support::run_script(
            &tclsh.path,
            include_bytes!("data/name-consumers/variables.tcl"),
        )
        .expect("run C Tcl variable oracle")
        .strict_text()
        .expect("variable oracle succeeds");
        assert_eq!(
            result, "/colon-run\n/empty-tail\n0\n/colon-namespace\n/nested-colon-namespace\n0",
            "{}",
            tclsh.patchlevel
        );
    }
}

#[test]
fn unknown_scope_aliases_preserve_prior_values_and_withdraw_imports() {
    let source = "set dir /a\nupvar 1 elsewhere dir\n";
    let profile = resolve_environment("tcl8.6").analyser_profile();
    let writes = constant_path_assignments(source, profile);
    let constants = fold_constant_assignments(&writes, None);
    assert_eq!(constants.lookup_at("dir", 10).as_deref(), Some("/a"));
    assert_eq!(
        constants.lookup_at("dir", u32::try_from(source.len()).unwrap()),
        None
    );
    let next = constant_path_assignments("set inherited $dir", profile);
    assert!(fold_constant_assignments_with_imports(&next, None, &constants).is_empty());
}
