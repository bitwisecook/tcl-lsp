# KCS: feature — tcllib package coverage

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, tcl-lsp-cli, mcp

## Summary

The command registry recognises a large set of [tcllib](https://core.tcl-lang.org/tcllib/)
packages so that their commands get completion, hover, signature help,
arity checking, and missing-`package require` warnings — the same
treatment core Tcl commands receive.

## Availability

| Context | How |
|---------|-----|
| Any LSP editor | Add `package require <name>` to a `.tcl` file, then use the package's commands |
| `tcl` CLI | Analyse a script that uses a covered package |

A tcllib command is only offered once the file contains the matching
`package require` (for example `package require crc16` before
`crc::crc16`). Using the command without the require raises the
missing-package warning (W120).

## Covered package families

Alongside the previously-supported packages (`base64`, `csv`, `dns`,
`fileutil`, `ip`, `json`, `logger`, `math::statistics`, `md5`, `mime`,
`sha1`, `sha2`, `snit`, `struct::list`/`queue`/`set`/`stack`, `textutil`,
`uri`, `uuid`, `yaml`, `cmdline`, `control`, `html`), the registry now
also covers:

- **Cryptography and hashing** — `md4`, `md5crypt`, `ripemd128`,
  `ripemd160`, `crc16`, `crc32`, `cksum`, `sum`, `aes`, `blowfish`,
  `des`, `rc4`, `otp`.
- **Encoding** — `base32`, `base32::hex`, `base32::core`, `ascii85`,
  `uuencode`, `yencode`.
- **Text** — `soundex`, `stringprep`, `unicode`, `stooop`,
  `textutil::repeat`, `textutil::split`, `textutil::wcswidth`,
  `term::ansi::code`, `term::ansi::send`.
- **Data and utility** — `inifile`, `units`, `counter`, `tie`,
  `lambda`, `defer`.
- **Mathematics** — `math`, `math::fuzzy`, `math::roman`,
  `math::constants`.
- **Web, protocol, and client** — `asn`, `ncgi`, `htmlparse`,
  `Markdown`, `oauth`, `imap4`, `rest`, `SASL`, `time` (SNTP),
  `websocket`, `log`, `ftp`, `ldap`, `pop3`, `irc`, `uevent`.
- **Formats and geo** — `gpx`, `png`, `jpeg`, `tiff`, `mapproj`,
  `nmea`, `bibtex`, `rcs`, `javascript`.
- **Ensembles** — `generator`, `debug`, `hook` (dispatched on a
  sub-command word, with per-sub-command arity and hover).

## Example

```tcl
package require crc16
set sum [crc::crc16 -format 0x%04X "hello"]
```

Hovering `crc::crc16` shows its summary, synopsis, and `-format` /
`-seed` / `-implementation` / `-filename` options. Omitting the
`package require crc16` line raises W120 on `crc::crc16`.

## Where the data lives

Each package is a set of `CommandSpec` entries in
[`tcl-registry`](../../design/compiler/command-registry.md) under
`rust/tcl-registry/src/commands/tcllib/`. Command names, arity bounds,
options, enum values, and hover text are derived from the upstream
tcllib manual pages.
