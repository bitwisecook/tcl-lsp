proc receipt {label body} {
    set code [catch {uplevel 1 $body} result]
    binary scan $result H* hex
    puts [list $label $code $hex]
}
proc observer {args} {}
array set a {k00 V k01 V k02 V k03 V k04 V k05 V k06 V k07 V k08 V k09 V k10 V k11 V k12 V}
if {[catch {trace add variable a(shell) read observer}]} {trace variable a(shell) r observer}
set s [array startsearch a]
receipt initial-id {set s}
if {[catch {trace remove variable a(shell) read observer}]} {trace vdelete a(shell) r observer}
receipt undefined-shell-removal-anymore {array anymore a $s}
receipt undefined-shell-removal-next {array nextelement a $s}
receipt unsigned-leading-zero {array anymore a s-01-a}
receipt signed-leading-plus {array anymore a s-+1-a}
receipt wrong-variable {array anymore a s-1-other}
set a(k00) OTHER
receipt define-existing-keeps-search {array anymore a $s}
set a(fresh) V
receipt new-key-invalidates {array anymore a $s}
set s [array startsearch a]
receipt restarted-id {set s}
set t [array startsearch a]
receipt concurrent-id {set t}
array donesearch a $t
set t [array startsearch a]
receipt deleted-top-id-reused {set t}
receipt long-overflow-identifier {array anymore a s-18446744073709551617-a}
array donesearch a $s
receipt done-search-miss {array anymore a $s}
