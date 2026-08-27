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

//! `fileutil::traverse` — the third of the redesign's named P5 hostile
//! shapes: a `snit::type` factory whose object carries three
//! command-prefix options and a real looping method.
//!
//! Everything here is read from
//! `tmp/tcllib-2.0/modules/fileutil/traverse.tcl` (package
//! `fileutil::traverse 0.7`) and its `traverse.man`:
//!
//! - `snit::type ::fileutil::traverse` with
//!   `constructor {basedir args}` (24, 99) — so the factory is
//!   `::fileutil::traverse ?objectName? basedir ?option value…?`, and
//!   the object command is named positionally.
//! - `option -filter`, `option -prefilter`, `option -errorcmd`, all
//!   `-readonly 1` (95-97).  The first two are invoked as
//!   `uplevel #0 [linsert $options(-filter) end $path]` (307, 318) —
//!   **one** appended word — and `-errorcmd` as
//!   `uplevel #0 [linsert $options(-errorcmd) end $path $msg]` (328) —
//!   **two**.  All three run *during* the traversal driven by `next`,
//!   never from an event loop, so the timing is same-invocation.
//! - `method files {}` (105), `method foreach {fvar body}` (111),
//!   `method next {fvar}` (148).
//!
//! `foreach` is a **genuine loop**, and worth contrasting with
//! `struct::tree walk`: its `switch` on the body's completion code
//! (128-143) maps 3 to a plain `return` (break out of the traversal) and
//! 4 to "next iteration" — ordinary `break`/`continue` semantics with no
//! library-defined extra code.  `walk`'s walker adds code 5
//! (`::struct::tree::prune`), which is exactly the case the model cannot
//! scope to a body.
//!
//! **What is not expressible.**  `-readonly 1` means the three options
//! may be set at construction and never through `$obj configure`;
//! [`OptionSpec`] has no construction-only bit, and the model has no way
//! to say that an option belongs to a *constructor* rather than to the
//! object's `configure` method.  The object's method set is also open —
//! snit installs `configure`, `configurelist`, `cget`, `destroy` and the
//! `Snit_…` internals beside the three documented methods — so the class
//! declares `allow_unknown_methods`.

use crate::prelude::*;

/// The three callback options, with the appended arity each call site
/// proves.
const TRAVERSE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-filter",
        value: OptionValue::command_prefix_n("cmdprefix", AppendedArity::Exactly(1)),
        detail: "Per-path predicate, invoked at level #0 with the path appended; a true result makes the path a result of the traversal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-prefilter",
        value: OptionValue::command_prefix_n("cmdprefix", AppendedArity::Exactly(1)),
        detail: "Per-directory predicate, invoked at level #0 with the path appended; a false result prunes the directory from the descent.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-errorcmd",
        value: OptionValue::command_prefix_n("cmdprefix", AppendedArity::Exactly(2)),
        detail: "Error handler, invoked at level #0 with the path and the message appended; without it traversal errors are thrown.",
        ..OptionSpec::DEFAULT
    },
];

const TRAVERSE_METHODS: &[SubCommand] = &[
    SubCommand {
        name: "files",
        arity: Arity::exact(0),
        detail: "Return the list of every path the traversal yields.",
        synopsis: "traverser files",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foreach",
        arity: Arity::exact(2),
        detail: "Run body once per traversed path, with the path in filevar. An ordinary loop: break stops the traversal, continue skips to the next path.",
        synopsis: "traverser foreach filevar body",
        arg_roles: &[(0, ArgRole::VarWrite), (1, ArgRole::Body)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "next",
        arity: Arity::exact(1),
        detail: "Store the next traversed path in filevar and return true, or return false when the traversal is exhausted.",
        synopsis: "traverser next filevar",
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
];

static TRAVERSE_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "fileutil::traverse",
    instance_methods: TRAVERSE_METHODS,
    superclasses: &[],
    // snit installs `configure`/`configurelist`/`cget`/`destroy` beside
    // the three documented methods, so the set is open.
    allow_unknown_methods: true,
    method_prefix_matching: PrefixMatching::Strict,
};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "::fileutil::traverse ?objectName? basedir ?option value ...?",
    ..FormSpec::DEFAULT
}];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    ..SideEffect::DEFAULT
}];

/// `::fileutil::traverse ?objectName? basedir ?option value …?`
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::traverse",
        arity: Arity::at_least(1),
        // snit's type command names the object at index 0 when a name is
        // given, exactly as `struct::tree`'s creator does.
        creates_instance_at: Some(0),
        object_class: Some(&TRAVERSE_CLASS),
        options: TRAVERSE_OPTIONS,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Create a directory-traversal object rooted at basedir.",
            synopsis: &["::fileutil::traverse ?objectName? basedir ?option value ...?"],
            snippet: "Creates a traverser over the directory hierarchy under *basedir*. The object's `files`, `foreach`, and `next` methods yield the paths found; `-filter` selects which paths are results, `-prefilter` decides which directories are descended into, and `-errorcmd` handles unreadable paths. All three options are read-only after construction.",
            source: "tcllib fileutil::traverse package",
            examples: "",
            return_value: "The name of the new traverser object command.",
        }),
        tcllib_package: Some("fileutil::traverse"),
        required_package: Some("fileutil::traverse"),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three callbacks' appended arities, from `traverse.tcl`'s own
    /// `linsert … end` call sites.
    #[test]
    fn the_callbacks_carry_their_measured_appended_arity() {
        let spec = spec();
        let arity = |name: &str| {
            spec.options
                .iter()
                .find(|option| option.name == name)
                .map(OptionSpec::value_appended_arity)
        };
        assert_eq!(arity("-filter"), Some(AppendedArity::Exactly(1)));
        assert_eq!(arity("-prefilter"), Some(AppendedArity::Exactly(1)));
        assert_eq!(arity("-errorcmd"), Some(AppendedArity::Exactly(2)));
    }

    /// `foreach` is a body-and-loop-variable method, and the class is
    /// reachable from the factory.
    #[test]
    fn the_object_class_carries_the_looping_method() {
        let spec = spec();
        let class = spec.object_class.expect("a traverser class");
        assert_eq!(spec.creates_instance_at, Some(0));
        let method = class.instance_method("foreach").expect("foreach");
        assert_eq!(
            method.arg_roles,
            &[(0, ArgRole::VarWrite), (1, ArgRole::Body)],
        );
        assert!(class.instance_method("files").is_some());
        assert!(class.instance_method("next").is_some());
    }
}
