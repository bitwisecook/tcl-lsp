# Startup values that do not depend on the host.
puts "version=$tcl_version"
puts "patch=$tcl_patchLevel"
puts "argc=$argc argv={$argv} argv0=[file tail $argv0]"
puts "interactive=$tcl_interactive"
puts "rc=[expr {[info exists tcl_rcFileName] ? [file tail $tcl_rcFileName] : {<none>}}]"
puts "library-dir=[file isdirectory $tcl_library]"
puts "auto_path-list=[expr {[llength $auto_path] > 0}]"
puts "pkgPath=[expr {[info exists tcl_pkgPath] ? [llength $tcl_pkgPath] : {<none>}}]"
