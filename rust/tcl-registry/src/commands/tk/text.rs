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

//! `text` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const USER_EVENT_INPUTS: &[CallbackTaintInput] = &[CallbackTaintInput::TK_EVENT_CHAR];

/// `text` `tag` sub-subcommand roles. `pathName tag bind tagName sequence script`
/// binds a deferred event-handler script (run from the Tk event loop) as its
/// trailing word. Args here are those AFTER the `tag` subcommand word:
/// `bind`(0) tagName(1) sequence(2) script(3).
fn text_tag_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args
        .first()
        .is_some_and(|name| !name.is_empty() && "bind".starts_with(name))
        && args.len() == 4
    {
        vec![(3, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

fn text_tag_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    if text_tag_arg_roles(args).is_empty() {
        Vec::new()
    } else {
        vec![(3, ScriptTiming::Deferred)]
    }
}

const DUMP_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "Include text, marks, tags, images, and embedded windows.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::command_prefix_n("command", AppendedArity::Exactly(3)),
        detail: "Invoke this command prefix for each element with key, value, and index appended.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-image",
        value: OptionValue::flag(),
        detail: "Include embedded images.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-mark",
        value: OptionValue::flag(),
        detail: "Include marks.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-tag",
        value: OptionValue::flag(),
        detail: "Include tag transitions.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-text",
        value: OptionValue::flag(),
        detail: "Include text segments.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-window",
        value: OptionValue::flag(),
        detail: "Include embedded windows.",
        ..OptionSpec::DEFAULT
    },
];

const SYNC_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-command",
    value: OptionValue::deferred_command_prefix_n("command", AppendedArity::Exactly(0)),
    detail: "Command prefix scheduled once after line-height metrics become current.",
    ..OptionSpec::DEFAULT
}];

/// The command's subcommands.
macro_rules! query_form {
    ($name:literal, $arity:expr, $($word:literal),+ $(,)?) => {
        SubCommandForm {
            name: $name,
            arity: $arity,
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&[$($word),+])),
            traits: Some(Traits::PURE),
            mutator: Some(false),
            side_effects: Some(super::common::TTK_WIDGET_READS),
            ..SubCommandForm::DEFAULT
        }
    };
}

macro_rules! mutation_form {
    ($name:literal, $arity:expr, $($word:literal),+ $(,)?) => {
        SubCommandForm {
            name: $name,
            arity: $arity,
            literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&[$($word),+])),
            traits: Some(Traits::empty()),
            mutator: Some(true),
            side_effects: Some(super::common::TTK_WIDGET_READS_WRITES),
            ..SubCommandForm::DEFAULT
        }
    };
}

const EDIT_FORMS: &[SubCommandForm] = &[
    query_form!("canredo", Arity::exact(1), "canredo"),
    query_form!("canundo", Arity::exact(1), "canundo"),
    query_form!("modified-query", Arity::exact(1), "modified"),
    mutation_form!("modified-set", Arity::exact(2), "modified"),
    mutation_form!("redo", Arity::exact(1), "redo"),
    mutation_form!("reset", Arity::exact(1), "reset"),
    mutation_form!("separator", Arity::exact(1), "separator"),
    mutation_form!("undo", Arity::exact(1), "undo"),
];

const IMAGE_FORMS: &[SubCommandForm] = &[
    query_form!("cget", Arity::exact(3), "cget"),
    query_form!("configure-all", Arity::exact(2), "configure"),
    query_form!("configure-one", Arity::exact(3), "configure"),
    mutation_form!(
        "configure-set",
        Arity::stepped(4, Arity::UNLIMITED, 2),
        "configure"
    ),
    mutation_form!("create", Arity::stepped(2, Arity::UNLIMITED, 2), "create"),
    query_form!("names", Arity::exact(1), "names"),
];

const MARK_FORMS: &[SubCommandForm] = &[
    query_form!("gravity-query", Arity::exact(2), "gravity"),
    mutation_form!("gravity-set", Arity::exact(3), "gravity"),
    query_form!("names", Arity::exact(1), "names"),
    query_form!("next", Arity::exact(2), "next"),
    query_form!("previous", Arity::exact(2), "previous"),
    mutation_form!("set", Arity::exact(3), "set"),
    mutation_form!("unset", Arity::at_least(2), "unset"),
];

