# Thread package (not bundled with these tclsh builds; thread 3.0.4 for Tcl 9
# comes with the Google Cloud SDK). Run with the SDK lib on auto_path.
if {[info exists ::env(TCL_LSP_PROBE_PACKAGES)]} {lappend auto_path $::env(TCL_LSP_PROBE_PACKAGES)}
if {[catch {package require Thread} v]} { puts "no-thread: $v"; return }
puts "loaded=[package vcompare $v 2.0]"
set t [thread::create {thread::wait}]
puts "exists=[thread::exists $t] self-differs=[expr {$t ne [thread::id]}]"
puts "send=[thread::send $t {expr {6 * 7}}]"
thread::send -async $t {expr {1 + 1}} ::res
vwait ::res
puts "async=$::res"
puts "names>=2=[expr {[llength [thread::names]] >= 2}]"
thread::release $t
