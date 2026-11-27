# The Tcl 8.4.20 `tclInit` search, transcribed from generic/tclInitScript.h.
# Lines marked `tclvm:` differ from the oracle text, for the reason given.
# tclvm: C seeds $tcl_libPath from `TclpInitLibraryPath` (unix/tclUnixInit.c)
# when it creates the interpreter; this computes the same list. In place of the
# compiled-in default library it ends with this release's library in a Tcl
# source tree beside the one $env(TCL_LIBRARY) names (…/tcl8.4.20/library),
# which is where a build-tree tclsh8.4 finds its own.
proc tclvmLibPath {} {
    global env
    set path {}
    if {[info exists env(TCL_LIBRARY)] && $env(TCL_LIBRARY) ne ""} {
	lappend path $env(TCL_LIBRARY)
	set parts [file split $env(TCL_LIBRARY)]
	if {[string tolower [lindex $parts end]] ne "tcl8.4"} {
	    lappend path [eval [linsert [lreplace $parts end end tcl8.4] 0 file join]]
	}
    }
    set exe [info nameofexecutable]
    if {$exe ne ""} {
	set v {}
	foreach part [file split $exe] {
	    switch -- $part {
		. {}
		.. { set v [lreplace $v end end] }
		default { lappend v $part }
	    }
	}
	set n [llength $v]
	set develop tcl[info patchlevel]/library
	foreach {min drop sub} [list 2 2 lib/tcl8.4 3 3 lib/tcl8.4 2 2 library \
		3 3 library 3 3 $develop 4 4 $develop] {
	    if {$n > $min} {
		set kept [lrange $v 0 [expr {$n - $drop - 1}]]
		lappend path [eval [linsert [concat $kept [list $sub]] 0 file join]]
	    }
	}
    }
    if {[info exists env(TCL_LIBRARY)] && [file tail $env(TCL_LIBRARY)] eq "library"} {
	set tree [file dirname $env(TCL_LIBRARY)]
	if {[regexp {^tcl[0-9]} [file tail $tree]]} {
	    lappend path [file join [file dirname $tree] tcl[info patchlevel] library]
	}
    }
    return $path
}
set tcl_libPath [tclvmLibPath]
rename tclvmLibPath {}
if {[info proc tclInit]==""} {
  proc tclInit {} {
    global tcl_libPath tcl_library errorInfo
    global env tclDefaultLibrary
    rename tclInit {}
    set errors {}
    set dirs {}
    if {[info exists tcl_library]} {
	lappend dirs $tcl_library
    } else {
	if {[info exists env(TCL_LIBRARY)]} {
	    lappend dirs $env(TCL_LIBRARY)
	}
	catch {
	    lappend dirs $tclDefaultLibrary
	    unset tclDefaultLibrary
	}
        set dirs [concat $dirs $tcl_libPath]
    }
    foreach i $dirs {
	set tcl_library $i
	set tclfile [file join $i init.tcl]
	if {[file exists $tclfile]} {
	    if {![catch {uplevel #0 [list source $tclfile]} msg]} {
		return
	    } else {
		append errors "$tclfile: $msg
$errorInfo
"
	    }
	}
    }
    set msg "Can't find a usable init.tcl in the following directories: 
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
