# Tk's globals exist only after Tk is loaded (needs a display).
puts "tk_version=[info exists tk_version] tk_library=[info exists tk_library]"
puts [list [catch {package require Tk} m]]
