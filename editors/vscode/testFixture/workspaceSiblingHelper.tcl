# A sibling workspace file for the recovery "known-command generality"
# tests (`errorRecovery.test.ts`): a proc the on-disk workspace scan
# indexes without this file ever being opened in an editor tab, so a
# break in a different open document can recover past a call to it.
proc workspace_helper {x} {return $x}
