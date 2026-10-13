
foreach ns {::N[0] ::N0 ::Nmissing ::W* ::WZ ::Q? ::Qa {::B\slash}} {::namespace eval $ns {}}
foreach name {::N[0]::p ::N0::p ::Nmissing::absent ::W*::p ::WZ::p ::Q?::p ::Qa::p {::B\slash::p} ::N[0]::p* ::N[0]::pZ ::N[0]::p? ::N[0]::pA ::N[0]::p[0] ::N[0]::p0 {::N[0]::p\tail} ::N[0]:: ::N[0]::absentZ :p p* pZ absentZ} {
    ::proc $name {} {::return EXACT}
}
package provide demo 1.0

namespace eval ::__tcl_name_presence {}
proc ::__tcl_name_presence::in_root {cpattern jimpattern supported} {
    if {!$supported} {error "command presence has no original-byte script transport"}
    if {![catch {::info tclversion} version] && [lsearch -exact {8.4 8.5 8.6 9.0 9.1} $version] >= 0} {
        set pattern $cpattern
    } elseif {![catch {::info version} version] && $version eq "0.84"} {
        set pattern $jimpattern
    } else {
        error "command presence has no selected interpreter query grammar"
    }
    return [expr {[llength [uplevel #0 [list ::info commands $pattern]]] > 0}]
}

namespace eval ::__spec_test {}
set ::__spec_test::count 0
proc ::__spec_test::say {kind command detail} {
    set detail [string map [list "\\" "\\\\" "\t" "\\t" "\n" "\\n" "\r" "\\r"] $detail]
    puts "\nSPEC-TEST\t$kind\t$command\t$detail"
}
proc ::__spec_test::asked {name} {
    incr ::__spec_test::count
    ::__spec_test::say asked $name {}
}
proc ::__spec_test::finish {} {
    ::__spec_test::say done - $::__spec_test::count
}
if {[catch {package require demo} ::__spec_test::loaded]} {
    ::__spec_test::say load - $::__spec_test::loaded
    ::__spec_test::finish
    exit 0
}
proc ::__spec_test::words {count} {
    set words {}
    for {set i 0} {$i < $count} {incr i} {lappend words x}
    return $words
}
proc ::__spec_test::attempt {name count} {
    set call [linsert [::__spec_test::words $count] 0 $name]
    set code [catch {uplevel #0 $call} result]
    return [list $code $result $call]
}
proc ::__spec_test::refused {name count} {
    foreach {code result call} [::__spec_test::attempt $name $count] break
    return [list [expr {$code != 0 && [string match "wrong # args*" $result]}] $call]
}
set ::__spec_test::skip 0
proc ::__spec_test::present {name cpattern jimpattern supported} {
    set ::__spec_test::skip 0
    if {[catch {::__tcl_name_presence::in_root $cpattern $jimpattern $supported} present]} {
        set ::__spec_test::skip 1
        ::__spec_test::say unsupported $name "command presence unavailable: $present"
        return
    }
    if {$present} return
    if {![catch {uplevel #0 [list auto_load $name]} loaded] && $loaded} return
    set ::__spec_test::skip 1
    ::__spec_test::say missing $name "declared, but the package does not define it"
}
proc ::__spec_test::arity {name min max step} {
    if {$::__spec_test::skip} return
    if {$min > 0} {
        foreach {refused call} [::__spec_test::refused $name [expr {$min - 1}]] break
        if {!$refused} {
            ::__spec_test::say arity $name "declares at least $min argument(s), but `$call` was not refused as `wrong # args`"
        }
    }
    if {$max >= 0} {
        foreach {refused call} [::__spec_test::refused $name [expr {$max + 1}]] break
        if {!$refused} {
            ::__spec_test::say arity $name "declares at most $max argument(s), but `$call` was not refused as `wrong # args`"
        }
    }
    set inside $min
    if {$max >= 0 && $step == 0 && $max != $min} {lappend inside $max}
    foreach count $inside {
        foreach {refused call} [::__spec_test::refused $name $count] break
        if {$refused} {
            ::__spec_test::say arity $name "declares $count argument(s) valid, but `$call` was refused as `wrong # args`"
        }
    }
}
proc ::__spec_test::is {type value} {
    switch -- $type {
        int { return [expr {[string is integer -strict $value] || [regexp {^[-+]?[0-9]+$} $value]}] }
        double { return [string is double -strict $value] }
        bool { return [string is boolean -strict $value] }
        numeric { return [expr {[::__spec_test::is int $value] || [string is double -strict $value]}] }
        list { return [expr {![catch {llength $value}]}] }
        dict { return [expr {![catch {llength $value}] && [llength $value] % 2 == 0}] }
        default { return 1 }
    }
}
proc ::__spec_test::example {name type line} {
    if {$::__spec_test::skip} return
    if {[catch {uplevel #0 $line} result]} {
        ::__spec_test::say example $name "`$line` raised: $result"
        return
    }
    if {$type ne "" && ![::__spec_test::is $type $result]} {
        ::__spec_test::say returns $name "declares it returns $type, but `$line` answered `$result`"
    }
}
proc ::__spec_test::reference {name text lines} {
    if {$::__spec_test::skip} return
    set child [interp create]
    $child eval [list set ::auto_path $::auto_path]
    if {[catch {$child eval {package require demo}} failure] || [catch {$child eval $text} failure]} {
        ::__spec_test::say reference $name "the reference body could not be run: $failure"
        interp delete $child
        return
    }
    foreach line $lines {
        set command [catch {uplevel #0 $line} commandResult]
        set body [catch {$child eval $line} bodyResult]
        if {$command != $body || $commandResult ne $bodyResult} {
            ::__spec_test::say reference $name "`$line` answers `$commandResult` ($command) as the command and `$bodyResult` ($body) as its reference body"
        }
    }
    interp delete $child
}
proc ::__spec_test::variables {} {
    set variables {}
    foreach coordinate [::__tcl_name_snapshot::variables {::__spec_test ::tcl}] {
        foreach {ns name} $coordinate break
        if {$ns eq "::" && [lsearch -exact {errorInfo errorCode} $name] >= 0} continue
        lappend variables $coordinate
    }
    return $variables
}
proc ::__spec_test::snapshot {} {
    set state {}
    foreach coordinate [::__spec_test::variables] {
        if {[::__tcl_name_snapshot::call $coordinate {::array exists}]} {
            set pairs {}
            foreach key [lsort [::__tcl_name_snapshot::call $coordinate {::array names}]] {
                lappend pairs $key [::__tcl_name_snapshot::element_value $coordinate $key]
            }
            lappend state $coordinate [list array $pairs]
        } elseif {[::__tcl_name_snapshot::call $coordinate {::info exists}]} {
            lappend state $coordinate [list scalar [::__tcl_name_snapshot::call $coordinate {::set}]]
        }
    }
    return $state
}
proc ::__spec_test::changed {before after} {
    array set was $before
    array set now $after
    set names {}
    foreach name [concat [array names was] [array names now]] {
        if {![info exists was($name)] || ![info exists now($name)] || $was($name) ne $now($name)} {
            lappend names $name
        }
    }
    return [lsort -unique $names]
}
proc ::__spec_test::begin {} {
    set ::__spec_test::snapshot_ready 0
    set ::__spec_test::snapshot_reason {}
    if {$::__spec_test::skip} return
    set ::__spec_test::writes {}
    catch {unset ::__spec_test::traced}
    array set ::__spec_test::traced {}
    if {[catch {::__spec_test::snapshot} state]} {
        set ::__spec_test::snapshot_reason $state
        return
    }
    set ::__spec_test::mark $state
    if {[catch {::__spec_test::watch 0} reason]} {
        set ::__spec_test::snapshot_reason $reason
        ::__spec_test::end
        return
    }
    if {[catch {
        foreach helper {attempt example reference pure} {
            trace add execution ::__spec_test::$helper leavestep ::__spec_test::step
        }
    } reason]} {
        set ::__spec_test::snapshot_reason $reason
        ::__spec_test::end
        return
    }
    set ::__spec_test::snapshot_ready 1
}
proc ::__spec_test::wrote {coordinate name1 name2 op} {
    lappend ::__spec_test::writes $coordinate
    if {$op eq "unset"} {
        if {[catch {::__tcl_name_snapshot::call $coordinate {::array exists}} present] || !$present} {
            catch {unset ::__spec_test::traced($coordinate)}
        }
    }
}
proc ::__spec_test::watch {created} {
    foreach coordinate [::__spec_test::variables] {
        if {[info exists ::__spec_test::traced($coordinate)]} continue
        set prefix [list ::__spec_test::wrote $coordinate]
        ::__tcl_name_snapshot::call $coordinate {::trace add variable} [list {write unset} $prefix]
        set ::__spec_test::traced($coordinate) $prefix
        if {$created} {lappend ::__spec_test::writes $coordinate}
    }
}
proc ::__spec_test::step {args} {
    if {!$::__spec_test::snapshot_ready} return
    if {[catch {::__spec_test::watch 1} reason]} {
        set ::__spec_test::snapshot_ready 0
        set ::__spec_test::snapshot_reason $reason
    }
}
proc ::__spec_test::end {} {
    foreach helper {attempt example reference pure} {
        catch {trace remove execution ::__spec_test::$helper leavestep ::__spec_test::step}
    }
    if {![array exists ::__spec_test::traced]} return
    foreach coordinate [array names ::__spec_test::traced] {
        catch {::__tcl_name_snapshot::call $coordinate {::trace remove variable} [list {write unset} $::__spec_test::traced($coordinate)]}
    }
}
proc ::__spec_test::pure {name lines} {
    if {$::__spec_test::skip} return
    if {!$::__spec_test::snapshot_ready} {
        ::__spec_test::end
        ::__spec_test::say unsupported $name "purity snapshot unavailable: $::__spec_test::snapshot_reason"
        return
    }
    set earlier $::__spec_test::writes
    set reported {}
    foreach line $lines {
        set ::__spec_test::writes {}
        catch {uplevel #0 $line}
        set names {}
        foreach written [lsort -unique $::__spec_test::writes] {lappend names [::__tcl_name_snapshot::shown $written]}
        if {[llength $names] > 0} {
            eval lappend reported $names
            ::__spec_test::say pure $name "declared pure, but `$line` wrote the variable(s) $names"
        }
    }
    ::__spec_test::end
    if {!$::__spec_test::snapshot_ready} {
        ::__spec_test::say unsupported $name "purity snapshot unavailable: $::__spec_test::snapshot_reason"
        return
    }
    if {[catch {::__spec_test::snapshot} after]} {
        ::__spec_test::say unsupported $name "purity snapshot unavailable: $after"
        return
    }
    set names {}
    set changed [::__spec_test::changed $::__spec_test::mark $after]
    foreach coordinate [lsort -unique [concat $earlier $changed]] {
        set shown [::__tcl_name_snapshot::shown $coordinate]
        if {[lsearch -exact $reported $shown] < 0} {lappend names $shown}
    }
    if {[llength $names] > 0} {
        ::__spec_test::say pure $name "declared pure, but running it created or changed the variable(s) $names"
    }
}

::__spec_test::present {::N[0]::p} {::N[0]::p} {::N\[0\]::p} 1
::__spec_test::asked {::N[0]::p}
::__spec_test::present {::W*::p} {::W*::p} {::W\*::p} 1
::__spec_test::asked {::W*::p}
::__spec_test::present {::Q?::p} {::Q?::p} {::Q\?::p} 1
::__spec_test::asked {::Q?::p}
::__spec_test::present {::B\slash::p} {::B\slash::p} {::B\\slash::p} 1
::__spec_test::asked {::B\slash::p}
::__spec_test::present {::N[0]::p*} {::N[0]::p\*} {::N\[0\]::p\*} 1
::__spec_test::asked {::N[0]::p*}
::__spec_test::present {::N[0]::p?} {::N[0]::p\?} {::N\[0\]::p\?} 1
::__spec_test::asked {::N[0]::p?}
::__spec_test::present {::N[0]::p[0]} {::N[0]::p\[0\]} {::N\[0\]::p\[0\]} 1
::__spec_test::asked {::N[0]::p[0]}
::__spec_test::present {::N[0]::p\tail} {::N[0]::p\\tail} {::N\[0\]::p\\tail} 1
::__spec_test::asked {::N[0]::p\tail}
::__spec_test::present {::N[0]::} {::N[0]::} {::N\[0\]::} 1
::__spec_test::asked {::N[0]::}
::__spec_test::present {:p} {:p} {:p} 1
::__spec_test::asked {:p}
::__spec_test::present {p*} {p\*} {p\*} 1
::__spec_test::asked {p*}
::__spec_test::present {::N[missing]::absent} {::N[missing]::absent} {::N\[missing\]::absent} 1
::__spec_test::asked {::N[missing]::absent}
::__spec_test::present {::N[0]::absent*} {::N[0]::absent\*} {::N\[0\]::absent\*} 1
::__spec_test::asked {::N[0]::absent*}
::__spec_test::present {absent*} {absent\*} {absent\*} 1
::__spec_test::asked {absent*}
::__spec_test::finish
