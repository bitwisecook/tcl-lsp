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

//! Unit tests for the tool-environment notice: its text, its state file, and
//! which environments earn it.

use super::*;

fn environment(name: &str) -> Arc<EnvironmentDefinition> {
    tcl_registry::model::resolve_known_environment(name)
        .unwrap_or_else(|| panic!("`{name}` names an environment"))
        .definition
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tcl-lsp-environment-notice-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|id| (*id).to_owned()).collect()
}

#[test]
fn vivado_text_names_the_release_and_every_ambient_package() {
    assert_eq!(
        notice_text(&environment("xilinx-eda-tcl")),
        "Xilinx Vivado (xilinx-eda-tcl) is Tcl 8.5 plus the vivado, sdc and upf packages. \
         Tool support is a set of library packages on a Tcl release, not a separate dialect; \
         your selection keeps working as before."
    );
}

#[test]
fn synopsys_text_reads_its_own_release_and_package_list() {
    assert_eq!(
        notice_text(&environment("synopsys-eda-tcl")),
        "Synopsys DC / PrimeTime / ICC2 / Formality (synopsys-eda-tcl) is Tcl 8.6 plus the \
         synopsys-dc, synopsys-pt, synopsys-icc2, synopsys-fm, synopsys, sdc and upf packages. \
         Tool support is a set of library packages on a Tcl release, not a separate dialect; \
         your selection keeps working as before."
    );
}

#[test]
fn text_lists_no_package_a_definition_lacks() {
    let with_packages = |count: usize| {
        let mut definition = (*environment("xilinx-eda-tcl")).clone();
        definition.expected_packages.truncate(count);
        notice_text(&definition)
    };
    assert!(
        with_packages(1).contains("is Tcl 8.5 plus the vivado package. "),
        "{}",
        with_packages(1)
    );
    assert!(
        with_packages(2).contains("is Tcl 8.5 plus the vivado and sdc packages. "),
        "{}",
        with_packages(2)
    );
    assert!(
        with_packages(0).contains("is Tcl 8.5. Tool support"),
        "{}",
        with_packages(0)
    );
}

#[test]
fn text_agrees_with_the_description_every_picker_shows() {
    for definition in tcl_registry::model::selectable_environments()
        .into_iter()
        .filter(|definition| qualifies(definition))
    {
        let description = definition.description();
        let (_, detail) = description
            .split_once(" — ")
            .expect("a Packages description");
        let (release, packages) = detail.split_once(" + ").expect("a release and packages");
        let packages: Vec<&str> = packages.split(", ").collect();
        let (last, init) = packages.split_last().expect("at least one package");
        let expected = format!(
            "{} ({}) is {release} plus the {} and {last} packages.",
            definition.display_name,
            definition.id.as_str(),
            init.join(", "),
        );
        assert!(
            notice_text(&definition).starts_with(&expected),
            "{}: {}",
            definition.id.as_str(),
            notice_text(&definition)
        );
    }
}

#[test]
fn only_the_six_bundled_tool_shells_qualify() {
    let mut qualifying: Vec<String> = tcl_registry::model::selectable_environments()
        .iter()
        .filter(|definition| qualifies(definition))
        .map(|definition| definition.id.as_str().to_owned())
        .collect();
    qualifying.sort();
    assert_eq!(
        qualifying,
        [
            "cadence-eda-tcl",
            "intel-quartus-eda-tcl",
            "mentor-eda-tcl",
            "microchip-libero-eda-tcl",
            "synopsys-eda-tcl",
            "xilinx-eda-tcl",
        ]
    );
    // `tk` is a tool shell too, but built in; the languages are not shells.
    for name in ["tk", "tcl8.6", "jim", "f5-irules", "bpf", "expect"] {
        assert!(!qualifies(&environment(name)), "{name}");
    }
    // A pack the user or a workspace supplies is theirs to explain.
    let mut own = (*environment("xilinx-eda-tcl")).clone();
    for provenance in [
        Provenance::BuiltIn,
        Provenance::User,
        Provenance::WorkspaceTrusted,
        Provenance::WorkspaceUntrusted,
    ] {
        own.provenance = provenance;
        assert!(!qualifies(&own), "{provenance:?}");
    }
}

#[test]
fn dismissed_ids_read_a_comma_or_whitespace_list() {
    let file = "[dismissed]\nenvironment-kind = xilinx-eda-tcl, synopsys-eda-tcl\n";
    assert_eq!(
        dismissed_ids(&parse_ini(file)),
        ids(&["xilinx-eda-tcl", "synopsys-eda-tcl"])
    );
    let spaced =
        "[dismissed]\nenvironment-kind =\n    xilinx-eda-tcl\n    mentor-eda-tcl xilinx-eda-tcl\n";
    assert_eq!(
        dismissed_ids(&parse_ini(spaced)),
        ids(&["xilinx-eda-tcl", "mentor-eda-tcl"])
    );
}

#[test]
fn an_empty_or_unrelated_file_dismisses_nothing() {
    for file in [
        "",
        "\n\n",
        "[other]\nenvironment-kind = xilinx-eda-tcl\n",
        "[dismissed]\nsomething-else = xilinx-eda-tcl\n",
        "environment-kind = xilinx-eda-tcl\n",
        "not an ini file at all {{{",
    ] {
        assert!(dismissed_ids(&parse_ini(file)).is_empty(), "{file:?}");
    }
}

