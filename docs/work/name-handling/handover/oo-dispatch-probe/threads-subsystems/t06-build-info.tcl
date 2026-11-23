# Version and build information.
puts "patchlevel-matches-provide=[string equal [info patchlevel] [package provide Tcl]]"
puts "tclversion=[info tclversion]"
puts "threaded-defined=[info exists tcl_platform(threaded)]"
set have [llength [info commands ::tcl::build-info]]
puts "build-info-cmd=$have"
if {$have} {
  puts "bi-version=[string equal [tcl::build-info version] [info patchlevel]]"
  puts "bi-patchlevel=[catch {tcl::build-info patchlevel} m] [string equal $m [info patchlevel]]"
  puts "bi-commit-hex=[regexp {^[0-9a-f]+$} [tcl::build-info commit]]"
  puts "bi-threaded=[tcl::build-info threaded]"
  puts "bi-debug=[tcl::build-info debug]"
}
puts "pkg-config=[catch {::tcl::pkgconfig get version} m]"
