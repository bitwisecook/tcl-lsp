package require tcltest
namespace import -force ::tcltest::*
test MpfitClassTest-9 {test procedure of optimization errors handling} -constraints {isInterface isMpfit} -setup {
    set x [list -1.7237128 1.8712276 -9.6608055E-01 -2.8394297E-01 1.3416969 1.3757038 -1.3703436 4.2581975E-02\
                   -1.4970151E-01 8.2065094E-01]
    set y [list 1.9000429e-01 6.5807428 1.4582725 2.7270851 5.5969253 5.6249280 0.787615 3.2599759 2.9771762 4.5936475]
    set ey [list 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07]
    set pdata [dict create x $x y $y ey $ey]
} -body {
    catch {set optimizer [Mpfit new -funct {} -m 10 -pdata $pdata]} errorStr
    return $errorStr
} -result {Function must have a name, empty string was provided} -cleanup {
    unset x y ey pdata errorStr
}
