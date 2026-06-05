//! `ILX::init` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::init",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates a handle to a running ILX plugin extension.",
            synopsis: &["ILX::init (EXTENSION | (PLUGIN EXTENSION))"],
            snippet: "Creates a handle for future use by ILX::call and ILX::notify.  This handle is a reference to a running ILX plugin extension.  The lifetime of this variable affects the behavior of the ILX target if controlled by BIG-IP.  Instances of the plugin extension will be held in draining mode as long as there are open references to the ILX handle in any event.",
            source: "https://clouddocs.f5.com/api/irules/ILX__init.html",
            examples: "when CLIENT_ACCEPTED {\n    # Get a handle to the running extension instance to call into.\n    set RPC_HANDLE [ILX::init my_plugin my_extension]\n    # Make the call and store the response in $rpc_response\n    set rpc_response [ILX::call $RPC_HANDLE my_js_function arg1 arg2]\n}",
            return_value: "Returns a handle to the running extension to call into.",
        }),
        ..CommandSpec::DEFAULT
    }
}
