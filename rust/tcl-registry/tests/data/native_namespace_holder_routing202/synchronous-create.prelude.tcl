set log {}
                 proc rec {old new op} {lappend ::log [namespace tail $old]}