const PEER_FORMS: &[SubCommandForm] = &[
    mutation_form!("create", Arity::at_least(2), "create"),
    query_form!("names", Arity::exact(1), "names"),
];

const TAG_FORMS: &[SubCommandForm] = &[
    mutation_form!("add", Arity::at_least(3), "add"),
    query_form!("bind-all", Arity::exact(2), "bind"),
    query_form!("bind-one", Arity::exact(3), "bind"),
    SubCommandForm {
        name: "bind-set",
        arity: Arity::exact(4),
        literal_argument_prefix: Some(LiteralArgumentPrefix::unique(&["bind"])),
        traits: Some(Traits::DEFERS_BODY),
        mutator: Some(true),
        side_effects: Some(super::common::TTK_WIDGET_READS_WRITES),
        ..SubCommandForm::DEFAULT
    },
    query_form!("cget", Arity::exact(3), "cget"),
    query_form!("configure-all", Arity::exact(2), "configure"),
    query_form!("configure-one", Arity::exact(3), "configure"),
    mutation_form!(
        "configure-set",
        Arity::stepped(4, Arity::UNLIMITED, 2),
        "configure"
    ),
    mutation_form!("delete", Arity::at_least(2), "delete"),
    mutation_form!("lower", Arity::new(2, 3), "lower"),
    query_form!("names", Arity::new(1, 2), "names"),
    query_form!("nextrange", Arity::new(3, 4), "nextrange"),
    query_form!("prevrange", Arity::new(3, 4), "prevrange"),
    mutation_form!("raise", Arity::new(2, 3), "raise"),
    query_form!("ranges", Arity::exact(2), "ranges"),
    mutation_form!("remove", Arity::at_least(3), "remove"),
];

const WINDOW_FORMS: &[SubCommandForm] = &[
    query_form!("cget", Arity::exact(3), "cget"),
    query_form!("configure-all", Arity::exact(2), "configure"),
    query_form!("configure-one", Arity::exact(3), "configure"),
    mutation_form!(
        "configure-set",
        Arity::stepped(4, Arity::UNLIMITED, 2),
        "configure"
    ),
    mutation_form!("create", Arity::stepped(2, Arity::UNLIMITED, 2), "create"),
    query_form!("names", Arity::exact(1), "names"),
];

