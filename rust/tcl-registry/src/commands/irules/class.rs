//! `class` iRules command.
use crate::prelude::*;
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
        ..CommandSpec::DEFAULT
    }
}
