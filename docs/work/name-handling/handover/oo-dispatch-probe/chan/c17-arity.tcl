# wrong-argument messages for the legacy commands and chan subcommands
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
foreach s {{puts a b c d} {gets} {read} {close a b c} {seek} {tell} {eof} {flush} {fconfigure} {fileevent} {fcopy a} {fblocked} {open} {chan close} {chan puts a b c d} {chan configure} {chan create read} {chan push a} {chan pop} {chan pipe x} {chan names a b} {chan postevent a} {chan event a} {chan copy a} {chan truncate} {chan pending a} {chan blocked}} {
  t $s $s
}
