set out {}

proc W {n1 n2 op} {lappend ::events [list whole $n1 $n2 $op]}
proc MX {n1 n2 op} {
    lappend ::events [list elem $n1 $n2 $op]
    set ::A($n2) traced
    set ::probe $::A($n2)
    set ::A(y) NEW
}
set events {}
array set A {x X}
trace add variable A read W
trace add variable A(x) read MX
lappend out [list basic [array get A] $events $probe [lsort [array names A]]]

proc DROP {n1 n2 op} {unset ::B($n2)}
array set B {x X}
trace add variable B(x) read DROP
set cb [catch {array get B} mb ob]
lappend out [list missing $cb $mb [dict get $ob -errorcode] \
    [info exists ::errorInfo] [info exists ::errorCode]]

proc DESTROY {n1 n2 op} {unset ::C}
array set C {x X}
trace add variable C(x) read DESTROY
set cc [catch {array get C} mc oc]
lappend out [list destroy $cc $mc [dict get $oc -errorcode]]

proc RETYPE {n1 n2 op} {unset ::D; set ::D scalar}
array set D {x X}
trace add variable D(x) read RETYPE
set cd [catch {array get D} md od]
lappend out [list retype $cd $md [dict get $od -errorcode] $D]

proc BOOM {n1 n2 op} {error BOOM}
array set E {x X}
trace add variable E(x) read BOOM
set ce [catch {array get E} me oe]
lappend out [list boom $ce $me [dict get $oe -errorcode] \
    [string match {*read trace on "E(x)"*} [dict get $oe -errorinfo]] \
    [dict get $oe -errorline] \
    [string match {*read trace on "E(x)"*} $::errorInfo] $::errorCode]
set cleak [catch {array get E; list after} mleak oleak]
lappend out [list carried $cleak $mleak [dict exists $oleak -errorcode] \
    [string match {*read trace on "E(x)"*} $::errorInfo] $::errorCode]

proc HARDD {n1 n2 op} {unset ::J; error BOOM}
array set J {x X}
trace add variable J(x) read HARDD
set cj [catch {array get J} mj oj]
lappend out [list harddestroy $cj $mj [dict get $oj -errorcode] \
    [string match {*read trace on "J(x)"*} [dict get $oj -errorinfo]] \
    [dict get $oj -errorline] [array exists J]]

proc HARDR {n1 n2 op} {unset ::JR; set ::JR scalar; error BOOM}
array set JR {x X}
trace add variable JR(x) read HARDR
set cjr [catch {array get JR} mjr ojr]
lappend out [list hardretype $cjr $mjr [dict get $ojr -errorcode] \
    [string match {*read trace on "JR(x)"*} [dict get $ojr -errorinfo]] \
    [dict get $ojr -errorline] $JR]

proc MIX {n1 n2 op} {
    incr ::mix
    if {$::mix == 1} {error FIRST}
    unset ::K($n2)
}
set mix 0
array set K {a A b B}
trace add variable K read MIX
set ck [catch {array get K} mk ok]
lappend out [list aggregate $ck $mk [dict get $ok -errorcode] \
    [string match {*FIRST*read trace on "K(*} [dict get $ok -errorinfo]] \
    [dict get $ok -errorline] \
    [string match {*FIRST*read trace on "K(*} $::errorInfo] $::errorCode]

proc OLD {n1 n2 op} {lappend ::live_events old}
proc NEW {n1 n2 op} {lappend ::live_events new}
proc CHANGE {n1 n2 op} {
    lappend ::live_events whole
    set variable ::L($n2)
    trace remove variable $variable read OLD
    trace add variable $variable read NEW
}
set live_events {}
array set L {x X}
trace add variable L(x) read OLD
trace add variable L read CHANGE
lappend out [list livegroup [array get L] $live_events]

proc QPINA {n1 n2 op} {
    unset ::Q(b)
    set ::trashA [string repeat z 400]
    set ::Q(b) overwritten
}
proc QPINB {n1 n2 op} {
    unset ::Q(a)
    set ::trashB [string repeat z 400]
    set ::Q(a) overwritten
}
array set Q [list a [string repeat A 200] b [string repeat B 200]]
trace add variable Q(a) read QPINA
trace add variable Q(b) read QPINB
set qr [array get Q]
lappend out [list owned [lsort -integer [list \
    [string length [dict get $qr a]] [string length [dict get $qr b]]]]]

proc RELEM {n1 n2 op} {unset ::F($n2); set ::F($n2) NEW}
array set F {x X}
trace add variable F(x) read RELEM
set cf [catch {array get F} mf of]
lappend out [list relem $cf $mf [dict exists $of -errorcode] [array get F]]

proc RBASE {n1 n2 op} {unset ::G; array set ::G {x NEW}}
array set G {x X}
trace add variable G(x) read RBASE
set cg [catch {array get G} mg og]
lappend out [list rbase $cg $mg [dict get $og -errorcode] [array get G]]

proc AROP args {uplevel 1 {upvar #0 ::IB alias}}
array set IA {x A}
array set IB {x B}
proc PA {} {
    upvar #0 ::IA alias
    trace add variable alias array AROP
    array get alias
}
lappend out [list opretarget [PA]]

proc ERET {n1 n2 op} {uplevel 1 {upvar #0 ::JB alias}}
array set JA {x A}
array set JB {x B}
proc PE {} {
    upvar #0 ::JA alias
    trace add variable ::JA(x) read ERET
    list [array get alias] [array get ::JB]
}
lappend out [list elemretarget [PE]]

proc UR {n1 n2 op} {
    set ::u_seen [list $n1 $n2 $op]
    set name {}
    append name $n1 ( $n2 )
    uplevel 1 [list set $name traced]
}
proc UGET {} {uplevel 1 {array get local}}
proc UP {} {
    array set local {x X}
    trace add variable local(x) read UR
    UGET
}
lappend out [list uplevel [UP] $u_seen]

namespace eval ::N {
    array set a {x X}
    trace add variable a(x) read ::UR
    set nr [array get a]
}
lappend out [list namespace $::N::nr $u_seen]

proc MUT {n1 n2 op} {upvar #0 $n1 v; set v($n2) traced}
array set H {x X}
trace add variable H(x) read MUT
array set I {x X}
trace add variable I(x) read MUT
set cmd array
lappend out [list calls [array get H] [$cmd get I]]

set out