static SUBCOMMANDS: [SubCommand; 27] = [
    SubCommand {
        name: "bbox",
        arity: Arity::exact(1),
        detail: "Return the bounding box of the character at the given index.",
        synopsis: "pathName bbox index",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cget",
        arity: Arity::exact(1),
        detail: "Return the current value of a text option.",
        synopsis: "pathName cget option",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(0),
        detail: "Query or change text options.",
        synopsis: "pathName configure ?option? ?value option value ...?",
        return_type: Some(TclType::String),
        subcommand_forms: super::common::CONFIGURE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "compare",
        arity: Arity::exact(3),
        detail: "Compare two indices according to a relational operator.",
        synopsis: "pathName compare index1 op index2",
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::at_least(2),
        detail: "Count the number of items between two indices.",
        synopsis: "pathName count ?option ...? index1 index2",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "debug",
        arity: Arity::new(0, 1),
        detail: "Enable or query consistency checking of the B-tree code.",
        synopsis: "pathName debug ?boolean?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Delete a range of characters from the text.",
        synopsis: "pathName delete index1 ?index2 ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dlineinfo",
        arity: Arity::exact(1),
        detail: "Return display information for the display line containing index.",
        synopsis: "pathName dlineinfo index",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dump",
        traits: Traits::TAINT_SOURCE.union(Traits::EVALUATES_CODE),
        arity: Arity::at_least(1),
        detail: "Return text contents in a parseable form, or call -command once per dumped segment.",
        synopsis: "pathName dump ?-all -image -mark -tag -text -window? ?-command commandPrefix? index1 ?index2?",
        options: DUMP_OPTIONS,
        // `-command` invokes arbitrary Tcl synchronously, making the whole
        // subcommand conservatively effectful even though the no-callback
        // form is a simple value query.
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "edit",
        arity: Arity::at_least(1),
        detail: "Control the undo/redo mechanism and modified flag.",
        synopsis: "pathName edit option ?arg ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: EDIT_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        traits: Traits::TAINT_SOURCE,
        arity: Arity::at_least(1),
        detail: "Return the text from the widget between the given indices.",
        synopsis: "pathName get ?-displaychars? ?--? index1 ?index2 ...?",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "image",
        arity: Arity::at_least(1),
        detail: "Manipulate images embedded in the text widget.",
        synopsis: "pathName image option ?arg ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: IMAGE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the position of index in line.char form.",
        synopsis: "pathName index index",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert text at the given index.",
        synopsis: "pathName insert index chars ?tagList chars tagList ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "locale",
        arity: Arity::exact(1),
        detail: "Return the locale governing word and character boundaries at an index.",
        synopsis: "pathName locale index",
        lifecycle: Lifecycle::introduced_in("9.1"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mark",
        arity: Arity::at_least(1),
        detail: "Manipulate marks within the text widget.",
        synopsis: "pathName mark option ?arg ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: MARK_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "peer",
        arity: Arity::at_least(1),
        detail: "Create or list peer text widgets.",
        synopsis: "pathName peer option ?arg ...?",
        mutator: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: PEER_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pendingsync",
        arity: Arity::exact(0),
        detail: "Return whether asynchronous line-height calculations are pending.",
        synopsis: "pathName pendingsync",
        pure: true,
        return_type: Some(TclType::Boolean),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::at_least(2),
        detail: "Replace a range of text with new text.",
        synopsis: "pathName replace index1 index2 chars ?tagList chars tagList ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(2),
        detail: "Implement scanning (fast dragging) of the text widget.",
        synopsis: "pathName scan option args",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "search",
        arity: Arity::at_least(2),
        detail: "Search for text matching a pattern within the widget.",
        synopsis: "pathName search ?switches? pattern index ?stopIndex?",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "see",
        arity: Arity::exact(1),
        detail: "Scroll the widget so the character at index is visible.",
        synopsis: "pathName see index",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sync",
        arity: Arity::exact(2).with_also_exact(0),
        detail: "Bring line-height metrics up to date now, or schedule a command once they are current.",
        synopsis: "pathName sync ?-command commandPrefix?",
        traits: Traits::DEFERS_BODY,
        options: SYNC_OPTIONS,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tag",
        arity: Arity::at_least(1),
        detail: "Manipulate tags applied to ranges of text.",
        synopsis: "pathName tag option ?arg ...?",
        arg_role_resolver: Some(text_tag_arg_roles),
        script_timing_resolver: Some(text_tag_script_timing),
        callback_taint_inputs: &[(3, USER_EVENT_INPUTS)],
        body_kind: BodyKind::Structural,
        traits: Traits::DEFERS_BODY,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: TAG_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "window",
        arity: Arity::at_least(1),
        detail: "Manipulate embedded windows within the text widget.",
        synopsis: "pathName window option ?arg ...?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: WINDOW_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal position of the text in the window.",
        synopsis: "pathName xview ?args?",
        mutator: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: super::common::VIEW_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yview",
        arity: Arity::at_least(0),
        detail: "Query or change the vertical position of the text in the window.",
        synopsis: "pathName yview ?args?",
        mutator: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        subcommand_forms: super::common::VIEW_FORMS,
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale name"),
        detail: "Locale used to determine word and character boundaries (Tk 9.1+).",
        lifecycle: Lifecycle::introduced_in("9.1"),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired width of the text widget in characters.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-background",
        value: OptionValue::value("color"),
        detail: "Normal background colour of the text widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value("screen units"),
        detail: "Width of the border around the text widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-foreground",
        value: OptionValue::value("color"),
        detail: "Normal foreground colour of the text widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value(""),
        detail: "Desired height of the text widget in lines.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-wrap",
        value: OptionValue::value(""),
        detail: "Line wrapping mode: none, char, or word.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value(""),
        detail: "State of the text widget: normal or disabled.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value(""),
        detail: "Font to use for text in the widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-bg",
        value: OptionValue::value(""),
        detail: "Shorthand for -background.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-fg",
        value: OptionValue::value(""),
        detail: "Shorthand for -foreground.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-spacing1",
        value: OptionValue::value(""),
        detail: "Extra space above each line of text, in screen units.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-spacing2",
        value: OptionValue::value(""),
        detail: "Extra space between display lines within a logical line, in screen units.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-spacing3",
        value: OptionValue::value(""),
        detail: "Extra space below each line of text, in screen units.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-tabs",
        value: OptionValue::value(""),
        detail: "Tab stop positions and alignment for the text widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertbackground",
        value: OptionValue::value(""),
        detail: "Colour of the insertion cursor.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the insertion cursor.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertofftime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is off during blinking.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertontime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is on during blinking.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertwidth",
        value: OptionValue::value(""),
        detail: "Width of the insertion cursor in screen units.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectbackground",
        value: OptionValue::value(""),
        detail: "Background colour for selected text.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around selected text.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour for selected text.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for communicating with horizontal scrollbars.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-yscrollcommand",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for communicating with vertical scrollbars.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-exportselection",
        value: OptionValue::value(""),
        detail: "Whether the selection is exported to the X selection.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-setgrid",
        value: OptionValue::value(""),
        detail: "Whether this widget controls the resizing grid for its toplevel.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-padx",
        value: OptionValue::value(""),
        detail: "Extra horizontal padding inside the text widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-pady",
        value: OptionValue::value(""),
        detail: "Extra vertical padding inside the text widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-undo",
        value: OptionValue::value(""),
        detail: "Whether the undo mechanism is active.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-maxundo",
        value: OptionValue::value(""),
        detail: "Maximum number of compound undo actions on the undo stack.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-autoseparators",
        value: OptionValue::value(""),
        detail: "Whether undo separators are inserted automatically.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the text widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the text widget accepts focus during keyboard traversal.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the widget does not have focus.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the widget has focus.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-blockcursor",
        value: OptionValue::boolean(),
        detail: "Whether to draw the insertion cursor as a character-sized block.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-endline",
        value: OptionValue::value("line"),
        detail: "Line just after the last line exposed by this widget; empty uses the store's end.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-inactiveselectbackground",
        value: OptionValue::value("color"),
        detail: "Selection colour while unfocused; empty hides the unfocused selection.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertunfocussed",
        value: OptionValue::value("cursorStyle"),
        detail: "Unfocused insertion cursor style: none, hollow, or solid.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-startline",
        value: OptionValue::value("line"),
        detail: "First line from the underlying text store exposed by this widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-tabstyle",
        value: OptionValue::value("style"),
        detail: "Tab-stop interpretation: tabular or wordprocessor.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "text pathName ?option value ...?",
    ..FormSpec::DEFAULT
}];

/// `text`'s instance command dispatches through the same subcommand table
/// as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static TEXT_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "text",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
    method_prefix_matching: PrefixMatching::Enabled,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "text",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a multi-line text widget.",
            synopsis: &["text pathName ?option value ...?"],
            snippet: "Displays one or more lines of text and allows the user to edit them. Supports embedded images and windows.",
            source: "Tk man page text.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: &SUBCOMMANDS,
        object_class: Some(&TEXT_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(name: &str) -> &'static SubCommand {
        SUBCOMMANDS
            .iter()
            .find(|sub| sub.name == name)
            .expect("subcommand exists")
    }

    fn option<'a>(sub: &'a SubCommand, name: &str) -> &'a OptionSpec {
        sub.options
            .iter()
            .find(|option| option.name == name)
            .expect("option exists")
    }

    #[test]
    fn dump_command_is_a_synchronous_three_argument_prefix() {
        let dump = sub("dump");
        let OptionValue::Takes(command) = option(dump, "-command").value else {
            panic!("dump -command takes a value")
        };
        assert_eq!(command.role, ArgRole::CommandPrefix);
        assert_eq!(command.appended_arity, AppendedArity::Exactly(3));
        assert!(dump.traits.contains(Traits::EVALUATES_CODE));
        assert!(dump.side_effects.iter().any(|effect| {
            effect.target == SideEffectTarget::Unknown && effect.reads && effect.writes
        }));
    }

    #[test]
    fn sync_command_is_a_deferred_zero_argument_prefix() {
        let sync = sub("sync");
        let OptionValue::Takes(command) = option(sync, "-command").value else {
            panic!("sync -command takes a value")
        };
        assert_eq!(command.role, ArgRole::CommandPrefix);
        assert_eq!(command.appended_arity, AppendedArity::Exactly(0));
        assert!(sync.arity.accepts(0));
        assert!(!sync.arity.accepts(1));
        assert!(sync.arity.accepts(2));
        assert!(sync.traits.contains(Traits::DEFERS_BODY));
        assert!(!sync.traits.contains(Traits::EVALUATES_CODE));
    }
}
