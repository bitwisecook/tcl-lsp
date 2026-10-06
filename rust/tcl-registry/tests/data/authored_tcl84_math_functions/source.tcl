namespace eval ::tcl::mathfunc {proc abs args {return 999}}
set ::effect NONE
set code [catch {expr {abs(077)}} value]
puts [list abs-octal $code $value $::effect]
set ::effect NONE
set code [catch {expr {sqrt(9)}} value]
puts [list sqrt $code $value $::effect]
set ::effect NONE
set code [catch {expr {pow(2,3)}} value]
puts [list pow $code $value $::effect]
set ::effect NONE
set code [catch {expr {int(4294967297)}} value]
puts [list int-window $code $value $::effect]
set ::effect NONE
set code [catch {expr {wide(4294967297)}} value]
puts [list wide $code $value $::effect]
set ::effect NONE
set code [catch {expr {double(077)}} value]
puts [list double $code $value $::effect]
set ::effect NONE
set code [catch {expr {round(2.5)}} value]
puts [list round $code $value $::effect]
set ::effect NONE
set code [catch {expr {sqrt(-1)}} value]
puts [list domain $code $value $::effect]
set ::effect NONE
set code [catch {expr {future([set ::effect ENTERED])}} value]
puts [list unknown $code $value $::effect]
set ::effect NONE
set code [catch {expr {pow([set ::effect ENTERED])}} value]
puts [list few $code $value $::effect]
set ::effect NONE
set code [catch {expr {abs([set ::effect FIRST],[set ::effect SECOND])}} value]
puts [list many $code $value $::effect]
set ::effect NONE
set code [catch {expr {abs([set ::effect -7])}} value]
puts [list effect $code $value $::effect]
set ::effect NONE
set code [catch {expr {0 && abs([set ::effect ENTERED])}} value]
puts [list lazy $code $value $::effect]
set ::effect NONE
set code [catch {expr {0 && future([set ::effect ENTERED])}} value]
puts [list lazy-unknown $code $value $::effect]
set ::effect NONE
set code [catch {expr {entier(2.0)}} value]
puts [list newer $code $value $::effect]
set ::effect NONE
set code [catch {expr {abs(-4)}} value]
puts [list renamed-public $code $value $::effect]
