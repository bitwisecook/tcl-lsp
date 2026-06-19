//! iRules BIG-IP object-reference spans for the semantic-tokens provider.
//!
//! The extractor itself now lives in the shared [`tcl_irules`] crate so the LSP
//! and the BIG-IP reference graph (`tcl-bigip`) share one implementation; this
//! module is a thin re-export of [`object_ref_spans`]. The tests below stay here
//! as a regression check that the highlighting behaviour the semantic-tokens
//! layer depends on is preserved.

pub use tcl_irules::object_ref_spans;

#[cfg(test)]
mod tests {
    use super::object_ref_spans;
    use tcl_registry::CommandRegistry;

    fn spans(src: &str) -> Vec<(usize, &str)> {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        object_ref_spans(src, &reg)
            .into_iter()
            .map(|s| (s.start() as usize, &src[s.as_range()]))
            .collect()
    }

    #[test]
    fn pool_reference_in_event_body() {
        // `pool` inside a `when` body is found via BODY recursion.
        let got = spans("when HTTP_REQUEST {\n  pool web_pool\n}\n");
        assert!(got.iter().any(|(_, n)| *n == "web_pool"), "{got:?}");
    }

    #[test]
    fn pool_member_subcommand_is_not_a_reference() {
        let got = spans("when HTTP_REQUEST {\n  pool members foo\n}\n");
        assert!(!got.iter().any(|(_, n)| *n == "foo"), "{got:?}");
    }

    #[test]
    fn virtual_and_node_and_snatpool_refs() {
        let got = spans("when CLIENT_ACCEPTED {\n  virtual vs1\n  node n1\n  snatpool sp1\n}\n");
        let names: Vec<&str> = got.iter().map(|(_, n)| *n).collect();
        assert!(names.contains(&"vs1"), "{names:?}");
        assert!(names.contains(&"n1"), "{names:?}");
        assert!(names.contains(&"sp1"), "{names:?}");
    }

    #[test]
    fn class_lookup_data_group_ref() {
        let got = spans("when HTTP_REQUEST {\n  class lookup [HTTP::host] my_dg\n}\n");
        assert!(got.iter().any(|(_, n)| *n == "my_dg"), "{got:?}");
    }

    #[test]
    fn data_group_ref_inside_if_condition() {
        // `class match` inside an `if` *condition* (an EXPR-role word) lives in
        // a braced expression; the EXPR recursion must still find `dg`.
        let got = spans(
            "when HTTP_REQUEST {\n  if {[class match [HTTP::host] equals dg]} {\n    pool p\n  }\n}\n",
        );
        let names: Vec<&str> = got.iter().map(|(_, n)| *n).collect();
        assert!(names.contains(&"dg"), "{names:?}");
        assert!(names.contains(&"p"), "{names:?}");
    }

    #[test]
    fn substituted_pool_name_is_not_a_reference() {
        // `pool $p` — the name is an unbound variable, not a literal.
        let got = spans("when HTTP_REQUEST {\n  pool $p\n}\n");
        assert!(got.is_empty(), "{got:?}");
    }
}
