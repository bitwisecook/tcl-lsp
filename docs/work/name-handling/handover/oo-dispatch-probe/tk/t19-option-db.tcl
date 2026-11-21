package require Tk
option add *Button.background grey widgetDefault
button .b
puts [.b cget -background]
