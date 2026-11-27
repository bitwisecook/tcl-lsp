# KCS: tclvm cannot find msgcat or another library package

> **Audience:** User
> **Type:** Issue

## Applies to

tcl-lsp-cli

## Question

Why does `package require msgcat` work in `tclsh` but fail in `tclvm`?

## Symptoms

- `package require msgcat 1.6` fails with `can't find package msgcat 1.6`.
- `[info library]` fails with `no library has been specified for Tcl`.
- `$auto_path` is empty.
- Or `tclvm` prints `application-specific initialization failed:` before your
  script runs.

## Answer

`tclvm` does not ship a Tcl script library. It uses the one `TCL_LIBRARY`
names, and sources that library's `init.tcl` at startup, as `tclsh` does.

1. Point `TCL_LIBRARY` at the `library` folder of a Tcl source tree or install.
2. Pick the `tclvm` release that matches it. The library checks the release
   it was written for:

   ```bash
   TCL_LIBRARY=/path/to/tcl9.1.0/library tclvm --tcl-version 9.1 script.tcl
   ```

3. Run `puts $auto_path`. It now lists the library and its parent folder.

If `tclvm` reports `version conflict for package "tcl"`, the library belongs to
another release. Change `--tcl-version` or `TCL_LIBRARY` so they agree.

## Related

- [How do I manage Tcl packages?](kcs-howto-manage-tcl-packages.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
