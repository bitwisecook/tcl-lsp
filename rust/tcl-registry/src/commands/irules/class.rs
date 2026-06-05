//! `class` iRules command.
use crate::prelude::*;

/// iRules subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "match",
        arity: Arity::at_least(0),
        detail: "Match an item against a data group.",
        synopsis: "class match ?options? ?--? <item> <operator> <class>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "search",
        arity: Arity::at_least(0),
        detail: "Search a data group for an item.",
        synopsis: "class search ?options? ?--? <class> <operator> <item>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::exact(2),
        detail: "Return the value paired with a name.",
        synopsis: "class lookup <name> <class>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "element",
        arity: Arity::exact(2),
        detail: "Return an element by index.",
        synopsis: "class element ?-value|-name? ?--? <index> <class>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Return the data type of a data group.",
        synopsis: "class type <class>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Check if a data group exists.",
        synopsis: "class exists <class>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Return the number of elements.",
        synopsis: "class size <class>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::new(1, 2),
        detail: "Return list of data group names.",
        synopsis: "class names ?-nocase? ?-list? ?--? <class> ?pattern?",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::new(1, 2),
        detail: "Return all elements as a list.",
        synopsis: "class get ?-nocase? ?-list? ?--? <class> ?pattern?",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "startsearch",
        arity: Arity::exact(1),
        detail: "Begin iterating over a data group.",
        synopsis: "class startsearch <class>",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nextelement",
        arity: Arity::exact(1),
        detail: "Get next element during iteration.",
        synopsis: "class nextelement ?options? ?--? <search_id>",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "anymore",
        arity: Arity::exact(1),
        detail: "Check if more elements remain.",
        synopsis: "class anymore <search_id>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "donesearch",
        arity: Arity::exact(1),
        detail: "End a data group iteration.",
        synopsis: "class donesearch <search_id>",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "class",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Advanced access of classes.",
            synopsis: &["class 'match' (((CLASS_SEARCH_OPTION) ('-all'))#)? ('--')? ITEM CLASS_OPERATOR CLASS_OBJ", "class match attempts to match the provided <item> to an element in <class> by applying the <operator> to the <item>.", "class match [HTTP::uri] ends_with image_class"],
            snippet: "The class command, implemented in v10.0.0, allows you to query data groups and data group properties.\n\nThese commands work for both internal (defined in the bigip.conf) and external (custom file) data groups. Internal data groups were not able to make use of the name/value pairing with the := separator until version 10.1. As of 10.1 all classes support the name/value pairing.\n\nThe class command deprecates the findclass and matchclass commands as it offers better functionality and performance than the older commands.",
            source: "https://clouddocs.f5.com/api/irules/class.html",
            examples: "when LB_FAILED {\n      HTTP::respond 200 content [b64decode [class element -value 0 img]] \"Content-Type\" \"image/png\"\n   }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "class <subcommand> ?options? ?--? args..." },
        ],
        options: &[
            OptionSpec { name: "-all", takes_value: false, value_hint: "", detail: "Return all matches.", dialects: None },
            OptionSpec { name: "-value", takes_value: false, value_hint: "", detail: "Return value instead of name.", dialects: None },
            OptionSpec { name: "-name", takes_value: false, value_hint: "", detail: "Return name.", dialects: None },
            OptionSpec { name: "-index", takes_value: false, value_hint: "", detail: "Return index.", dialects: None },
            OptionSpec { name: "-element", takes_value: false, value_hint: "", detail: "Return full element.", dialects: None },
            OptionSpec { name: "-nocase", takes_value: false, value_hint: "", detail: "Case-insensitive comparison.", dialects: None },
            OptionSpec { name: "-list", takes_value: false, value_hint: "", detail: "Return value always as a list.", dialects: None },
        ],
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
