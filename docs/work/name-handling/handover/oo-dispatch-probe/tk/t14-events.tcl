package require Tk
event add <<Go>> <Control-g>
bind . <<Go>> {puts go}
bind all <Control-q> {puts quit}
bindtags .  [linsert [bindtags .] 0 MyTag]
bind MyTag <Key> {puts key}
puts [catch {event add Go <Control-g>} m]:$m ;# BAD virtual event name
