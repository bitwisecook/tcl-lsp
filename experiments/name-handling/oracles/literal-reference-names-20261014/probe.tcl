set "x" "PLAIN"
set "{x}" "WRAPPED"
set "{x" "FIRST"
set "cash\$name" "CASH"
set "arr(k)" "ELEMENT"
set "" "EMPTY"
set code [catch "list \${{x}}" value]
puts [list brace_name $code $value]
set code [catch "list \${cash\$name}" value]
puts [list sigil_name $code $value]
set code [catch "list \${arr(k)}" value]
puts [list combined_element $code $value]
set code [catch "list \$arr(k)" value]
puts [list separate_element $code $value]
set code [catch "list \${}" value]
puts [list empty_name $code $value]
