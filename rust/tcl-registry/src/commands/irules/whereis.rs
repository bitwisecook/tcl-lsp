//! `whereis` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "whereis",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns geographical information on an IP address.",
            synopsis: &["whereis (ldns | IP_ADDR)"],
            snippet: "Returns the geographic location of a specific IP address.\nFor more information on using whereis in LTM, you can check Jason\nRahm's article\n\nLegal usage notes\n\n   The data is purchased by F5 for use on BIG-IP systems and products for\n   traffic management. The key to understanding EULA compliance is to\n   figure out where the geolocation decision is being made.",
            source: "https://clouddocs.f5.com/api/irules/whereis.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "whereis (ldns | IP_ADDR)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
