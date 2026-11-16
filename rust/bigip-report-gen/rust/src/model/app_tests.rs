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

#[test]
fn folder_helpers_split_paths() {
    assert_eq!(folder_of("/Common/appA/vs_a"), "/Common/appA");
    assert_eq!(folder_of("/Common/webstore.app/vs"), "/Common/webstore.app");
    // partition-root object has no folder
    assert_eq!(folder_of("/Common/root_vs"), "");
    assert_eq!(folder_of("not-a-path"), "");
    assert_eq!(
        app_folder_of("/Common/appA/pool_a"),
        Some(("Common".to_owned(), "appA".to_owned()))
    );
    // deeper nesting still groups under the first sub-folder
    assert_eq!(
        app_folder_of("/Common/appA/sub/x"),
        Some(("Common".to_owned(), "appA".to_owned()))
    );
    // root object is not grouped
    assert_eq!(app_folder_of("/Common/root_vs"), None);
}

#[test]
fn build_apps_groups_by_folder_and_skips_root() {
    let mut device = Map::new();
    let mk = |name: &str, fp: &str| {
        let mut o = Map::new();
        o.insert("name".into(), J::String(name.into()));
        o.insert("fullPath".into(), J::String(fp.into()));
        J::Object(o)
    };
    device.insert(
        "virtuals".into(),
        J::Array(vec![
            mk("root_vs", "/Common/root_vs"),
            mk("vs_a", "/Common/appA/vs_a"),
            mk("web_vs", "/Common/store.app/web_vs"),
        ]),
    );
    device.insert(
        "pools".into(),
        J::Array(vec![mk("pool_a", "/Common/appA/pool_a")]),
    );

    let J::Array(apps) = build_apps(&device) else {
        panic!("expected array");
    };
    // appA (folder) + store (iapp) — the root vs is NOT an app.
    assert_eq!(apps.len(), 2);
    let a = apps[0].as_object().unwrap();
    assert_eq!(bstr(a, "name"), "appA");
    assert_eq!(bstr(a, "source"), "folder");
    assert_eq!(a.get("memberCount").and_then(J::as_u64), Some(2));
    let b = apps[1].as_object().unwrap();
    assert_eq!(bstr(b, "name"), "store"); // ".app" stripped
    assert_eq!(bstr(b, "source"), "iapp");
}

#[test]
fn rule_events_use_top_level_rooted_normalised_owner() {
    let mut fields = Map::new();
    fields.insert("name".into(), J::String("r".into()));
    fields.insert("full-path".into(), J::String("/Common/r".into()));
    fields.insert(
        "body".into(),
        J::String("::when http_request { if {1} { :::when client_data {} } }".into()),
    );
    let shaped = shape_rule(&fields, &HashMap::new());
    let mut events: Vec<&str> = shaped["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(J::as_str)
        .collect();
    events.sort_unstable();
    assert_eq!(events, ["HTTP_REQUEST"]);
}

#[test]
fn rule_events_ignore_when_text_stored_as_data() {
    let mut fields = Map::new();
    fields.insert("name".into(), J::String("r".into()));
    fields.insert("full-path".into(), J::String("/Common/r".into()));
    fields.insert(
            "body".into(),
            J::String("set payload {when CLIENT_DATA {}}\nset q \"when SERVER_DATA {}\"\nwhen HTTP_REQUEST {}".into()),
        );
    let shaped = shape_rule(&fields, &HashMap::new());
    assert_eq!(
        shaped["events"],
        J::Array(vec![J::String("HTTP_REQUEST".into())])
    );
}
