namespace eval :: {set ephemeral /LOCAL; puts "in:$ephemeral"}
puts "out:[info exists ::ephemeral]"
