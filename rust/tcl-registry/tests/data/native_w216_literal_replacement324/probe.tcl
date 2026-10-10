proc check {label root index corrected} {
    set key K
    set intended "${root}($index)"
    set $intended VALUE_$label
    set code [catch $corrected value]
    puts [list $label $code [string equal $value VALUE_$label] [info exists $intended] $value]
}
check dollar {$arr} K {[::set "\$arr($key)"]}
check quote {a"b} K {[::set "a\"b($key)"]}
check brackets {a[b]} K {[::set "a\[b\]($key)"]}
check backslash {a\b} K {[::set "a\\b($key)"]}
check unicode {é} K {[::set "é($key)"]}
check quoted_index {$arr} {K"x"} {[::set "\$arr($key\"x\")"]}
check nested_command {$arr} K {[::set "\$arr([format "%s" $key])"]}
check split_root {a
b} K {[::set "a\nb($key)"]}
set key K
rename ::set ::original_stock_set
proc ::set args {return CHANGED}
puts [list replaced_set [catch {[::set "\$arr($key)"]} value] $value]
