//! `create_bd_intf_port` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "create_bd_intf_port -mode mode -vlnv vlnv port_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_bd_intf_port",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a block design interface port.",
            &["create_bd_intf_port -mode mode -vlnv vlnv port_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
