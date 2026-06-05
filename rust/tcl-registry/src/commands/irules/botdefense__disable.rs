//! `BOTDEFENSE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables processing by Bot Defense on the connection.",
            synopsis: &["BOTDEFENSE::disable"],
            snippet: "Disables processing and blocking of the request by Bot Defense for the duration of the current TCP connection, or until BOTDEFENSE::enable is called.\nWhen called from events that occur before Bot Defense processing such as HTTP_REQUEST then the commands takes effect on the current request. Otherwise, if invoked in the BOTDEFENSE_REQUEST, BOTDEFENSE_ACTION or any other event that occurs after Bot Defense processing then the command will take effect only on the following request on the same connection.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__disable.html",
            examples: "# EXAMPLE: Disabling Bot Defense for a netmask of client IP addresses\nwhen CLIENT_ACCEPTED {\n    if {[IP::addr [IP::client_addr] equals 10.10.10.0/24]} {\n        BOTDEFENSE::disable\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::disable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
