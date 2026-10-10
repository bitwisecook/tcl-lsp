puts "VERSION|[info patchlevel]"
set rows {}
set a(k) OLD
lappend rows [list builtin [array get a]]
rename array array_old
lappend rows [list renamed [array_old get a]]
set alias_setup [catch {interp alias {} array_alias {} array_old} alias_setup_result]
lappend rows [list alias_setup $alias_setup $alias_setup_result]
if {$alias_setup == 0} {
    set alias_code [catch {array_alias get a} alias_result]
    lappend rows [list alias $alias_code $alias_result]
}
puts $rows
