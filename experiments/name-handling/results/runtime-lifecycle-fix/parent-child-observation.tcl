set seen {}
                      proc ensdeleted {tag old new op} {
                          lappend ::seen [list $tag [namespace exists ::P]                                                [namespace exists ::P::C]]
                      }
                      proc cmddeleted {old new op} {
                          lappend ::seen [list PC [namespace exists ::P]                                                [namespace exists ::P::C]                                                [info commands ::CE]]
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
                      set seen
puts [set seen]
