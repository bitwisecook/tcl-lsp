//! `CATEGORY::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the category or safesearch results retrieved during normal traffic flow.",
            synopsis: &["CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')"],
            snippet: "This iRule command is useful for when it is necessary to know the category or safesearch parameters returned during the categorization in the Category Lookup Agent in the per-request policy. As opposed to CATEGORY::lookup and CATEGORY::safesearch, which each require an additional query to the categorization engine, CATEGORY::result will give back what was found and stored, eliminating the need for additional lookups.\n\nChoose which should be returned (either \"category\" or \"safesearch\"). If \"category\", additional specifications may apply: \"-display\" will return categories in display name format.",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__result.html",
            examples: "when CATEGORY_MATCHED {\n    set cat [CATEGORY::result category -display request_default_and_custom]\n    log local0. \"Category result retrieved: [lindex $cat 0]\"\n    set ss [CATEGORY::result safesearch]\n    log local0. \"Safe Search result retrieved: [lindex $ss 0], [lindex $ss 1]\"\n}",
            return_value: "Returns a list of categories or safe search parameters. Return format is the same as CATEGORY::lookup and CATEGORY::safesearch.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CATEGORY"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')" },
        ],
        options: &[
            OptionSpec { name: "-display", takes_value: false, value_hint: "", detail: "Return categories in display name format.", dialects: None },
            OptionSpec { name: "-id", takes_value: false, value_hint: "", detail: "Return categories in ID format.", dialects: None },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ClassificationState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
