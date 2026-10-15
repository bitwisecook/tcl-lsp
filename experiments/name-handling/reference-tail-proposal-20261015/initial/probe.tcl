set open [format %c 123]
set close [format %c 125]
set sigil [format %c 36]
set namespace_name "ns$open"
namespace eval $namespace_name {}
set old_name "${namespace_name}::x$close"
set first_name "${namespace_name}::x"
set new_name "${namespace_name}::new"
set $old_name OLD_BALANCED
set $first_name OLD_FIRST
set $new_name NEW
set old "$sigil$open$old_name$close"
set proposed "$sigil$open$new_name$close"
foreach label {old proposed} {
    set reference [set $label]
    set code [catch {subst -nocommands -nobackslashes $reference} value]
    puts [list $label $reference [info complete $reference] $code $value]
}
