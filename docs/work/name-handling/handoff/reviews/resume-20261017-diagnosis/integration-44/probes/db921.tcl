# DB921 probe: one evaluated proc is one command with one body.
proc cmds {i} { lsort [concat [$i eval {info commands ::*}] [$i eval {info commands ::n::*}]] }
proc delta {i script} {
    set before [cmds $i]
    set code [catch {$i eval $script} res]
    set added {}
    foreach c [cmds $i] { if {[lsearch -exact $before $c] < 0} { lappend added $c } }
    list code $code added $added
}
set i [interp create]
puts "[info patchlevel] fixture: [delta $i {proc sample {value} {puts $value}}]"
puts "  info procs sample=[$i eval {info procs sample}] args=[list [$i eval {info args sample}]] body=[list [$i eval {info body sample}]]"
interp delete $i
set i [interp create]
puts "  two sites, same body text: [delta $i {proc a {v} {puts $v}; proc b {v} {puts $v}}]"
interp delete $i
set i [interp create]
puts "  one site, two executed bodies: [delta $i {foreach b {{puts 1} {puts 2}} {proc p {} $b}}] final body=[list [$i eval {info body p}]]"
interp delete $i
set i [interp create]
$i eval {namespace eval n {}}
puts "  same text, other namespace: [delta $i {proc sample {value} {puts $value}; namespace eval n {proc sample {value} {puts $value}}}]"
interp delete $i
