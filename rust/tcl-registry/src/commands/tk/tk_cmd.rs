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

//! `tk` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "accessible",
        arity: Arity::at_least(1),
        detail: "Access Tk's screen-reader accessibility operations.",
        synopsis: "tk accessible subcommand ?arg ...?",
        // First present in the official Tk 9.1 command surface
        // (`core-9-1-a1` / doc/accessible.n). The nested operation table is
        // intentionally left opaque here: this descriptor records the exact
        // top-level ensemble shape without inventing shared semantics for its
        // platform-backed operations.
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "appname",
        arity: Arity::new(0, 1),
        detail: "Query or set the application name for send commands.",
        synopsis: "tk appname ?newName?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "attribtable",
        arity: Arity::exact(1),
        detail: "Create a command that manages attributes attached to widgets.",
        synopsis: "tk attribtable tableName",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "busy",
        arity: Arity::at_least(1),
        detail: "Make a window appear busy (greyed out with a busy cursor).",
        synopsis: "tk busy subcommand ?arg ...?",
        lifecycle: Lifecycle::introduced_in("8.6"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "caret",
        arity: Arity::at_least(1),
        detail: "Query or set the caret (text cursor) position for accessibility.",
        synopsis: "tk caret window ?-x x? ?-y y? ?-height height?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "fileicon",
        arity: Arity::exact(2),
        detail: "Return a platform-native icon for a file at the requested size.",
        synopsis: "tk fileicon file size",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "fontchooser",
        arity: Arity::at_least(1),
        detail: "Control the platform font selection dialogue.",
        synopsis: "tk fontchooser subcommand ?arg ...?",
        lifecycle: Lifecycle::introduced_in("8.6"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inactive",
        arity: Arity::at_least(0),
        detail: "Query or reset the user inactivity timer in milliseconds.",
        synopsis: "tk inactive ?-displayof window? ?reset?",
        lifecycle: Lifecycle::introduced_in("8.5"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "print",
        arity: Arity::exact(1),
        detail: "Open the platform print workflow for a canvas or text widget.",
        synopsis: "tk print window",
        lifecycle: Lifecycle::introduced_in("9.0"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scaling",
        arity: Arity::at_least(0),
        detail: "Query or set the number of pixels per point on the display.",
        synopsis: "tk scaling ?-displayof window? ?number?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sysnotify",
        arity: Arity::exact(2),
        detail: "Post a platform-specific system notification.",
        synopsis: "tk sysnotify title message",
        // Added on the Tk 8.7 development line (`core-8-7-a5`) and retained
        // by every Tk 9.0 release. Tk lifecycle data already uses the 8.7
        // package floor for features from that development line.
        lifecycle: Lifecycle::introduced_in("8.7"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "systray",
        arity: Arity::at_least(1),
        detail: "Create, configure, query, or destroy the platform system-tray icon.",
        synopsis: "tk systray subcommand ?arg ...?",
        lifecycle: Lifecycle::introduced_in("8.7"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "useinputmethods",
        arity: Arity::at_least(0),
        detail: "Query or set whether Tk should use XIM input methods.",
        synopsis: "tk useinputmethods ?-displayof window? ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "windowingsystem",
        arity: Arity::exact(0),
        detail: "Return the windowing system in use: x11, win32, or aqua.",
        synopsis: "tk windowingsystem",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "tk subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manipulate Tk internal state.",
            // Individual subcommand synopses carry their own lifecycle. Keep
            // the command-level line generic so an 8.4 hover cannot advertise
            // a 9.1-only form merely because HoverSnippet has no lifecycle
            // column of its own.
            synopsis: &["tk subcommand ?arg ...?"],
            snippet: "Provides access to miscellaneous Tk internal state and the windowing system.",
            source: "Tk man page tk.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subcommands_for_profile(name: &str) -> Vec<&'static str> {
        let profile = crate::model::ingress::resolve_environment(name).analyser_profile();
        let floor = profile
            .library_floor_default("Tk")
            .unwrap_or_else(|| panic!("{name} must pin Tk"));
        spec()
            .subcommand_table(Some(profile.surface_query()), Some(floor), None)
            .names()
            .collect()
    }

    #[test]
    fn tk_subcommands_follow_the_official_release_profiles() {
        // doc/tk.n at the corresponding upstream release tags:
        // core-8-4-20, core-8-5-19, core-8-6-16, core-9-0-4,
        // and core-9-1-b0.
        let common = [
            "appname",
            "caret",
            "scaling",
            "useinputmethods",
            "windowingsystem",
        ];
        let expected = [
            ("tcl8.4", common.to_vec()),
            ("tcl8.5", [common.as_slice(), &["inactive"]].concat()),
            (
                "tcl8.6",
                [common.as_slice(), &["busy", "fontchooser", "inactive"]].concat(),
            ),
            (
                "tcl9.0",
                [
                    common.as_slice(),
                    &[
                        "busy",
                        "fontchooser",
                        "inactive",
                        "print",
                        "sysnotify",
                        "systray",
                    ],
                ]
                .concat(),
            ),
            (
                "tcl9.1",
                [
                    common.as_slice(),
                    &[
                        "accessible",
                        "attribtable",
                        "busy",
                        "fileicon",
                        "fontchooser",
                        "inactive",
                        "print",
                        "sysnotify",
                        "systray",
                    ],
                ]
                .concat(),
            ),
        ];

        for (profile, mut want) in expected {
            let mut got = subcommands_for_profile(profile);
            got.sort_unstable();
            want.sort_unstable();
            assert_eq!(got, want, "tk subcommands under {profile}");
        }
    }

    #[test]
    fn newer_tk_forms_keep_their_documented_outer_arities() {
        let spec = spec();
        for (name, arity) in [
            ("attribtable", Arity::exact(1)),
            ("fileicon", Arity::exact(2)),
            ("print", Arity::exact(1)),
            ("sysnotify", Arity::exact(2)),
        ] {
            assert_eq!(spec.subcommand(name).expect(name).arity, arity, "tk {name}");
        }
        for name in ["accessible", "systray"] {
            assert_eq!(
                spec.subcommand(name).expect(name).arity,
                Arity::at_least(1),
                "tk {name} remains an accurately bounded opaque ensemble"
            );
        }
    }
}
