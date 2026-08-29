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

//! `option` — one row of a command's / subcommand's option table.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The option row's own flags.
///
/// `-takes HINT` is what makes an option a *value* option; without it the
/// option is a flag. The value's own fields and the option's own fields are
/// flags on the same row, which is what keeps `options` a singular row
/// statement rather than a nested block.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-takes",
        value: OptionValue::value("hint"),
        detail: "the option takes a value; the word is its completion hint",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-arity",
        value: OptionValue::value("{Fixed N}"),
        detail: "static value-word count (`One` is the default)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-arity-hook",
        value: OptionValue::Takes(OptionArg {
            arity: OptionArity::Fixed(2),
            role: ArgRole::ParamList,
            hint: "{words ctx} { … }",
            ..OptionArg::DEFAULT
        }),
        detail: "dynamic value-word count: a `{words ctx}` body, or `-native ID`",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-role",
        value: OptionValue::value("role"),
        detail: "the value word's ArgRole",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-also-role",
        value: OptionValue::value("role"),
        detail: "a second role at the same value position (two-way bindings)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-body-kind",
        value: OptionValue::value("Plain|Structural"),
        detail: "for a Body value, whether it runs in the caller's frame",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-script-timing",
        value: OptionValue::value("SameInvocation|Deferred|ReferenceOnly"),
        detail: "for an executable value, whether it runs now, is stored for later, or is only identified",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value("values"),
        detail: "inline enumerable value set",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values-from",
        value: OptionValue::value("table"),
        detail: "name a pack-level `values` table",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-closed",
        detail: "the value set is exhaustive",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-taints-var-write",
        detail: "the named variable is writable by external/user input",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::value("{Range LO HI}"),
        detail: "numeric domain accepted alongside `-values` (`max`/`min` sentinels)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-appends",
        value: OptionValue::value("{Exactly N}"),
        detail: "callback arity for a CommandPrefix value",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-detail",
        value: OptionValue::value("prose"),
        detail: "the option's one-line description",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-aliases",
        value: OptionValue::value("names"),
        detail: "alternate spellings the option table itself recognises",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-dialects",
        value: OptionValue::value("set"),
        detail: "per-option dialect gate",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-min-abbrev",
        value: OptionValue::value("n"),
        detail: "documented minimum abbreviation length",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-introduced",
        value: OptionValue::value("version"),
        detail: "Lifecycle.introduced (the owning package's version axis)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-deprecated",
        value: OptionValue::value("version"),
        detail: "Lifecycle.deprecated",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-retired",
        value: OptionValue::value("version"),
        detail: "Lifecycle.retired",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-deprecation-fix",
        value: OptionValue::value("fix"),
        detail: "Lifecycle.deprecation_fix",
        ..OptionSpec::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "option -name ?-takes hint? ?flags…?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "option",
        traits: Traits::LANGUAGE_KEYWORD,
        surface: Some(SpecSurface::SPECTCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Declare one option of the enclosing command or subcommand.",
            synopsis: &["option -name ?-takes hint? ?flags…?"],
            snippet: "`-takes HINT` is what makes an option a value option; without it the option is a flag. `-arity` takes the static shapes only — the dynamic variant is `-arity-hook`, which is spelled separately because a hook is *two* words (a parameter list and a body) where every other flag value is one.",
            source: "SpecTcl (docs/design/spec-packs.md)",
            examples: "option -stride -takes strideLength -integer {Range 2 max} -dialects tcl8.6+ -detail {Treat list as consecutive groups of strideLength elements.}",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        arg_roles: &[(0, ArgRole::Option)],
        ..CommandSpec::DEFAULT
    }
}
