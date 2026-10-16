# Range and domain errors of every Tcl 8.4 math function: 8.4 checks
# errno and the result (TclExprFloatError), later releases keep the value.
proc show {c} {
    set ::errorCode NONE
    set code [catch {expr $c} r]
    puts "$c -> $code <$r> <$::errorCode>"
}
foreach c {
    {acos(0.5)} {acos(2)} {acos(-2)}
    {asin(0.5)} {asin(2)} {asin(1e-300*1e-20)}
    {atan(1)} {atan(1e-300*1e-20)}
    {atan2(1,2)} {atan2(0,0)} {atan2(0,-1)} {atan2(1,0)}
    {atan2(1e-300,1e300)} {atan2(-1e-300,1e300)} {atan2(1e-300,-1e300)}
    {atan2(1e-300*1e-20,1)}
    {ceil(1.5)} {ceil(-0.5)} {ceil(1e-300*1e-20)}
    {cos(0)} {cosh(1)} {cosh(0)} {cosh(710)} {cosh(1000)} {cosh(-1000)}
    {double(3)} {double(1e-300*1e-20)}
    {exp(1)} {exp(0)} {exp(709)} {exp(710)} {exp(1000)}
    {exp(-708)} {exp(-720)} {exp(-745)} {exp(-746)} {exp(-1000)}
    {exp(1e-300*1e-20)}
    {floor(-1.5)} {floor(1e-300*1e-20)}
    {fmod(7,3)} {fmod(1,0)} {fmod(0,0)} {fmod(0,1)}
    {fmod(5,1e-300*1e-20)} {fmod(1e-300*1e-20,1)}
    {hypot(3,4)} {hypot(1e308,1e308)} {hypot(1e-300*1e-20,0)}
    {int(2.5)} {int(1e300)}
    {log(1)} {log(0)} {log(-1)} {log(-0.0)} {log(1e-300*1e-20)}
    {log10(100)} {log10(0)} {log10(-1)}
    {pow(2,10)} {pow(0,0)} {pow(0,1)} {pow(-2,3)}
    {pow(10,308)} {pow(10,309)} {pow(-10,309)}
    {pow(0,-1)} {pow(0,-0.5)} {pow(-1,0.5)} {pow(-8,1.0/3)}
    {pow(10,-320)} {pow(10,-400)} {pow(2,-1074)} {pow(2,-1075)}
    {pow(-2,-1074)} {pow(-2,-1075)} {pow(0.5,1100)} {pow(-0.5,1101)}
    {pow(1e-160,2)} {pow(1e-200,2)} {pow(1e-300*1e-20,1)} {pow(1e-300*1e-20,2)}
    {round(2.5)} {round(1e300)}
    {sin(1)} {sin(1e-300*1e-20)}
    {sinh(1)} {sinh(0)} {sinh(710)} {sinh(-710)} {sinh(1000)} {sinh(-1000)}
    {sinh(1e-300*1e-20)}
    {sqrt(4)} {sqrt(-1)} {sqrt(-0.0)} {sqrt(1e-300*1e-20)}
    {tan(1)} {tan(1e-300*1e-20)}
    {tanh(1)} {tanh(1000)} {tanh(-1000)} {tanh(1e-300*1e-20)}
    {wide(2.5)} {wide(1e300)} {wide(-1e300)} {wide(9.3e18)} {wide(-9.3e18)}
    {int(9.3e18)} {int(-9.3e18)} {int(9.2e18)} {int(-1e19)}
    {round(9.2e18)} {round(-9.2e18)} {round(9.3e18)} {round(-1e300)}
    {abs(-2.5)} {abs(-1e-300*1e-20)}
    {1e-300*1e-300} {1e300*1e300} {1e-320} {-1e-320} {sin(1e-320)}
    {1e-400} {0e-400} {1e400} {2.5e-308} {2.2250738585072014e-308}
} {
    show $c
}

proc p {} {
    set ::errorCode NONE
    puts "exp: [catch {expr {exp(-1000)}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "exp big: [catch {expr {exp(1000)}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "pow: [catch {expr {pow(10, -400)}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "atan2: [catch {expr {atan2(1e-300, 1e300)}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "log: [catch {expr {log(0)}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "sqrt: [catch {expr {sqrt(-1)}} r] <$r> <$::errorCode>"
    set x -1000
    set ::errorCode NONE
    puts "exp var: [catch {expr {exp($x)}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "exp sum: [catch {expr {1 + exp($x) * 2}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "pow var: [catch {expr {pow($x, -401)}} r] <$r> <$::errorCode>"
    set ::errorCode NONE
    puts "literal: [catch {expr {sin(1e-320)}} r] <$r> <$::errorCode>"
}
p
puts [expr {exp(-1000)}]
