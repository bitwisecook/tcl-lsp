# Error handling
set code [catch {expr {1 / 0}} result]
set code2 [catch {set undefined_var_xyz} result2]
set code3 [catch {expr {2 + 3}} result3]

if {$code != 0} {
    set handled "caught division"
}
if {$code2 != 0} {
    set handled2 "caught undefined"
}