#[test]
fn the_state_file_round_trips() {
    let listed = ids(&["xilinx-eda-tcl", "synopsys-eda-tcl"]);
    let text = render_notices(Vec::new(), &listed);
    assert_eq!(dismissed_ids(&parse_ini(&text)), listed);
    assert!(
        text.contains("environment-kind = xilinx-eda-tcl, synopsys-eda-tcl\n"),
        "{text}"
    );
    // Rendering what was parsed changes nothing that matters.
    let again = render_notices(parse_ini(&text), &listed);
    assert_eq!(dismissed_ids(&parse_ini(&again)), listed);
    assert_eq!(again, text);
}

#[test]
fn rewriting_keeps_what_another_server_version_recorded() {
    let file = "[dismissed]\n\
                environment-kind = xilinx-eda-tcl\n\
                future-notice = a, b\n\
                \n\
                [seen]\n\
                tour =\n\
                \x20   one\n\
                \x20   two\n";
    let text = render_notices(parse_ini(file), &ids(&["xilinx-eda-tcl", "mentor-eda-tcl"]));
    let sections = parse_ini(&text);
    assert_eq!(
        dismissed_ids(&sections),
        ids(&["xilinx-eda-tcl", "mentor-eda-tcl"])
    );
    let dismissed = sections
        .iter()
        .find(|section| section.name == "dismissed")
        .expect("dismissed section");
    assert!(
        dismissed
            .entries
            .iter()
            .any(|(key, value)| key == "future-notice" && value == "a, b"),
        "{text}"
    );
    let entries_of = |sections: &[Section]| {
        sections
            .iter()
            .find(|section| section.name == "seen")
            .expect("seen section")
            .entries
            .clone()
    };
    assert_eq!(entries_of(&sections), entries_of(&parse_ini(file)));
}

#[test]
fn a_dismissal_is_written_and_read_back_at_start_up() {
    let dir = scratch_dir("write-back");
    let path = dir.join("state").join("notices.ini");
    let notice = EnvironmentNotice::new(Some(path.clone()), ids(&["xilinx-eda-tcl"]));
    notice.persist().expect("write the state file");
    let content = std::fs::read_to_string(&path).expect("state file exists");
    assert!(
        content.contains("environment-kind = xilinx-eda-tcl\n"),
        "{content}"
    );

    let reloaded = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(path));
    assert!(reloaded.take_load_warning().is_none());
    assert!(!reloaded.wants(&environment("xilinx-eda-tcl")));
    assert!(reloaded.wants(&environment("synopsys-eda-tcl")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn two_sessions_dismissing_different_environments_both_stick() {
    let dir = scratch_dir("two-writers");
    let path = dir.join("notices.ini");
    EnvironmentNotice::new(Some(path.clone()), ids(&["xilinx-eda-tcl"]))
        .persist()
        .expect("first session writes");
    EnvironmentNotice::new(Some(path.clone()), ids(&["synopsys-eda-tcl"]))
        .persist()
        .expect("second session writes");
    let content = std::fs::read_to_string(&path).expect("state file exists");
    assert_eq!(
        dismissed_ids(&parse_ini(&content)),
        ids(&["xilinx-eda-tcl", "synopsys-eda-tcl"]),
        "{content}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_missing_state_file_is_the_ordinary_first_run() {
    let dir = scratch_dir("missing");
    let notice = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(dir.join("notices.ini")));
    assert!(notice.take_load_warning().is_none());
    assert!(notice.wants(&environment("xilinx-eda-tcl")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unreadable_state_file_warns_once_and_dismisses_nothing() {
    let dir = scratch_dir("unreadable");
    let path = dir.join("notices.ini");
    std::fs::write(&path, [0xff, 0xfe, 0x00, 0x9f]).expect("write a file that is not UTF-8");
    let notice = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(path));
    assert!(notice.wants(&environment("xilinx-eda-tcl")));
    let warning = notice.take_load_warning().expect("a warning");
    assert!(warning.contains("notices.ini"), "{warning}");
    assert!(
        notice.take_load_warning().is_none(),
        "the warning is given once"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_session_with_no_state_directory_still_dismisses_in_memory() {
    let notice = EnvironmentNotice::load(&crate::vfs::NativeStore, None);
    assert!(notice.take_load_warning().is_none());
    notice.persist().expect("nothing to write is not a failure");
}

#[test]
fn an_environment_is_shown_once_per_session_and_never_after_a_dismissal() {
    let notice = EnvironmentNotice::in_memory();
    let vivado = environment("xilinx-eda-tcl");
    assert!(notice.wants(&vivado));
    assert!(notice.claim("xilinx-eda-tcl"));
    assert!(!notice.wants(&vivado), "shown already");
    assert!(!notice.claim("xilinx-eda-tcl"), "a second claim is refused");
    // Another environment is unaffected.
    assert!(notice.claim("synopsys-eda-tcl"));

    let dismissed = EnvironmentNotice::new(None, ids(&["xilinx-eda-tcl"]));
    assert!(!dismissed.wants(&vivado));
    assert!(!dismissed.claim("xilinx-eda-tcl"));
    assert!(dismissed.claim("mentor-eda-tcl"));
}

#[test]
fn the_setting_defaults_on_and_toggles_live() {
    let notice = EnvironmentNotice::in_memory();
    assert!(notice.enabled());
    notice.set_enabled(false);
    assert!(!notice.enabled());
    notice.set_enabled(true);
    assert!(notice.enabled());
}

#[test]
fn the_state_file_lives_under_the_xdg_state_home() {
    let dir = scratch_dir("xdg-state");
    let expected = dir.join("tcl-lsp").join("notices.ini");
    temp_env::with_var("XDG_STATE_HOME", Some(&dir), || {
        assert_eq!(
            tcl_lsp_core::tcl_install::user_notices_path(),
            Some(expected.clone())
        );
    });
    let _ = std::fs::remove_dir_all(dir);
}
