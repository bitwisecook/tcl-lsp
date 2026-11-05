# BIG-IP stock Tcl: wiki-code-2

## Question

Does namespace code capture a procedure's local variables, or only its namespace for later lookup?

This case informs name resolution and diagnostics in the LSP, compiler and VM; it does not establish behavior in other environments.

## Assumptions

Recorded case scope: Fresh child/process; evaluate complete unchanged suite using real tcltest and actual isolated helper/constraints. A constraint skip is not unsupported proof.

Other assumptions were not separately recorded. Connection identifiers were redacted in the retained invocation.

## Script

Fixture: [bigip-stock-8.4-2](fixtures/bigip-stock-8.4-2.md)

- run: `nh20261012pat1`
- case: `wiki-code-2`

### nh_setup

```tcl-visible
package require tcltest
::tcltest::configure -match wiki-code-2
```

## Results

### BIG-IP stock-8.4

- Recorded version: BIG-IP 21.1.0.1; Tcl patchlevel 8.4.13.
- Recorded input channel: SSH to /usr/bin/tclsh8.4, script on standard input.
- Exit status: 0
- Standard output:

```text-visible
NHO|nh20261012pat1|wiki-code-2|stock-8.4|setup|0|none|once|result|utf-8|0|1|
oracle 8.4.13 library /usr/share/compat-tcl8.4 tcltest 2.2.8
++++ wiki-code-2 PASSED
:⟦TAB⟧Total⟦TAB⟧30⟦TAB⟧Passed⟦TAB⟧1⟦TAB⟧Skipped⟦TAB⟧29⟦TAB⟧Failed⟦TAB⟧0
NHO|nh20261012pat1|wiki-code-2|stock-8.4|lookup|0|none|once|result|utf-8|0|1|
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
