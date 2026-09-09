# KCS: feature — tcllib package coverage

> **Audience:** User
> **Type:** Functionality

## Summary

The command registry recognises a large set of [tcllib](https://core.tcl-lang.org/tcllib/)
packages so that their commands get completion, hover, signature help,
arity checking, and missing-`package require` warnings — the same
treatment core Tcl commands receive.

## Applies to

all-editors, tcl-lsp-cli, mcp

## How to use

Add the matching `package require` to the file, then use the package's
commands. A tcllib command is only offered once the require is present (for
example `package require crc16` before `crc::crc16`); using it without the
require raises the missing-package warning (W120). The `tcl` CLI applies the
same rule when it analyses a script.

## Covered package families

- **Cryptography and hashing** — `md4`, `md5`, `md5crypt`, `sha1`, `sha2`,
  `ripemd128`, `ripemd160`, `crc16`, `crc32`, `cksum`, `sum`, `aes`,
  `blowfish`, `des`, `rc4`, `otp`.
- **Encoding** — `base64`, `base32`, `base32::hex`, `base32::core`,
  `ascii85`, `uuencode`, `yencode`, `mime`.
- **Text** — `textutil`, `textutil::repeat`, `textutil::split`,
  `textutil::wcswidth`, `soundex`, `stringprep`, `unicode`, `stooop`,
  `term::ansi::code`, `term::ansi::send`.
- **Data and utility** — `csv`, `json`, `yaml`, `inifile`, `units`,
  `counter`, `tie`, `lambda`, `defer`, `cmdline`, `control`, `uuid`,
  `logger`, `snit`, `struct::list` / `queue` / `set` / `stack`.
- **Mathematics** — `math`, `math::statistics`, `math::fuzzy`,
  `math::roman`, `math::constants`.
- **Web, protocol, and client** — `uri`, `html`, `ncgi`, `htmlparse`,
  `Markdown`, `asn`, `oauth`, `imap4`, `rest`, `SASL`, `time` (SNTP),
  `websocket`, `log`, `ftp`, `ldap`, `pop3`, `irc`, `uevent`, `dns`, `ip`.
- **Formats and geo** — `gpx`, `png`, `jpeg`, `tiff`, `mapproj`,
  `nmea`, `bibtex`, `rcs`, `javascript`.
- **Files** — `fileutil`.
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
