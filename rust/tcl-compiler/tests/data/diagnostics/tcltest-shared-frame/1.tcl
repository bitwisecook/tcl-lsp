package require tcltest
namespace import -force ::tcltest::*
test MpfitClassTest-1 {test procedure of optimization of linear function without constraints} -match approxEqual\
        -constraints {isOptimization isMpfit} -setup {
    set x [list -1.7237128 1.8712276 -9.6608055E-01 -2.8394297E-01 1.3416969 1.3757038 -1.3703436 4.2581975E-02\
                   -1.4970151E-01 8.2065094E-01]
    set y [list 1.9000429e-01 6.5807428 1.4582725 2.7270851 5.5969253 5.6249280 0.787615 3.2599759 2.9771762 4.5936475]
    set ey [list 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07]
    set pdata [dict create x $x y $y ey $ey]
    set par0 [ParameterMpfit new a 1.0]
    set par1 [ParameterMpfit new b 1.0]
    set optimizer [Mpfit new -funct linfunc -m 10 -pdata $pdata]
    $optimizer addPars $par0 $par1
} -body {
    $optimizer run
} -result {bestnorm 2.756284982812983 orignorm 12304.732640816545 status {Convergence in chi-square value} niter 3 nfev\
8 npar 2 nfree 2 npegged 0 nfunc 10 resid {0.4665000078783141 0.8131242968301778 -0.5829829524838808 0.2852768459629816\
0.15536884846509832 -0.3049449011807021 0.06378629589861731 -0.36286492015767785 0.4617857350880605 -0.995049256495241}\
xerror {0.02221017630949747 0.018937556387763524} x {3.209965716826263 1.7709542026825729} debug {} covar\
{0.0004932919316989627 -3.4359717592592004e-5 -3.4359717592592004e-5 0.000358631041939723}} -cleanup {
    objsDestroy $par0 $par1 $optimizer
    unset x y ey pdata optimizer par0 par1
}
