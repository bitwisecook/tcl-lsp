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

//! Classic Tk widget option tables against the Tk C sources.
//!
//! `tests/data/tk_option_tables/tk<release>.txt` is extracted from each
//! release's `Tk_OptionSpec` / `Tk_ConfigSpec` tables by
//! `scripts/dev/gen-tk-option-tables.py`.

use tcl_registry::CommandRegistry;

const RELEASES: &[(&str, &str, &str)] = &[
    (
        "tcl8.4",
        "8.4",
        include_str!("../data/tk_option_tables/tk8.4.20.txt"),
    ),
    (
        "tcl8.5",
        "8.5",
        include_str!("../data/tk_option_tables/tk8.5.19.txt"),
    ),
    (
        "tcl8.6",
        "8.6",
        include_str!("../data/tk_option_tables/tk8.6.18.txt"),
    ),
    (
        "tcl9.0",
        "9.0",
        include_str!("../data/tk_option_tables/tk9.0.4.txt"),
    ),
    (
        "tcl9.1",
        "9.1",
        include_str!("../data/tk_option_tables/tk9.1.0.txt"),
    ),
];

#[test]
fn tk_option_tables_match_the_c_sources() {
    let mut report = Vec::new();
    for &(tcl, tk, table) in RELEASES {
        let mut registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find(tcl).expect("Tcl release"));
        registry.insert_ambient_package("Tk", tk);
        for line in table.lines() {
            let mut words = line.split_whitespace();
            let widget = words.next().expect("widget");
            let c_options: Vec<&str> = words.collect();
            let spec = registry
                .get(widget)
                .unwrap_or_else(|| panic!("Tk {tk} {widget} is registered"));
            let missing: Vec<&str> = c_options
                .iter()
                .copied()
                .filter(|option| spec.find_option(option, None, Some(tk)).is_none())
                .collect();
            let extra: Vec<&str> = spec
                .options
                .iter()
                .filter(|option| spec.find_option(option.name, None, Some(tk)).is_some())
                .map(|option| option.name)
                .filter(|name| !c_options.contains(name))
                .collect();
            if !missing.is_empty() || !extra.is_empty() {
                report.push(format!(
                    "Tk {tk} {widget}: missing {missing:?} extra {extra:?}"
                ));
            }
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}

/// `ttkPanedwindow.c` keeps `-weight` in `PaneOptionSpecs`: it is set per
/// pane through `add`, `insert` and `pane` (Tk 8.5 to 9.1), and the widget
/// itself rejects it (`unknown option "-weight"` on Tk 9.0.3).
#[test]
fn ttk_panedwindow_weight_is_a_pane_option() {
    for &(tcl, tk, _) in RELEASES.iter().skip(1) {
        let mut registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find(tcl).expect("Tcl release"));
        registry.insert_ambient_package("Tk", tk);
        let spec = registry.get("ttk::panedwindow").expect("ttk::panedwindow");
        assert!(
            spec.find_option("-weight", None, Some(tk)).is_none(),
            "Tk {tk}"
        );
        for (method, prefix) in [("add", 1), ("insert", 2), ("pane", 1)] {
            let sub = spec.resolve_subcommand(method).expect("pane method");
            assert!(
                sub.options.iter().any(|option| option.name == "-weight"),
                "Tk {tk} {method}"
            );
            assert_eq!(sub.option_prefix_words, prefix, "Tk {tk} {method}");
        }
    }
}

const METHOD_RELEASES: &[(&str, &str, &str)] = &[
    (
        "tcl8.5",
        "8.5",
        include_str!("../data/tk_option_tables/tk8.5.19.methods.txt"),
    ),
    (
        "tcl8.6",
        "8.6",
        include_str!("../data/tk_option_tables/tk8.6.18.methods.txt"),
    ),
    (
        "tcl9.0",
        "9.0",
        include_str!("../data/tk_option_tables/tk9.0.4.methods.txt"),
    ),
    (
        "tcl9.1",
        "9.1",
        include_str!("../data/tk_option_tables/tk9.1.0.methods.txt"),
    ),
];

