namespace eval ::n {}
set ::shared 1
namespace eval ::n { set shared 2 }
puts $::shared:[info exists ::n::shared]
