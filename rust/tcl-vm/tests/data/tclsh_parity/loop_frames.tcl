# The errorInfo an error in a loop or control body leaves: a body the
# release compiles adds no frame of its command's own, one it runs does.
proc e {} { error boom }
proc info_of {script} {
    catch {uplevel #0 $script}
    string map [list \n { | }] $::errorInfo
}
proc p1 {} { foreach x {1} { e } }
proc p2 {} { while 1 { e } }
proc p3 {} { for {} 1 {} { e } }
proc p4 {} { for {e} 1 {} {} }
proc p5 {} { for {} 1 {e} {} }
proc p6 {} { if 1 { e } }
proc p7 {} { foreach x {1} { foreach y {2} { e } } }
proc p8 {} { foreach x {1} { while 1 { e } } }
proc p9 {} { while 1 { foreach y {2} { e } } }
proc p10 {} { catch { foreach y {2} { e } } m; error $::errorInfo }
proc p11 {} { set l [foreach x {1} { e }] }
proc p12 {} { namespace eval n { foreach x {1} { e } } }
proc p13 {} { set f e; catch $f; error $::errorInfo }
proc p14 {} { dict for {k v} {a b} { e } }
proc p15 {} { dict for {k v} {a b} { if 1 { e } } }
proc p16 {} { lmap x {1} { e } }
proc p17 {} { eval { foreach x {1} { e } } }
foreach p {p1 p2 p3 p4 p5 p6 p7 p8 p9 p10 p11 p12 p13 p14 p15 p16 p17} {
    puts "$p: [info_of $p]"
}
puts [info_of {foreach x {1} { e }}]
puts [info_of {while 1 { e }}]
puts [info_of {for {} 1 {e} {}}]
puts [info_of {if 1 { e }}]
puts [info_of {dict for {k v} {a b} { e }}]
puts [info_of {namespace eval n { if 1 { e } }}]
puts [info_of {namespace inscope n { if 1 { e } }}]
puts [info_of {eval { foreach x {1} { e } }}]
puts [info_of {set f e; catch $f; error $::errorInfo}]
foreach x {1} {
    while 1 {
        if 1 {
            e
        }
    }
}
