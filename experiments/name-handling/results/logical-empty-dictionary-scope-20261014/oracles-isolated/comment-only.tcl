puts [list VERSION [info patchlevel]]
set d {commented 13}
dict with d {# lexical comment only
}
puts [list RESULT [set d] [set commented]]
