namespace eval a:::b {variable dir /LIB}
puts "home:[set ::a:::b::dir]"
puts "normal:[info exists ::a::b::dir]"
