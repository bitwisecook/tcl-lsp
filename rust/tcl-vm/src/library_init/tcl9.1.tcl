# The Tcl 9.1.0 init.tcl search, transcribed from `LocatePreInitScript` and
# `RunPreInitScript` (generic/tclInterp.c), which 9.1 implements in C: the
# first of $tcl_library and $env(TCL_LIBRARY) that is set and non-empty is the
# only place looked; otherwise $tclDefaultLibrary and <exe>/../lib/tcl9.1. An
# init.tcl that is found is sourced once, and its failure is final.
# tclvm: no configure-time or build-tree candidates; the zipfs attachment is
# the application hook below.
# tclvm: tclsh 9 attaches its own script library, and its application hook
# (TclZipfs_AppHook) sets $tcl_library to it before Tcl_Init runs. tclvm's
# counterpart is this release's library in a Tcl source tree beside the one
# $env(TCL_LIBRARY) names (…/tcl<patchlevel>/library).
if {![info exists ::tcl_library] && [info exists ::env(TCL_LIBRARY)]
	&& [file tail $::env(TCL_LIBRARY)] eq "library"
	&& [regexp {^tcl[0-9]} [file tail [file dirname $::env(TCL_LIBRARY)]]]
	&& [file readable [file join [file dirname [file dirname $::env(TCL_LIBRARY)]] \
		tcl[info patchlevel] library init.tcl]]} {
    set ::tcl_library [file join [file dirname [file dirname $::env(TCL_LIBRARY)]] \
	    tcl[info patchlevel] library]
}
namespace eval ::tcl {
    variable InitSearched {}
    variable InitDir {}
    variable InitStop 0
    foreach {InitVar InitFinal} {::tcl_library 1 ::env(TCL_LIBRARY) 1 ::tclDefaultLibrary 0} {
	if {![info exists $InitVar]} continue
	set InitCandidate [set $InitVar]
	lappend InitSearched $InitCandidate
	if {$InitCandidate eq {}} continue
	if {[file readable [file join $InitCandidate init.tcl]]} {
	    set InitDir $InitCandidate
	}
	if {$InitDir ne {} || $InitFinal} {
	    set InitStop 1
	    break
	}
    }
    if {!$InitStop && [info nameofexecutable] ne {}} {
	set InitCandidate [file join [file dirname [file dirname \
		[info nameofexecutable]]] lib tcl[info tclversion]]
	lappend InitSearched $InitCandidate
	if {[file readable [file join $InitCandidate init.tcl]]} {
	    set InitDir $InitCandidate
	}
    }
    set InitMessage "Cannot find a usable init.tcl in the following directories: \n\
	    \   $InitSearched\n\nThis probably means that Tcl wasn't installed properly.\n"
    unset -nocomplain InitSearched InitStop InitVar InitFinal InitCandidate
}
if {$::tcl::InitDir eq {}} {
    unset ::tcl::InitDir
    # tclvm: a search that found nothing says so in errorCode, so a host can
    # tell a missing library from a broken one.
    error $::tcl::InitMessage[unset ::tcl::InitMessage] {} {TCLVM LIBRARY ABSENT}
}
set ::tcl_library $::tcl::InitDir
unset ::tcl::InitDir ::tcl::InitMessage
if {[catch {uplevel #0 [list source [file join $::tcl_library init.tcl]]} ::tcl::InitError]} {
    error "Error sourcing Tcl initialization script from [file join $::tcl_library\
	    init.tcl]:\n$::tcl::InitError[unset ::tcl::InitError]"
}
unset ::tcl::InitError
