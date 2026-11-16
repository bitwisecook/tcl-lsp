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
fn genuine_templated_single_word_body_keeps_the_authored_registration() {
    for source in [
        "speclib demo 1.1 {set body unsafe_command; command one $body}",
        "speclib demo 1.1 {proc define {name body} {command $name $body}; define one unsafe_command}",
    ] {
        let pack = evaluate_pack(source);
        let spec = pack
            .command("one")
            .expect("actual evaluated body is not identified by whitespace");
        // `pure` is a subcommand row; a one-word command body uses a command row.
        assert!(spec.spec.unsafe_command, "{:?}", pack.notices);
    }
}

#[test]
fn evaluated_layout_retains_value_phase_and_refuses_extra_or_unknown_flags() {
    let values = |words: &[&str]| {
        words
            .iter()
            .map(|word| (*word).to_owned())
            .collect::<Vec<_>>()
    };
    for words in [
        vec!["one", "pure"],
        vec!["one", "-override", "pure"],
        vec!["one", ""],
        vec!["one", "-override"],
    ] {
        let args = values(&words);
        assert_eq!(
            EvaluatedCommandBody::select(&args).unwrap().text,
            words[words.len() - 1]
        );
    }
    let data = values(&["one", "-override"]);
    let data = EvaluatedCommandBody::select(&data).unwrap();
    assert_eq!(data.text, "-override");
    assert!(!data.overrides, "body data cannot select an option role");
    let flagged = values(&["one", "-override", "pure"]);
    let flagged = EvaluatedCommandBody::select(&flagged).unwrap();
    assert_eq!(flagged.text, "pure");
    assert!(flagged.overrides);
    for words in [
        vec!["one"],
        vec!["one", "-other", "pure"],
        vec!["one", "-override", "pure", "arity 9"],
    ] {
        assert!(EvaluatedCommandBody::select(&values(&words)).is_none());
    }
}
