puts [list IDENTITY [info patchlevel]]
set code [catch {
namespace eval N {
                          proc x {} {return OLD}
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {
                                  proc x {} {return NEW}
                                  namespace export x
                              }
                              namespace eval ::B {namespace import ::N::x}
                              rename x y
                              list [namespace origin ::A::x]                                    [namespace origin ::B::x]                                    [::A::x] [::B::x] [y]
                          }
                          namespace export x p
                      }
                      namespace eval A {namespace import ::N::x}
                      set inside [N::p]
                      list $inside [namespace origin ::B::x] [B::x]                            [info commands ::A::x]
} result]
puts [list OBSERVATION retained-import $code $result]
