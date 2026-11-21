package require Tk
set img [image create photo -width 10 -height 10]
$img put red -to 0 0 10 10
label .l -image $img
font create Big -family Helvetica -size 18 -weight bold
label .l2 -font Big -text big
label .l3 -font {Helvetica 12 bold}
puts [catch {image create foto} m]:$m ;# BAD type
puts [catch {font create F -wieght bold} m]:$m ;# BAD option
