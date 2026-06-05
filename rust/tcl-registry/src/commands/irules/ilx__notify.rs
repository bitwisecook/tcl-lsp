//! `ILX::notify` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::notify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Calls an ILX method asynchronously.",
            synopsis: &["ILX::notify HANDLE METHOD (ARGS)*"],
            snippet: "Make a call to the plugin extension defined by the handle but do not wait for a response before continuing to process the remainder of the iRule. The delivery of the call to the plugin extension is \"best effort\" and is not guaranteed.",
            source: "https://clouddocs.f5.com/api/irules/ILX__notify.html",
            examples: "when CLIENT_ACCEPTED {\n    # Get a handle to the running extension instance to call into.\n    set RPC_HANDLE [ILX::init my_plugin my_extension]\n    # Make the asynchronous call\n    ILX::notify $RPC_HANDLE my_js_function arg1 arg2\n}",
            return_value: "None",
        }),
        ..CommandSpec::DEFAULT
    }
}
