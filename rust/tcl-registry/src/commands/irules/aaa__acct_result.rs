//! `AAA::acct_result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::acct_result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used to check whether the accounting information is sent successfully to IVS(internal virtual server) or not.",
            synopsis: &["AAA::acct_result AAA_REQUEST_ID"],
            snippet: "This command is used to check whether the accounting information is sent successfully to IVS(internal virtual server) or not.",
            source: "https://clouddocs.f5.com/api/irules/AAA__acct_result.html",
            examples: "when HTTP_REQUEST_DATA {\n    set aaa_result [AAA::acct_result $request_id]\n    if { $aaa_result == \"INPROGRESS\"  } {\n        after 200\n        continue\n    }\n\n    if { $aaa_result == \"OK\" } {\n        # request was successfull\n    } else {\n        # handle errors\n    }\n}",
            return_value: "There are 4 possible return values for this command (All STRING type): \"OK\" - the request was successful. \"FAIL\" - the request has been rejected. \"INPROGRESS\" - the request is still in progress (asyncronous). \"ERROR\" - there was an error during the request.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AAA::acct_result AAA_REQUEST_ID" },
        ],
        ..CommandSpec::DEFAULT
    }
}
