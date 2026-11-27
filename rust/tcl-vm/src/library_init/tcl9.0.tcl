# The Tcl 9.0.4 `tclInit` search, transcribed from generic/tclInterp.c.
# Lines marked `tclvm:` differ from the oracle text, for the reason given.
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
if {[namespace which -command tclInit] eq ""} {
  proc tclInit {} {
    global tcl_libPath tcl_library env tclDefaultLibrary
    rename tclInit {}
    if {[info exists tcl_library]} {
	set scripts {{set tcl_library}}
    } else {
	set scripts {}
	if {[info exists env(TCL_LIBRARY)] && ($env(TCL_LIBRARY) ne {})} {
	    lappend scripts {set env(TCL_LIBRARY)}
	    lappend scripts {
if {[regexp ^tcl(.*)$ [file tail $env(TCL_LIBRARY)] -> tail] == 0} continue
if {$tail eq [info tclversion]} continue
file join [file dirname $env(TCL_LIBRARY)] tcl[info tclversion]}
	}
	# tclvm: no zipfs; the attached library is the application hook above.
	# tclvm: no configure-time script directory, so no
	# `::tcl::pkgconfig get scriptdir,runtime` candidate (the attached
	# library is the application hook above).
	if {[info exists tclDefaultLibrary]} {
	    lappend scripts {set tclDefaultLibrary}
	}
	lappend scripts {
set parentDir [file dirname [file dirname [info nameofexecutable]]]
set grandParentDir [file dirname $parentDir]
file join $parentDir lib tcl[info tclversion]} \
	{file join $grandParentDir lib tcl[info tclversion]} \
	{file join $parentDir library} \
	{file join $grandParentDir library} \
	{file join $grandParentDir tcl[info tclversion] library} \
	{file join $grandParentDir tcl[info patchlevel] library} \
	{
file join [file dirname $grandParentDir] tcl[info patchlevel] library}
	if {[info exists tcl_libPath]
		&& [catch {llength $tcl_libPath} len] == 0} {
	    for {set i 0} {$i < $len} {incr i} {
		lappend scripts [list lindex \$tcl_libPath $i]
	    }
	}
    }
    set dirs {}
    set errors {}
    foreach script $scripts {
	if {[set tcl_library [eval $script]] eq ""} continue
	set tclfile [file join $tcl_library init.tcl]
	if {[file exists $tclfile]} {
	    if {[catch {uplevel #0 [list source $tclfile]} msg opts]} {
		append errors "$tclfile: $msg
"
		append errors "[dict get $opts -errorinfo]
"
		continue
	    }
	    unset -nocomplain tclDefaultLibrary
	    return
	}
	lappend dirs $tcl_library
    }
    unset -nocomplain tclDefaultLibrary
    set msg "Cannot find a usable init.tcl in the following directories: 
"
    append msg "    $dirs

"
    append msg "$errors

"
    append msg "This probably means that Tcl wasn't installed properly.
"
    # tclvm: a search that sourced nothing says so in errorCode, so a host
    # can tell a missing library from a broken one.
    if {$errors eq ""} {
	error $msg {} {TCLVM LIBRARY ABSENT}
    }
    error $msg
  }
}
tclInit
