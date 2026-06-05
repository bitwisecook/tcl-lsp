//! `CLASSIFY::urlcat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::urlcat",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Allows to set or add an url category to the classification.",
            synopsis: &["CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME"],
            snippet: "This command allows you to set or add an url category to the\nclassification.\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFY::urlcat set <URL_Category>\n\n     * will immediately classify flow as URL_category.\n\nCLASSIFY::application add <app_name>\n\n     * adds an URL Category to the URL classification token to the final\n       classification result issued by the classification engine. This can\n       be issued multiple times in order to add multiple tokens to the classification result.",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__urlcat.html",
            examples: "when HTTP_REQUEST\n{\n    if { [HTTP::host] contains \"google\"} {\n        CLASSIFY::urlcat set customCategory\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
