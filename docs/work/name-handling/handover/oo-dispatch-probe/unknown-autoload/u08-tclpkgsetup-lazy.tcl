# pkgIndex.tcl with tclPkgSetup: package require declares commands; the file is sourced on first use.
file mkdir pk
set f [open pk/foo.tcl w]
puts $f {namespace eval foo { proc hello {} { return "hello from foo" } }; package provide foo 1.0; set ::foo_sourced 1}
close $f
set f [open pk/pkgIndex.tcl w]
puts $f {package ifneeded foo 1.0 [list tclPkgSetup $dir foo 1.0 {{foo.tcl source {foo::hello}}}]}
close $f
lappend auto_path [file join [pwd] pk]
puts "require: [package require foo]"
puts "sourced after require: [info exists ::foo_sourced]"
puts "auto_index: [info exists auto_index(foo::hello)] [info exists auto_index(::foo::hello)]"
puts [foo::hello]
puts "sourced after call: [info exists ::foo_sourced]"
file delete -force pk
