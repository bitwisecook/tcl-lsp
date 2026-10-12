set malformed [format {set value %c} 34]
set complete_bad {error BODY}
foreach body [list $malformed $complete_bad] {
 set definition [catch {proc p {x} $body} value]
 puts [list define $definition $value]
 foreach call {{p} {p OK}} {
  set code [catch {eval $call} value]
  puts [list $call $code $value]
 }
}
proc q {x} {set side BODY;return OK}
set side BEFORE
set code [catch {q} value]
puts [list valid_wrongarity $code $value $side]
