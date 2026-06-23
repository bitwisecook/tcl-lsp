//! `::tcl::unsupported::corotype` — inspect a coroutine's suspension state.
//!
//! Tcl 9 ships `::tcl::unsupported::corotype CORONAME` as part of the
//! `::tcl::unsupported::*` namespace — a documented-but-internal API
//! exposed for tooling and the tcltest harness.
//!
//! Only the fully-qualified spelling is registered (matches the
//! WASM `ns_resolve_qualified` path).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "::tcl::unsupported::corotype",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::new(1, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Return the suspension state of a coroutine.",
            &["::tcl::unsupported::corotype coroName"],
            "Tcl ::tcl::unsupported::corotype (internal)",
        )),
        ..CommandSpec::DEFAULT
    }
}
