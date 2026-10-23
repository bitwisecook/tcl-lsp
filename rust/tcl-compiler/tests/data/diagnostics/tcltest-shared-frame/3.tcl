package require tcltest
namespace import -force ::tcltest::*
test MpfitClassTest-18 {test procedure of optimization of linear function with -nofinitecheck} -match approxEqual\
        -constraints {isOptimization isMpfit} -setup {
    set x [list -1.7237128 1.8712276 -9.6608055E-01 -2.8394297E-01 1.3416969 1.3757038 -1.3703436 4.2581975E-02\
                   -1.4970151E-01 8.2065094E-01]
    set y [list 1.9000429e-01 6.5807428 1.4582725 2.7270851 5.5969253 5.6249280 0.787615 3.2599759 2.9771762 4.5936475]
    set ey [list 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07 0.07]
    set pdata [dict create x $x y $y ey $ey]
    set par0 [ParameterMpfit new a 0.5]
    set par1 [ParameterMpfit new b 1.0]
    set optimizer [Mpfit new -funct linfunc -m 10 -pdata $pdata -nofinitecheck]
    $optimizer addPars $par0 $par1
} -body {
    $optimizer run
} -result {bestnorm 2.756284982812975 orignorm 17475.81280510226 status {Convergence in chi-square value} niter 3 nfev\
8 npar 2 nfree 2 npegged 0 nfunc 10 resid {0.4665000063092644 0.8131242947498356 -0.5829829541606824\
0.28527684418915605 0.15536884646007362 -0.30494490319056106 0.06378629427930944 -0.3628649219779361\
0.46178573329514544 -0.9950492584261663} xerror {0.022210176319442228 0.018937556548612428} x {3.2099657169532576\
1.7709542026925287} debug {} covar {0.0004932919321407123 -3.4359715519660666e-5 -3.4359715519660666e-5\
0.00035863104803189345}} -cleanup {
    objsDestroy $par0 $par1 $optimizer
    unset x y ey pdata
}
