namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
                 proc mk {old new op} {
                     lappend ::log [namespace tail $old]
                     proc ::N::zz {} {}
                     trace add command ::N::zz delete rec
                 }
                 trace add command ::N::six delete mk
                 namespace delete ::N