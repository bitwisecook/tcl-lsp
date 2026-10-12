package require tcltest
proc p {} {offset-lifecycle run n d -cleanup {puts CLEANUP} -body {puts BODY} -setup {puts SETUP}}