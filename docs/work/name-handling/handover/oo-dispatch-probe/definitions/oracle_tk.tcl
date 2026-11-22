set out [open [lindex $argv 0] w]
set extra [lindex $argv 1]
set only [expr {[lindex $argv 2] eq "only"}]
set argv {}; set argc 0
set skip {::http::wait http::wait ::tcltest::cleanupTests ::platform::shell::generic ::platform::shell::identify ::platform::shell::platform exit vwait ::exit ::vwait history ::history tcltest::runAllTests ::tcltest::runAllTests tcltest::main ::tcltest::main}
set pkgs {Tk}
proc esc {s} { string map [list \\ \\\\ \t { } \n { } \r { }] $s }
proc classify {code msg} {
    if {$code == 0 || $code == 2 || $code == 3 || $code == 4} { return ok }
    if {[string match "wrong # args*" $msg]} { return args }
    if {[string match {invalid command name*} $msg]} { return nocmd }
    if {[regexp {^(unknown or ambiguous subcommand|unknown subcommand|bad option|unknown or ambiguous option|bad subcommand|ambiguous subcommand)} $msg]} { return badsub }
    return other
}
proc fresh {} {
    global pkgs
    set c [interp create]
    $c eval {set argv {}; set argc 0}
    foreach p $pkgs { catch {$c eval [list package require $p]} }
    catch {$c eval auto_load_index}
    return $c
}
set counts {0 1 2 3 4 5 6 7 12}
proc probe {c cmd sub} {
    global out counts
    foreach n $counts {
        set words [list $cmd]
        if {$sub ne ""} { lappend words $sub }
        for {set i 0} {$i < $n} {incr i} { lappend words __x__ }
        catch {$c eval [list catch {auto_load} ]}
        set code [catch {$c eval $words} msg]
        if {[catch {$c eval {set x 1}}]} { interp delete $c; set c [fresh] }
        puts $out "P\t$cmd\t$sub\t$n\t[classify $code $msg]\t[esc [string range $msg 0 160]]"
        flush $out
    }
    return $c
}
proc subs_of {c cmd} {
    set code [catch {$c eval [list $cmd __bogus__]} msg]
    if {[regexp {"__bogus__": must be (.*)$} $msg -> list] || [regexp {"__bogus__": should be (.*)$} $msg -> list]} {
        regsub -all {,? or } $list {, } list
        set r {}
        foreach w [split $list ,] { set w [string trim $w " ."]; if {$w ne "" && [regexp {^[-a-zA-Z0-9_:]+$} $w]} { lappend r $w } }
        return $r
    }
    return {}
}
set e [fresh]
set cmds [$e eval {info commands}]
foreach ns {::oo ::tcl::mathfunc ::tcl::mathop ::http ::msgcat ::tcltest ::platform ::platform::shell ::tcl::prefix ::tcl::unsupported} {
    if {![$e eval [list namespace exists $ns]]} continue
    set pats [$e eval [list namespace eval $ns {namespace export}]]
    foreach c [$e eval [list info commands ${ns}::*]] {
        if {[lsearch -exact {::oo ::tcl::mathfunc ::tcl::mathop ::tcl::prefix ::tcl::unsupported} $ns] >= 0} { lappend cmds $c; continue }
        set tail [namespace tail $c]
        foreach p $pats { if {[string match $p $tail]} { lappend cmds $c; break } }
    }
}
if {[$e eval {info exists auto_index}]} { foreach c [$e eval {array names auto_index}] { lappend cmds $c } }
set cmds [lsort -unique $cmds]
interp delete $e
puts $out "V\t[info patchlevel]"
foreach cmd $cmds {
    if {$only} break
    if {[lsearch -exact $skip $cmd] >= 0} continue
    set c [fresh]
    catch {$c eval [list auto_load $cmd]}
    puts $out "C\t$cmd\t[expr {[$c eval [list info commands $cmd]] ne {}}]"
    set s [subs_of $c $cmd]
    puts $out "S\t$cmd\t$s"
    set c [probe $c $cmd ""]
    foreach sub $s { set c [probe $c $cmd $sub] }
    catch {interp delete $c}
}
if {$extra ne "" && [file exists $extra]} {
    set f [open $extra]; set lines [split [read $f] \n]; close $f
    foreach l $lines {
        if {$l eq ""} continue
        foreach {cmd sub} [split $l \t] break
        if {[lsearch -exact $skip $cmd] >= 0} continue
        set c [fresh]
        catch {$c eval [list auto_load $cmd]}
        puts $out "X\t$cmd\t$sub\t[expr {[$c eval [list info commands $cmd]] ne {}}]"
        set c [probe $c $cmd $sub]
        catch {interp delete $c}
    }
}
close $out
