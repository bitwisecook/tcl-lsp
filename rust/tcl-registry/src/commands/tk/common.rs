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

//! Enum value sets shared across Tk widget options.  These mirror Tk's own
//! `Tk_GetRelief` / `Tk_GetAnchor` / `Tk_GetJustify` converters, whose accepted
//! spellings are uniform across every core widget — so an option carrying one
//! of these sets can mark it `closed` and let W127 flag a value outside it.

use crate::prelude::*;

/// Standard Tk `relief` values (`Tk_GetRelief`).
pub(crate) const RELIEF: &[ArgValue] = &[
    ArgValue {
        value: "flat",
        detail: "No 3-D border.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "groove",
        detail: "Grooved (incised) border.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "raised",
        detail: "Raised 3-D border.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "ridge",
        detail: "Ridged (embossed) border.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "solid",
        detail: "Solid one-pixel border.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "sunken",
        detail: "Sunken 3-D border.",
        ..ArgValue::DEFAULT
    },
];

/// Standard Tk `anchor` positions (`Tk_GetAnchor`).
pub(crate) const ANCHOR: &[ArgValue] = &[
    ArgValue {
        value: "n",
        detail: "North (top center).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "ne",
        detail: "North-east (top right).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "e",
        detail: "East (right center).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "se",
        detail: "South-east (bottom right).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "s",
        detail: "South (bottom center).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "sw",
        detail: "South-west (bottom left).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "w",
        detail: "West (left center).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "nw",
        detail: "North-west (top left).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "center",
        detail: "Centered.",
        ..ArgValue::DEFAULT
    },
];

/// Standard Tk `justify` values (`Tk_GetJustify`).
pub(crate) const JUSTIFY: &[ArgValue] = &[
    ArgValue {
        value: "left",
        detail: "Left-justify lines.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "right",
        detail: "Right-justify lines.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "center",
        detail: "Center lines.",
        ..ArgValue::DEFAULT
    },
];

pub(crate) const TTK_WIDGET_READS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    ..SideEffect::DEFAULT
}];

pub(crate) const TTK_WIDGET_READS_WRITES: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Getter/setter forms for the shared widget `configure` method.
///
/// Zero or one argument only reads the option table; two or more arguments
/// are option/value pairs and mutate it. The setter arity models the legal
/// pair rhythm rather than treating an incomplete trailing option as a
/// concrete mutating form.
pub(crate) const CONFIGURE_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "query",
        arity: Arity::new(0, 1),
        traits: Some(Traits::PURE),
        mutator: Some(false),
        side_effects: Some(TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "set",
        arity: Arity::stepped(2, Arity::UNLIMITED, 2),
        traits: Some(Traits::CONFIGURES_INSTANCE_OPTIONS),
        mutator: Some(true),
        side_effects: Some(TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
];

/// Zero-argument widget query and one-argument mutation forms.
pub(crate) const QUERY_OR_SET_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "query",
        arity: Arity::exact(0),
        traits: Some(Traits::PURE),
        mutator: Some(false),
        side_effects: Some(TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "set",
        arity: Arity::exact(1),
        traits: Some(Traits::empty()),
        mutator: Some(true),
        side_effects: Some(TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
];

/// [`QUERY_OR_SET_FORMS`] with a user-controlled zero-argument result.
pub(crate) const TAINTED_QUERY_OR_SET_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "query",
        arity: Arity::exact(0),
        traits: Some(Traits::PURE.union(Traits::TAINT_SOURCE_ZERO_ARGS)),
        mutator: Some(false),
        side_effects: Some(TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "set",
        arity: Arity::exact(1),
        traits: Some(Traits::empty()),
        mutator: Some(true),
        side_effects: Some(TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
];

/// Standard scroll-view query and mutation forms used by Tk widget `xview`
/// and `yview` methods.
pub(crate) const VIEW_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "query",
        arity: Arity::exact(0),
        traits: Some(Traits::PURE),
        mutator: Some(false),
        side_effects: Some(TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "move",
        arity: Arity::at_least(1),
        traits: Some(Traits::empty()),
        mutator: Some(true),
        side_effects: Some(TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
];

/// The themed entry-family `selection` operation has one read-only form;
/// `clear` and `range` remain conservatively mutating on the parent row.
pub(crate) const ENTRY_SELECTION_FORMS: &[SubCommandForm] = &[SubCommandForm {
    name: "present",
    arity: Arity::exact(1),
    literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["present"])),
    traits: Some(Traits::PURE),
    mutator: Some(false),
    side_effects: Some(TTK_WIDGET_READS),
    ..SubCommandForm::DEFAULT
}];

/// State query plus an optional arbitrary Tcl body. The widget-state read is
/// known, while the body can observe or mutate any state the interpreter can
/// reach; `Unknown` preserves that barrier instead of presenting the callback
/// form as a harmless getter.
pub(crate) const TTK_INSTATE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::InterpState,
        reads: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        target: SideEffectTarget::Unknown,
        reads: true,
        writes: true,
        ..SideEffect::DEFAULT
    },
];

/// A widget mutation that can synchronously evaluate an arbitrary configured
/// Tcl callback (`invoke`, validation, toggle). The interpreter-state effect
/// records the widget/variable transition; `Unknown` retains the callback's
/// unrestricted reads and writes.
pub(crate) const TTK_CALLBACK_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::InterpState,
        reads: true,
        writes: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        target: SideEffectTarget::Unknown,
        reads: true,
        writes: true,
        ..SideEffect::DEFAULT
    },
];

