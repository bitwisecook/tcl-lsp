# handler protocol errors: missing required methods, bad mode
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
proc noread {cmd args} { if {$cmd eq "initialize"} { return {initialize finalize watch} } }
proc badinit {cmd args} { if {$cmd eq "initialize"} { return {initialize read} } }
proc ok {cmd args} { if {$cmd eq "initialize"} { return {initialize finalize watch read} } ; return "" }
t no-read {chan create read noread}
t missing {chan create read badinit}
t bad-mode {chan create sideways ok}
t unknown-cmd {chan create read nosuchhandler}
