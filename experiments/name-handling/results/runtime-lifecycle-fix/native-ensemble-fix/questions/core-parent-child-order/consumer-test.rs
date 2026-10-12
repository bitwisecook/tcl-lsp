    #[test]
    fn parent_teardown_reaches_each_child_after_parent_commands() {
        // Tcl drains a parent's owned ensembles while both namespace tokens are
        // live, marks only that parent dying for its ordinary command traces,
        // and reaches each child ensemble through a separate recursive delete.
        // Exact Tcl 9.0.4 oracle result.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc ensdeleted {tag old new op} {
                          lappend ::seen [list $tag [namespace exists ::P] \
                                               [namespace exists ::P::C]]
                      }
                      proc cmddeleted {old new op} {
                          lappend ::seen [list PC [namespace exists ::P] \
                                               [namespace exists ::P::C] \
                                               [info commands ::CE]]
                      }
                      namespace eval ::P {
                          namespace ensemble create -command ::PE
                          proc p {} {}
                          namespace eval C {
                              namespace ensemble create -command ::CE
                          }
                      }
                      trace add command ::PE delete [list ensdeleted P]
                      trace add command ::P::p delete cmddeleted
                      trace add command ::CE delete [list ensdeleted C]
                      namespace delete ::P
                      set seen"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{P 1 1} {PC 0 1 ::CE} {C 0 1}");
        });
    }
