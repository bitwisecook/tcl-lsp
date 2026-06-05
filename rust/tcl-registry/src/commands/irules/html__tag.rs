//! `HTML::tag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Query and update the HTML tag.",
            synopsis: &["HTML::tag ((append STRING) | name | (prepend STRING) | remove)", "HTML::tag append <data>", "HTML::tag name", "HTML::tag prepend <data>"],
            snippet: "Queries, removes and changes attribute/value pairs of this HTML tag.\n        \nHTML::tag append <data>\nInsert a string after the closing delimiter of the HTML tag; when multiple appends are issued, the inserted strings are ordered according to the sequence of the append commands as they are issued for the given tag.\n\nHTML::tag name\nReturn HTML tag name, where name is the HTML element if the tag is a start tag, and if the tag is an end tag, tag name returns \"/\" + the HTML element.",
            source: "https://clouddocs.f5.com/api/irules/HTML__tag.html",
            examples: "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n    HTTP::header replace \"Host\" \"finance.yahoo.com\"\n}",
            return_value: "\"HTML::tag name\" returns tag name. \"HTML::tag attribute value <name>\" returns the value of the attribute under this HTML tag. \"HTML::tag attribute count\" returns the number of attributes in this HTML tag.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTML"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTML::tag ((append STRING) | name | (prepend STRING) | remove)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
