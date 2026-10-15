namespace eval {a{b}} {variable x OUT}
proc sourceNameProbe {} {return ${a{b}::x}}
set sourceNameCode [catch {sourceNameProbe} sourceNameResult]
binary scan $sourceNameResult H* sourceNameHex
puts [list BRACED_QUALIFIED $sourceNameCode $sourceNameHex]
set ::g OUT
proc indexNameProbe {} {set {a({OUT})} IN; return $a({$::g})}
set indexNameCode [catch {indexNameProbe} indexNameResult]
binary scan $indexNameResult H* indexNameHex
puts [list INDEX_BRACES $indexNameCode $indexNameHex]
