puts [list [info complete {set a b}] [info complete {set a "b}] [info complete "set a \{b"] [info complete {set a [b}] [info complete "# \{"] [info complete "set a \\\{"]]
