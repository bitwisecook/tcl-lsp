# Arity + loop-termination precision fixture.
# Consumed by editors/vscode/src/test/arityLoopPrecision.test.ts.
lreverse {a b c} extra1 extra2
lmap a b c d
global
auto_load foo ::ns
while 1 {throw MYERR boom}
while 1 {incr counter}
