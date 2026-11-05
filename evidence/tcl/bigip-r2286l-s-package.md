# BIG-IP and appliance Tcl context: s package

## Question

When evaluating this source, is it true that package names reports the available packages in the execution context? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
package names⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case s_package; TMM log tag tmm1, RULE_INIT, case s_package; TMM log tag tmm2, RULE_INIT, case s_package; TMM log tag tmm3, RULE_INIT, case s_package; tmsh CLI script, case s_package; TMM log tag tmm, RULE_INIT, case p_packages; TMM log tag tmm1, RULE_INIT, case p_packages; TMM log tag tmm2, RULE_INIT, case p_packages; TMM log tag tmm3, RULE_INIT, case p_packages

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Tcl⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.4, case s_package; appliance tclsh8.4, case p_packages

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Tcl⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.5, case s_package; appliance tclsh8.5, case p_packages

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
tcl::tommath Tcl⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### iApp implementation, case s_package; iCall script, case s_package; iApp implementation, case p_packages; iCall script, case p_packages

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
tclparser xml::tcl http uri uuencode xslt::libxslt sha256 xmldep iapp tls opt sgml xslt::cache xmlswitch tcltest xml::tclparser xpath xslt::process md5 ascii85 msgcat xml::libxml2 yencode dom sgmlparser xml TclCurl uri::urn ldap ldapx xslt::resources asn iapp::legacy xmldefs xslt::utilities xslt Tcl puretclparser sha1 base64 dom::libxml2 tclperl⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### tmsh CLI script, case p_packages

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
Tcl⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.
