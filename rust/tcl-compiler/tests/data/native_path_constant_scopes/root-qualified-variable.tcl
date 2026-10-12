namespace eval a {}
variable ::a::x /TOP
puts "flat:[info exists x]"
puts "target:[info exists ::a::x]"
