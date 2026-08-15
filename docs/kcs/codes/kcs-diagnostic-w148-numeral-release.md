# KCS: W148 — numeral spelling is not accepted by the target Tcl release

> **Audience:** User
> **Type:** Diagnostic

## What does W148 mean?

W148 marks a literal numeral whose spelling is valid in Tcl 9 but is not
accepted by the document's resolved Tcl release. For example, `0d5` and
`1_000` are valid Tcl 9 spellings but are not numerals in Tcl 8.4–8.6.

The warning is profile-aware. A Tcl 9 document accepts these spellings, and
leading-zero octal (`010`) is not warned about in either family because it is
valid in Tcl 8.x and valid as decimal in Tcl 9.

## How do I fix it?

Use a spelling accepted by the target release, such as `5` instead of `0d5`,
or remove digit separators from a value that must run on Tcl 8.x. Alternatively,
raise the document's resolved Tcl release if the script requires Tcl 9 syntax.

W148 abstains when the numeral is dynamic or the document has no resolved
release, so it does not claim a value that static analysis cannot establish.

## Related diagnostics

W137 and W138 cover version-gated command arguments and format conversions;
W148 covers the numeral spelling itself.
