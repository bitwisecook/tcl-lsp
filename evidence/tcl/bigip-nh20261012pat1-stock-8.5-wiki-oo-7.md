# BIG-IP stock Tcl: wiki-oo-7

## Question

Does replacing a TclOO method preserve or reset that method's explicit export state?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Fresh child/process; evaluate complete unchanged suite using real tcltest and actual isolated helper/constraints. A constraint skip is not unsupported proof.

The intended operation was not reached; this capture records the earlier outcome.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.5-2](fixtures/bigip-stock-8.5-2.md)

- run: `nh20261012pat1`
- case: `wiki-oo-7`

### nh_setup

```tcl-visible
package require tcltest
::tcltest::configure -match wiki-oo-7
```

## Results

### BIG-IP stock-8.5

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.5.13.
- Recorded input channel: SSH to /usr/bin/tclsh8.5, script on standard input.
- Exit status: 0
- Standard output:

```text-visible
NHO|nh20261012pat1|wiki-oo-7|stock-8.5|setup|0|none|once|result|utf-8|0|1|
oracle 8.5.13 library /usr/share/tcl8.5 tcltest 2.3.4
++++ wiki-oo-7 SKIPPED: tcloo
:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧0⟦TAB⟧Skipped⟦TAB⟧30⟦TAB⟧Failed⟦TAB⟧0
Number of tests skipped for each constraint:
⟦TAB⟧1⟦TAB⟧tcloo
NHO|nh20261012pat1|wiki-oo-7|stock-8.5|lookup|0|none|once|result|utf-8|0|1|
```

- Standard error:

```text-visible
** WARNING: connection is not using a post-quantum key exchange algorithm.⟦CRLF⟧
** This session may be vulnerable to "store now, decrypt later" attacks.⟦CRLF⟧
** The server may need to be upgraded. See https://openssh.com/pq.html⟦CRLF⟧
```

The exit status and streams are from the SSH invocation; standard error can include SSH transport messages as well as remote errors. The entries below decode complete NHO result payloads from standard output.

- setup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```

- lookup, completion 0, result (utf-8; unit none, invocation once):

```text-visible
⟦EOF⟧
```
