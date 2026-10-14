if {[catch {info patchlevel} version]} {catch {info version} version}
puts [list version $version]
proc overwriteBody {} {upvar 1 body body; set body {return replacement}; return 1}
proc readOverwrite {name index op} {upvar 1 body body; set body {return replacement}}
proc exprCommandProbe {} {set body {return original}; expr {[overwriteBody]}; set body}
proc exprReadTraceProbe {} {set body {return original}; set watched 1; trace variable watched r readOverwrite; expr {$watched}; set body}
proc assignExprProbe {} {set body {return original}; set other [expr {[overwriteBody]}]; set body}
proc assignValueProbe {} {set body {return original}; set other [overwriteBody]; set body}
proc incrReadTraceProbe {} {set body {return original}; set watched 1; set other 0; trace variable watched r readOverwrite; incr other $watched; set body}
proc constantWriteTraceProbe {} {set body {return original}; set watched initial; trace variable watched w readOverwrite; set watched {literal}; set body}
foreach test {exprCommandProbe exprReadTraceProbe assignExprProbe assignValueProbe incrReadTraceProbe constantWriteTraceProbe} {
    set code [catch {eval $test} result]
    puts [list control $test $code $result]
}
