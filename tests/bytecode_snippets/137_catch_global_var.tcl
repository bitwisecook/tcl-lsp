set ::foo 1
proc test {} {
    catch {set x 3} ::foo
}
