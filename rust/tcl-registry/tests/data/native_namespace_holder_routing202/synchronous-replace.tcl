namespace eval N {}
             foreach n {one two three four five six seven eight nine ten} {
                 proc ::N::$n {} {}
                 trace add command ::N::$n delete rec
             }
                 proc remake {old new op} {
                     proc ::N::two {} {}
                     trace add command ::N::two delete rec
                     lappend ::log remade
                 }
                 trace add command ::N::two delete remake
                 namespace delete ::N