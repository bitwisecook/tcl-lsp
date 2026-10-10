puts [eval {set a OTHER;list [catch {array names a}] [catch {array get a}] [array exists a]}]