/// Instance operations shared by every classic Tk widget command.
///
/// Widget-specific methods remain on their own class descriptors; this base
/// only states the two operations implemented by the classic widget core.
pub(crate) const CLASSIC_WIDGET_CGET: SubCommand = SubCommand {
    name: "cget",
    arity: Arity::exact(1),
    detail: "Return the current value of a widget option.",
    synopsis: "pathName cget option",
    pure: true,
    return_type: Some(TclType::String),
    side_effects: TTK_WIDGET_READS,
    ..SubCommand::DEFAULT
};

pub(crate) const CLASSIC_WIDGET_CONFIGURE: SubCommand = SubCommand {
    name: "configure",
    arity: Arity::at_least(0),
    detail: "Query or change widget options.",
    synopsis: "pathName configure ?option? ?value option value ...?",
    return_type: Some(TclType::String),
    subcommand_forms: CONFIGURE_FORMS,
    ..SubCommand::DEFAULT
};

pub(crate) static CLASSIC_WIDGET_METHODS: &[SubCommand] =
    &[CLASSIC_WIDGET_CGET, CLASSIC_WIDGET_CONFIGURE];

/// Declare a classic-widget class using the shared `cget`/`configure` base.
macro_rules! classic_widget_class {
    ($class:ident, $name:literal $(,)?) => {
        static $class: ObjectClassSpec = ObjectClassSpec {
            class_name: $name,
            instance_methods: $crate::commands::tk::common::CLASSIC_WIDGET_METHODS,
            superclasses: &[],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Enabled,
        };
    };
}

pub(crate) use classic_widget_class;

/// Declare the standard themed-widget instance API plus any widget-specific
/// methods.  Tk implements these six operations in the shared Ttk widget core;
/// keeping the table in one registry macro prevents individual widget specs
/// from drifting or forcing consumers to know which methods are inherited.
macro_rules! ttk_widget_class {
    ($methods:ident, $class:ident, $name:literal, style_since = $style_since:literal $(, $extra:expr)* $(,)?) => {
        $crate::commands::tk::common::ttk_widget_class!(
            @impl $methods, $class, $name, Lifecycle::introduced_in($style_since)
            $(, $extra)*
        );
    };
    ($methods:ident, $class:ident, $name:literal $(, $extra:expr)* $(,)?) => {
        $crate::commands::tk::common::ttk_widget_class!(
            @impl $methods, $class, $name, Lifecycle::introduced_in("8.7")
            $(, $extra)*
        );
    };
    (@impl $methods:ident, $class:ident, $name:literal, $style_lifecycle:expr $(, $extra:expr)* $(,)?) => {
        static $methods: &[SubCommand] = &[
            SubCommand {
                name: "cget",
                arity: Arity::exact(1),
                detail: "Return the current value of a widget option.",
                synopsis: "pathName cget option",
                pure: true,
                return_type: Some(TclType::String),
                side_effects: $crate::commands::tk::common::TTK_WIDGET_READS,
                ..SubCommand::DEFAULT
            },
            SubCommand {
                name: "configure",
                arity: Arity::at_least(0),
                detail: "Query or change widget options.",
                synopsis: "pathName configure ?option? ?value option value ...?",
                return_type: Some(TclType::String),
                subcommand_forms: $crate::commands::tk::common::CONFIGURE_FORMS,
                ..SubCommand::DEFAULT
            },
            SubCommand {
                name: "identify",
                arity: Arity::exact(3),
                detail: "Identify a themed component at the given coordinates.",
                synopsis: "pathName identify element x y",
                pure: true,
                return_type: Some(TclType::String),
                side_effects: $crate::commands::tk::common::TTK_WIDGET_READS,
                ..SubCommand::DEFAULT
            },
            SubCommand {
                name: "instate",
                arity: Arity::new(1, 2),
                detail: "Test widget state and optionally run a script.",
                synopsis: "pathName instate statespec ?script?",
                arg_roles: &[(1, ArgRole::Body)],
                traits: Traits::EVALUATES_CODE,
                side_effects: $crate::commands::tk::common::TTK_INSTATE_EFFECTS,
                ..SubCommand::DEFAULT
            },
            SubCommand {
                name: "state",
                arity: Arity::new(0, 1),
                detail: "Query or change widget state.",
                synopsis: "pathName state ?stateSpec?",
                return_type: Some(TclType::List),
                subcommand_forms: $crate::commands::tk::common::QUERY_OR_SET_FORMS,
                ..SubCommand::DEFAULT
            },
            SubCommand {
                name: "style",
                arity: Arity::exact(0),
                lifecycle: $style_lifecycle,
                detail: "Return the widget's current style.",
                synopsis: "pathName style",
                pure: true,
                return_type: Some(TclType::String),
                side_effects: $crate::commands::tk::common::TTK_WIDGET_READS,
                ..SubCommand::DEFAULT
            },
            $($extra),*
        ];

        static $class: ObjectClassSpec = ObjectClassSpec {
            class_name: $name,
            instance_methods: $methods,
            superclasses: &[],
            allow_unknown_methods: false,
            method_prefix_matching: PrefixMatching::Enabled,
        };
    };
}

pub(crate) use ttk_widget_class;
