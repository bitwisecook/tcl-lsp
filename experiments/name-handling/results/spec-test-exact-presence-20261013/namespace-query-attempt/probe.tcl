package provide demo 1.0
proc :literal {} {}
proc ghostOther {} {}
proc querX {} {}
proc {present*} {} {}
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
if {[catch {package require {demo}} ::__spec_test::loaded]} {
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
proc ::__spec_test::present {name} {
    set ::__spec_test::skip 0
    if {[uplevel #0 [list namespace which -command $name]] ne ""} return
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
    if {[catch {$child eval {package require {demo}}} failure] || [catch {$child eval $text} failure]} {
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
proc ::__spec_test::ours {name} {
    set name [string trimleft $name :]
    return [expr {[string match __spec_test* $name] || [lsearch -exact {errorInfo errorCode} $name] >= 0}]
}
proc ::__spec_test::namespaces {ns} {
    set found [list $ns]
    foreach child [namespace children $ns] {
        if {$child eq "::__spec_test" || $child eq "::tcl"} continue
        foreach inner [::__spec_test::namespaces $child] {lappend found $inner}
    }
    return $found
}
proc ::__spec_test::variables {} {
    set names {}
    foreach ns [::__spec_test::namespaces ::] {
        foreach name [info vars [expr {$ns eq "::" ? "::*" : "${ns}::*"}]] {
            if {![::__spec_test::ours $name]} {lappend names $name}
        }
    }
    return [lsort -unique $names]
}
proc ::__spec_test::shown {name} {
    set bare [string trimleft $name :]
    if {[string first :: $bare] < 0} {return $bare}
    return $name
}
proc ::__spec_test::snapshot {} {
    set state {}
    foreach name [::__spec_test::variables] {
        if {[array exists $name]} {
            set pairs {}
            foreach key [lsort [array names $name]] {lappend pairs $key [set ${name}($key)]}
            lappend state $name [list array $pairs]
        } elseif {[info exists $name]} {
            lappend state $name [list scalar [set $name]]
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
    if {$::__spec_test::skip} return
    set ::__spec_test::mark [::__spec_test::snapshot]
    set ::__spec_test::writes {}
    catch {unset ::__spec_test::traced}
    array set ::__spec_test::traced {}
    ::__spec_test::watch 0
    foreach helper {attempt example reference pure} {
        trace add execution ::__spec_test::$helper leavestep ::__spec_test::step
    }
}
proc ::__spec_test::wrote {name name1 name2 op} {
    lappend ::__spec_test::writes $name
    if {$op eq "unset" && ![array exists $name]} {unset ::__spec_test::traced($name)}
}
proc ::__spec_test::watch {created} {
    foreach qualified [::__spec_test::variables] {
        if {[info exists ::__spec_test::traced($qualified)]} continue
        set prefix [list ::__spec_test::wrote $qualified]
        if {![catch {trace add variable $qualified {write unset} $prefix}]} {
            set ::__spec_test::traced($qualified) $prefix
            if {$created} {lappend ::__spec_test::writes $qualified}
        }
    }
}
proc ::__spec_test::step {args} {
    ::__spec_test::watch 1
}
proc ::__spec_test::end {} {
    foreach helper {attempt example reference pure} {
        trace remove execution ::__spec_test::$helper leavestep ::__spec_test::step
    }
    foreach qualified [array names ::__spec_test::traced] {
        catch {trace remove variable $qualified {write unset} $::__spec_test::traced($qualified)}
    }
}
proc ::__spec_test::pure {name lines} {
    if {$::__spec_test::skip} return
    set earlier $::__spec_test::writes
    set reported {}
    foreach line $lines {
        set ::__spec_test::writes {}
        catch {uplevel #0 $line}
        set names {}
        foreach written [lsort -unique $::__spec_test::writes] {lappend names [::__spec_test::shown $written]}
        if {[llength $names] > 0} {
            eval lappend reported $names
            ::__spec_test::say pure $name "declared pure, but `$line` wrote the variable(s) $names"
        }
    }
    ::__spec_test::end
    set names {}
    set changed [::__spec_test::changed $::__spec_test::mark [::__spec_test::snapshot]]
    foreach qualified [lsort -unique [concat $earlier $changed]] {
        set shown [::__spec_test::shown $qualified]
        if {[lsearch -exact $reported $shown] < 0} {lappend names $shown}
    }
    if {[llength $names] > 0} {
        ::__spec_test::say pure $name "declared pure, but running it created or changed the variable(s) $names"
    }
}

foreach name {:literal ghost* quer? present* ::present* ::::present*} {
 ::__spec_test::present $name
 puts [list PRESENCE $name $::__spec_test::skip]
}
puts [list DIRECT :literal [catch {:literal} result] $result]
