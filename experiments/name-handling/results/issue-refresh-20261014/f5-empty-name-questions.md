# F5 Tcl: empty names in materialized scripts

These cases have no BIG-IP result yet. Run them in a Tcl scope appropriate to the selected BIG-IP interpreter and report the exact platform/software version, interpreter patchlevel, script bytes, stdout, stderr, and catch completion code. Keep iRules event execution separate from a Tcl shell or management interpreter.

Does the host preserve an empty scalar name and an array whose root name is empty through `eval`, reads, writes, and `unset`? Does its `interp alias` surface permit capturing an empty variable-name argument? If the command is unavailable, record that result without replacing the script with a different mechanism.

C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, and 9.1.0 produced the observations below. Latest measured Jim (`5bac7c99ad65864c87da513e22e2f01703fa4e03`) agrees for scalar, array, and replacement controls; it rejects the alias syntax. These observations supply no F5 expectations.

## empty-materialized-set-read-unset

```tcl
eval {set {} FIRST}
puts [eval {set {}}]
eval {set {} SECOND}
puts [eval {set {}}]
eval {unset {}}
puts [info exists {}]
```

Measured C stdout: `FIRST\nSECOND\n0\n`; exit code 0.

## empty-materialized-array

```tcl
eval {set {(key)} VALUE}
puts [eval {set {(key)}}]
eval {unset {(key)}}
puts [info exists {(key)}]
```

Measured C stdout: `VALUE\n0\n`; exit code 0.

## empty-materialized-alias

```tcl
interp alias {} put {} set {}
eval {put VALUE}
puts ${}
```

Measured C stdout: `VALUE\n`; exit code 0.

## empty-materialized-replacement

```tcl
rename set original_set
proc set args {return CUSTOM}
puts [eval {set {} VALUE}]
puts [info exists {}]
```

Measured C stdout: `CUSTOM\n0\n`; exit code 0.

Reply in the same branch with an answer document and the raw execution artifacts. The compiler tests must follow measured host results when the F5 surface differs.
