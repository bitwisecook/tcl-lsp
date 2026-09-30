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

//! Unit tests for the tool-environment notice: its text, its dismissal markers,
//! and which environments earn it.

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
fn text_and_description_read_the_same_release_and_packages() {
    for definition in tcl_registry::model::selectable_environments()
        .into_iter()
        .filter(|definition| qualifies(definition))
    {
        let release = definition.core_label().expect("a tool shell has a core");
        let packages: Vec<&str> = definition.ambient_packages().collect();
        let (last, init) = packages.split_last().expect("at least one package");
        let id = definition.id.as_str();
        let expected = format!(
            "{} ({id}) is {release} plus the {} and {last} packages.",
            definition.display_name,
            init.join(", ")
        );
        assert!(
            notice_text(&definition).starts_with(&expected),
            "{id}: {}",
            notice_text(&definition)
        );
        assert_eq!(
            definition.description(),
            format!(
                "{} — {release} + {}",
                definition.display_name,
                packages.join(", ")
            ),
            "{id}"
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

/// The marker file names in `dir`, sorted.
fn marker_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("list the marker directory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn a_dismissal_is_one_empty_marker_named_for_the_environment() {
    let dir = scratch_dir("marker");
    let markers = dir.join("notices").join("environment-kind");
    let notice = EnvironmentNotice::new(Some(markers.clone()), Vec::new());
    notice
        .persist("xilinx-eda-tcl")
        .expect("record the dismissal");
    assert_eq!(marker_names(&markers), ["xilinx-eda-tcl"]);
    let marker = markers.join("xilinx-eda-tcl");
    assert!(marker.is_file());
    assert_eq!(std::fs::metadata(&marker).expect("metadata").len(), 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_dismissal_is_recorded_and_read_back_at_start_up() {
    let dir = scratch_dir("write-back");
    let notices = dir.join("state").join("notices");
    let notice = EnvironmentNotice::new(
        Some(notices.join("environment-kind")),
        ids(&["xilinx-eda-tcl"]),
    );
    notice
        .persist("xilinx-eda-tcl")
        .expect("record the dismissal");

    let reloaded = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(notices));
    assert!(reloaded.take_load_warning().is_none());
    assert!(!reloaded.wants(&environment("xilinx-eda-tcl")));
    assert!(reloaded.wants(&environment("synopsys-eda-tcl")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_existing_marker_means_already_dismissed() {
    let dir = scratch_dir("already");
    let markers = dir.join("environment-kind");
    let notice = EnvironmentNotice::new(Some(markers.clone()), Vec::new());
    notice.persist("xilinx-eda-tcl").expect("first dismissal");
    notice
        .persist("xilinx-eda-tcl")
        .expect("a second dismissal of the same environment is success");
    assert_eq!(marker_names(&markers), ["xilinx-eda-tcl"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn two_sessions_dismissing_different_environments_both_stick() {
    let dir = scratch_dir("two-writers");
    let markers = dir.join("environment-kind");
    EnvironmentNotice::new(Some(markers.clone()), Vec::new())
        .persist("xilinx-eda-tcl")
        .expect("first session records");
    EnvironmentNotice::new(Some(markers.clone()), Vec::new())
        .persist("synopsys-eda-tcl")
        .expect("second session records");
    assert_eq!(
        marker_names(&markers),
        ["synopsys-eda-tcl", "xilinx-eda-tcl"]
    );
    let reloaded = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(dir.clone()));
    assert!(!reloaded.wants(&environment("xilinx-eda-tcl")));
    assert!(!reloaded.wants(&environment("synopsys-eda-tcl")));
    assert!(reloaded.wants(&environment("mentor-eda-tcl")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn simultaneous_dismissals_all_stick() {
    let dir = scratch_dir("simultaneous");
    let markers = dir.join("environment-kind");
    let wanted = [
        "cadence-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "microchip-libero-eda-tcl",
        "synopsys-eda-tcl",
        "xilinx-eda-tcl",
    ];
    let start = std::sync::Barrier::new(wanted.len() * 2);
    std::thread::scope(|scope| {
        // Every environment is dismissed by two servers at once.
        for id in wanted.into_iter().chain(wanted) {
            let (markers, start) = (&markers, &start);
            scope.spawn(move || {
                start.wait();
                record_dismissal(markers, id).expect("record the dismissal");
            });
        }
    });
    assert_eq!(marker_names(&markers), wanted);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_id_that_is_not_a_plain_file_name_is_refused() {
    let dir = scratch_dir("unsafe-name");
    let markers = dir.join("environment-kind");
    for id in [
        "",
        ".hidden",
        "../escape",
        "a/b",
        r"a\b",
        "with space",
        "tcl:8.6",
    ] {
        let error = record_dismissal(&markers, id).expect_err(id);
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{id:?}");
    }
    assert!(!markers.exists(), "nothing was created");
    assert!(!dir.join("escape").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn only_files_in_the_marker_directory_are_dismissals() {
    let dir = scratch_dir("only-files");
    let markers = dir.join("environment-kind");
    std::fs::create_dir_all(markers.join("mentor-eda-tcl")).expect("a directory, not a marker");
    std::fs::write(markers.join("xilinx-eda-tcl"), "").expect("a marker");
    let notice = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(dir.clone()));
    assert!(notice.take_load_warning().is_none());
    assert!(!notice.wants(&environment("xilinx-eda-tcl")));
    assert!(notice.wants(&environment("mentor-eda-tcl")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_missing_marker_directory_is_the_ordinary_first_run() {
    let dir = scratch_dir("missing");
    let notice = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(dir.join("notices")));
    assert!(notice.take_load_warning().is_none());
    assert!(notice.wants(&environment("xilinx-eda-tcl")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unreadable_marker_directory_warns_once_and_dismisses_nothing() {
    let dir = scratch_dir("unreadable");
    let notices = dir.join("notices");
    std::fs::create_dir_all(&notices).expect("create the notices directory");
    // A file where the directory belongs cannot be listed, whoever runs the test.
    std::fs::write(notices.join("environment-kind"), "").expect("write a file in its place");
    let notice = EnvironmentNotice::load(&crate::vfs::NativeStore, Some(notices));
    assert!(notice.wants(&environment("xilinx-eda-tcl")));
    let warning = notice.take_load_warning().expect("a warning");
    assert!(warning.contains("environment-kind"), "{warning}");
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
    notice
        .persist("xilinx-eda-tcl")
        .expect("nothing to write is not a failure");
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
fn the_markers_live_under_the_xdg_state_home() {
    let dir = scratch_dir("xdg-state");
    let notices = dir.join("tcl-lsp").join("notices");
    temp_env::with_var("XDG_STATE_HOME", Some(&dir), || {
        assert_eq!(
            tcl_lsp_core::tcl_install::user_notices_dir(),
            Some(notices.clone())
        );
    });
    let notice = EnvironmentNotice::new(Some(notices.join("environment-kind")), Vec::new());
    notice
        .persist("xilinx-eda-tcl")
        .expect("record the dismissal");
    assert!(
        notices
            .join("environment-kind")
            .join("xilinx-eda-tcl")
            .is_file()
    );
    let _ = std::fs::remove_dir_all(dir);
}
