# -buffering/-buffersize values and errors, -blocking
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
set f [open c20.txt w]
t buffering {list [fconfigure $f -buffering] [fconfigure $f -buffersize] [fconfigure $f -blocking]}
t set-line {fconfigure $f -buffering line; fconfigure $f -buffering}
t bad-buf {fconfigure $f -buffering sideways}
t bad-size {fconfigure $f -buffersize abc}
t huge-size {fconfigure $f -buffersize 99999999; fconfigure $f -buffersize}
t bad-trans {fconfigure $f -translation sideways}
t pair-trans {fconfigure $f -translation {lf crlf}; fconfigure $f -translation}
t bad-enc {fconfigure $f -encoding nosuch-enc}
close $f
