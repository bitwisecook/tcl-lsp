//! `POLICY::controls` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::controls",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns details about the policy controls for the virtual server the iRule is enabled on.",
            synopsis: &["POLICY::controls ('acceleration' |"],
            snippet: "Returns details about the policy controls for the policies associated\nwith the virtual server that the iRule is enabled on. Policy controls\nare typically virtual server profiles or features which can be enabled,\ndisabled or modified per iRule execution via policies.\n\nAs of v11.4 the following controls are available:\n acceleration        - Application Acceleration Manager (AAM)\n asm                 - Application Security Manager\n avr                 - Application Visibility and Reporting\n caching             - HTTP caching",
            source: "https://clouddocs.f5.com/api/irules/policy__controls.html",
            examples: "# Log the policy controls for this virtual server\nwhen HTTP_REQUEST {\n\n        # Log the policy controls enabled on this virtual server\n        log local0. \"\\[POLICY::controls\\]: [POLICY::controls]\"\n\n        # Loop through each possible control type and log whether it is enabledor not (1 for enabled, 0 for not enabled)\n        foreach control {acceleration asm avr caching classification compression forwarding l7dos bot-defense request-adaptation response-adaptation server-ssl} {",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
