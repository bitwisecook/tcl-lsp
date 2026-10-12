namespace eval N {
                    proc p {} {
                        namespace delete ::N
                        proc r {} {return R}
                        list [r] [info commands ::N::r] [namespace which r] \
                             [catch {proc ::N::s {} {}} m] $m [info commands ::N::*] \
                             [namespace exists ::N]
                    }
                }
                set a [::N::p]
                list $a [info commands ::N::*] [namespace exists ::N]