/// Each themed widget's instance methods are exactly the `Ttk_Ensemble`
/// its `WidgetSpec` registers, in Tk 8.5 to 9.1.
#[test]
fn ttk_widget_methods_match_the_c_ensembles() {
    let mut report = Vec::new();
    for &(tcl, tk, table) in METHOD_RELEASES {
        let mut registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find(tcl).expect("Tcl release"));
        registry.insert_ambient_package("Tk", tk);
        for line in table.lines() {
            let mut words = line.split_whitespace();
            let widget = words.next().expect("widget");
            let c_methods: Vec<&str> = words.collect();
            let class = registry
                .get(widget)
                .and_then(|spec| spec.object_class)
                .unwrap_or_else(|| panic!("Tk {tk} {widget} has an instance class"));
            let declared: Vec<&str> = class
                .instance_methods
                .iter()
                .filter(|method| method.lifecycle.available_at(Some(tk)))
                .map(|method| method.name)
                .collect();
            let missing: Vec<&str> = c_methods
                .iter()
                .copied()
                .filter(|name| !declared.contains(name))
                .collect();
            let extra: Vec<&str> = declared
                .iter()
                .copied()
                .filter(|name| !c_methods.contains(name))
                .collect();
            if !missing.is_empty() || !extra.is_empty() {
                report.push(format!(
                    "Tk {tk} {widget}: missing {missing:?} extra {extra:?}"
                ));
            }
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}

const CLASSIC_METHOD_RELEASES: &[(&str, &str, &str)] = &[
    (
        "tcl8.4",
        "8.4",
        include_str!("../data/tk_option_tables/tk8.4.20.classic-methods.txt"),
    ),
    (
        "tcl8.5",
        "8.5",
        include_str!("../data/tk_option_tables/tk8.5.19.classic-methods.txt"),
    ),
    (
        "tcl8.6",
        "8.6",
        include_str!("../data/tk_option_tables/tk8.6.18.classic-methods.txt"),
    ),
    (
        "tcl9.0",
        "9.0",
        include_str!("../data/tk_option_tables/tk9.0.4.classic-methods.txt"),
    ),
    (
        "tcl9.1",
        "9.1",
        include_str!("../data/tk_option_tables/tk9.1.0.classic-methods.txt"),
    ),
];

/// Each classic widget's instance methods are exactly those its command
/// lists for an unknown method under that release's `wish`, Tk 8.4.20 to
/// 9.1.0 (`scripts/dev/tk_classic_methods.tcl`; issue #1710). The widget's
/// own list already applies the per-class filtering a shared C table such
/// as tkButton.c's hides.
#[test]
fn classic_widget_methods_match_each_releases_wish() {
    let mut report = Vec::new();
    for &(tcl, tk, table) in CLASSIC_METHOD_RELEASES {
        let mut registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find(tcl).expect("Tcl release"));
        registry.insert_ambient_package("Tk", tk);
        for line in table.lines().filter(|line| !line.starts_with('#')) {
            let mut words = line.split_whitespace();
            let widget = words.next().expect("widget");
            let wish_methods: Vec<&str> = words.collect();
            let Some(class) = registry.get(widget).and_then(|spec| spec.object_class) else {
                report.push(format!("Tk {tk} {widget}: no instance class"));
                continue;
            };
            let declared: Vec<&str> = class
                .instance_methods
                .iter()
                .filter(|method| method.lifecycle.available_at(Some(tk)))
                .map(|method| method.name)
                .collect();
            let missing: Vec<&str> = wish_methods
                .iter()
                .copied()
                .filter(|name| !declared.contains(name))
                .collect();
            let extra: Vec<&str> = declared
                .iter()
                .copied()
                .filter(|name| !wish_methods.contains(name))
                .collect();
            if !missing.is_empty() || !extra.is_empty() {
                report.push(format!(
                    "Tk {tk} {widget}: missing {missing:?} extra {extra:?}"
                ));
            }
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}
