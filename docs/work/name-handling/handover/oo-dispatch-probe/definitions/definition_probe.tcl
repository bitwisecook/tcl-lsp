# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

# Tcl 8.4 and Jim Tcl both run this driver. The result file is the protocol:
# some of the probed commands write to stdout themselves.
foreach {entries results lane tkLibrary modules confirmed release} $argv break
set output [open $results w]
set jim [expr {$lane eq "jim"}]

proc fresh {} {
    global jim lane tkLibrary modules
    if {$jim} {
        return [interp]
    }
    set child [interp create]
    if {$lane eq "tk"} {
        $child eval [list load $tkLibrary Tk]
    } elseif {$lane eq "tcl"} {
        foreach package {http msgcat tcltest platform platform::shell opt} {
            catch {$child eval [list package require $package]}
        }
        catch {$child eval {auto_load_index}}
        catch {$child eval {auto_load ::http::formatQuery}}
    } elseif {$lane eq "tcllib"} {
        $child eval [list lappend auto_path $modules]
        $child eval {package require oo::util; package require struct::list}
    }
    return $child
}

proc discard {child} {
    global jim
    if {$jim} {
        $child delete
    } else {
        interp delete $child
    }
}

proc report {args} {
    global output
    puts $output [join $args "\t"]
}

proc classify {code message sub} {
    if {$code != 1} { return ok }
    if {[regexp {^(wrong # args|usage:|value for .* missing:|too many arguments|too few arguments|not enough arguments|Incorrect number of arguments)} $message]} {
        return arity
    }
    if {$sub ne "" && [string first "\"$sub\"" $message] >= 0 &&
        [regexp {^(unknown (or ambiguous )?(subcommand|option|method)|bad (option|subcommand)|ambiguous subcommand)} $message]} {
        return subcommand
    }
    return value
}

proc subcommands {child command} {
    catch {$child eval [list $command __definition_probe__]} message
    if {![regexp {"__definition_probe__": (must be|should be) (.*)$} $message -> verb names]} {
        return {}
    }
    regsub -all {,? or } $names {, } names
    set result {}
    foreach name [split $names ,] {
        set name [string trim $name " ."]
        if {[regexp {^[a-zA-Z0-9_]+$} $name]} { lappend result $name }
    }
    return $result
}

proc probe {child command sub} {
    set words [list $command]
    if {$sub ne ""} { lappend words $sub }
    set args {}
    for {set n 0} {$n <= 12} {incr n} {
        if {$n < 8 || $n == 12} {
            set code [catch {$child eval [concat $words $args]} message]
            set verdict [classify $code $message $sub]
            if {$verdict eq "ok"} { set message "" }
            set message [string map [list "\t" " " "\n" " " "\r" " "] $message]
            report P $command $sub $n $verdict $message
        }
        lappend args __definition_probe__
    }
}

if {$confirmed ne "" && ($lane eq "tcl" || $lane eq "tk")} {
    set input [open $confirmed r]
    while {[gets $input line] >= 0} {
        if {[string index $line 0] eq "#"} { continue }
        foreach {first last expected source} [split $line "\t"] break
        if {[string compare $first $release] > 0 || [string compare $last $release] < 0} { continue }
        set isTk [expr {[string first "package require Tk" $source] == 0}]
        if {$isTk != ($lane eq "tk")} { continue }
        # The a.out helper can launch a toolchain. Its presence and arity
        # descriptor are checked without executing its body.
        if {[string first "tclLdAout" $source] == 0} { continue }
        set control [fresh]
        set source [string map [list {\n} "\n"] $source]
        set code [catch {$control eval $source} message]
        discard $control
        set verdict [classify $code $message ""]
        if {$code == 1} {
            if {[string match {invalid command name*} $message]} {
                set verdict unknown
            } elseif {[string match {unknown method*} $message]} {
                set verdict subcommand
            } elseif {[regexp {^bad (option|switch)} $message]} {
                set verdict option
            }
        }
        if {$expected eq "unavailable"} { set expected unknown }
        if {$verdict ne $expected &&
            !($verdict eq "value" && ($expected eq "ok" || $expected eq "value"))} {
            error [list "confirmed definition changed" $lane $release $line $verdict $message]
        }
    }
    close $input
}

set child [fresh]
set patch [$child eval {info patchlevel}]
if {$lane eq "tk"} { set patch [$child eval {set tk_patchLevel}] }
report V $patch

set commands [$child eval {info commands}]
set namespaces {}
if {$lane eq "tcl"} {
    set namespaces {oo oo::Helpers tcl::mathop tcl::mathfunc tcl::prefix tcl::tm tcl::unsupported http msgcat tcltest platform platform::shell}
    foreach command [$child eval {array names auto_index}] {
        if {![string match tcltest::* [string trimleft $command :]]} {
            lappend commands [string trimleft $command :]
        }
    }
} elseif {$lane eq "tk"} {
    set before [interp create]
    set core [$before eval {info commands}]
    interp delete $before
    set tkCommands {}
    foreach command $commands {
        if {[lsearch -exact $core $command] < 0} { lappend tkCommands $command }
    }
    set commands $tkCommands
    set namespaces {ttk}
} elseif {$lane eq "tcllib"} {
    set commands {struct::list oo::Helpers::classvariable oo::Helpers::link oo::Helpers::mymethod}
}
foreach ns $namespaces {
    set exports {}
    if {[lsearch -exact {http msgcat tcltest platform platform::shell} $ns] >= 0 &&
        [$child eval [list namespace exists ::$ns]]} {
        set exports [$child eval [list namespace eval ::$ns {namespace export}]]
        if {[llength $exports] == 0} { continue }
    }
    foreach command [$child eval [list info commands ::${ns}::*]] {
        if {[llength $exports] == 0} { lappend commands [string trimleft $command :]; continue }
        foreach pattern $exports {
            if {[string match $pattern [namespace tail $command]]} {
                lappend commands [string trimleft $command :]
                break
            }
        }
    }
}

set input [open $entries r]
while {[gets $input line] >= 0} {
    foreach {command sub} [split $line "\t"] break
    set selected([list $command $sub]) 1
    set selected([list $command {}]) 1
}
close $input
foreach command $commands { set selected([list $command {}]) 1 }
if {$lane ne "tk"} { discard $child }

# These need specific inputs or can block, exit or change the probe environment.
set skip {zlib tk_chooseColor tk_chooseDirectory tk_getOpenFile tk_getSaveFile tk_messageBox tk_dialog exit exec vwait tkwait after unknown rename interp history update tclLdAout tcltest::test tcltest::runTests tcltest::cleanupTests tcltest::runAllTests tcltest::main tcltest::ProcessFlags tcltest::processCmdLineArgs http::wait}
foreach key [lsort [array names selected]] {
    foreach {command sub} $key break
    if {$lane ne "tk"} { set child [fresh] }
    set name $command
    if {[string first :: $command] >= 0} { set name ::$command }
    regsub -all {[][?*\\]} $name {\\&} wildcard
    set exists [expr {[lsearch -exact [$child eval [list info commands $wildcard]] $name] >= 0}]
    if {!$exists && $lane eq "tcl"} {
        catch {$child eval [list auto_load $name]} loaded
        set exists [expr {$loaded eq "1"}]
    }
    if {$sub eq ""} { report C $command $exists }
    set public [expr {
        ![string match tcltest::* $command]
        || [lsearch -exact {tcltest::configure tcltest::testConstraint tcltest::match tcltest::skip
            tcltest::constraint tcltest::customMatch tcltest::bytestring} $command] >= 0
    }]
    if {$exists && $public && [lsearch -exact $skip $command] < 0} {
        # An unbounded iota can exhaust memory.
        if {$command ne "struct::list" || $sub ne "iota"} { probe $child $command $sub }
        if {$sub eq "" && $command ne "destroy"} {
            foreach found [subcommands $child $command] {
                if {[info exists selected([list $command $found])]} { continue }
                probe $child $command $found
            }
        }
    }
    if {$lane ne "tk"} { discard $child }
}
report END
close $output
exit
