set dir /GLOBAL
namespace eval N {set dir /LOCAL; puts "local:$dir"; variable linked /LINK; set linked /NEW; puts "link:$linked"}
puts "global:$dir"
puts "unpublished:[info exists ::N::dir]"
puts "published:[set ::N::linked]"
