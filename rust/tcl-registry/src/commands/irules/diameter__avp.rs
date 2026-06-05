//! `DIAMETER::avp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::avp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Provides detailed access to diameter attribute-value pairs.",
            synopsis: &["DIAMETER::avp <subcommand> ?args?", "DIAMETER::avp code <avp_code> ?vendor_id? ?index?", "DIAMETER::avp data <avp_code> ?vendor_id? ?index?"],
            snippet: "This iRule command gives access to set and get attribute-value pairs.\nSpecifics for each command are below in the syntax section.\n\nThe AVP upon which this command operates is specified in a flexible\nmanner.  An AVP name or code (usually) must be specified, and an\noptional index may be also specified.  Many commands also accept a\nvendor-id.  When an AVP name is specified, it is converted to a code.\nNames are written as listed in RFC 3588, formatted as e.g.,\n\"HOST-IP-ADDRESS\".  AVP codes are 32-bit (4-octet) integer values.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__avp.html",
            examples: "when DIAMETER_EGRESS {\n     # Sets the flags of the AVP Product Name to 0 (for Vendor Specific, Mandatory, Protected and Reserved)\n     DIAMETER::avp flags set 269 0\n     # Checks that the flags are properly set (was a bug in 11.3, solved in 11.4)\n     log local0. \"AVP : [DIAMETER::avp flags get 269] \"\n     # Removes the Supported-Vendor-Id from the request\n     DIAMETER::avp delete 265\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::avp <subcommand> ?args?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
