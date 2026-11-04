# Clay definition forwarding and HTTPD class declarations

## Question

Does pinned Tcllib 2.0 Clay preserve the original parameters and body of a plain lowercase `method` declaration, can `clay::define` extend an existing class, and does its HTTPD package declare `::httpd::server` with `connect`? The distinction matters to #1956: source class/member inventory must not assume that every Clay definition is a fresh TclOO allocation or that its transforming keywords preserve an entered body.

## Assumptions

Each row used a fresh process on 2026-11-04 UTC with the exact script below on standard input and no command-line arguments. `LC_ALL=C`; current working directory was `/workspace/tcl-lsp/tmp/lsp-providers`. Other inherited environment details: Not recorded. The script appends every pinned Tcllib module directory to `auto_path`, loads the packages through their actual package indexes, creates classes but instantiates no HTTPD server, and performs no network request. The Clay file SHA-256 is `150b48a3ddf4916cca5e182ce05274773ddcc3bc296c9b1e6f8c4d9afd1c49b0`; HTTPD is `36cdae3de3a8669d14deca6ac9ea6430670aefd82e28a163bb6d398f60c7288d`. Namespace-string equality is the measured redefinition observation, not a transferable identity receipt. Native execution does not establish compiler coverage or LSP dispatch authority. Jim stops during directory enumeration because its `glob` lacks `-types`; no Clay operation was reached there.

## Script

```tcl-visible
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
puts [list VERSION [info patchlevel]]
foreach dir [glob -nocomplain -directory /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules -types d *] {
  lappend auto_path $dir
}
set clay_code [catch {package require clay 0.7} clay_result]
puts [list CLAY_CODE $clay_code RESULT $clay_result]
if {!$clay_code} {
  namespace eval ::lsp1956 {}
  ::clay::define ::lsp1956::Probe {
    method connect {sock ip port} {return [list $sock $ip $port]}
    method Upper {} {return upper}
    constructor {} {return}
  }
  set before [info object namespace ::lsp1956::Probe]
  ::clay::define ::lsp1956::Probe {method second {} {return second}}
  puts [list PROBE_CLASS [info object isa class ::lsp1956::Probe] SAME_NAMESPACE [string equal $before [info object namespace ::lsp1956::Probe]]]
  puts [list CONNECT_DEFINITION [info class definition ::lsp1956::Probe connect]]
  puts [list UPPER_EXPORTED [expr {[lsearch -exact [info class methods ::lsp1956::Probe] Upper]>=0}] SECOND_PRESENT [expr {[lsearch -exact [info class methods ::lsp1956::Probe] second]>=0}]]
  puts [list CONSTRUCTOR_BODY_UNCHANGED [string equal [lindex [info class constructor ::lsp1956::Probe] 1] {return}]]
  namespace eval ::lsp1956 {::clay::define Local {method local {} {return local}}}
  puts [list RELATIVE_CLASS [info object isa class ::lsp1956::Local]]
  set httpd_code [catch {package require httpd} httpd_result]
  puts [list HTTPD_CODE $httpd_code RESULT $httpd_result]
  if {!$httpd_code} {
    puts [list HTTPD_SERVER_CLASS [info object isa class ::httpd::server] CONNECT_PRESENT [expr {[lsearch -exact [info class methods ::httpd::server -all] connect]>=0}]]
    puts [list HTTPD_CONNECT_PARAMETERS [lindex [info class definition ::httpd::server connect] 0]]
  }
}
```

## Results

### Tcl 8.4.20

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.4.20
CLAY_CODE 1 RESULT {can't find package clay 0.7}
```

- Standard error:

```text-visible
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/log/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/multiplexer/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/htmlparse/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/json/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/rest/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/uev/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/bee/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/websocket/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ftp/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/virtchannel_core/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/nmea/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/hook/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/control/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/report/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/mkdoc/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/practcl/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/defer/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/http/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/bench/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/tar/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/bibtex/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/comm/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/pt/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/clock/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/imap4/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/wip/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/fileutil/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/oauth/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/markdown/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/textutil/test-assets/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/textutil/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/pki/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/pop3/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/md5crypt/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/amazon-s3/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/png/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/exif/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/mapproj/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/dtplite/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/doctools2idx/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ident/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/base32/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/profiler/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/term/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/doctools2toc/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/irc/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/grammar_aycock/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/sasl/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/httpd/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/base64/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/md4/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/tiff/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/aes/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/try/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/tie/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/calendar/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/nettool/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/counter/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/lambda/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/pluginmgr/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/md5/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/namespacex/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/smtpd/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/mime/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/clay/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/nns/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ncgi/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/blowfish/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/dns/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ripemd/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/yaml/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/stooop/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/generator/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/zip/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/coroutine/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/snit/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/pop3d/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/doctools/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ftpd/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/cache/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/udpcluster/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/virtchannel_base/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/nntp/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/jpeg/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/uri/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/uuid/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/rc4/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/des/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ooutil/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/inifile/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/lazyset/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/fumagic/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/crc/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/interp/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/cron/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/cmdline/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/grammar_fa/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ldap/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/html/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/gpx/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/soundex/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/grammar_me/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/map/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/doctools2base/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/debug/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/string/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/tool/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/oometa/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/ntp/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/transfer/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/struct/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/tepam/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/javascript/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/math/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/otp/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/sha1/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/treeql/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/units/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/virtchannel_transform/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/asn/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/rcs/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/docstrip/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
error reading package index file /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules/csv/pkgIndex.tcl: wrong # args: should be "package vsatisfies version1 version2"
```

### Tcl 8.5.19

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.5.19
CLAY_CODE 1 RESULT {can't find package clay 0.7}
```

- Standard error: empty.

### Tcl 8.6.18

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.6.18
CLAY_CODE 0 RESULT 0.8.8
PROBE_CLASS 1 SAME_NAMESPACE 1
CONNECT_DEFINITION {{sock ip port} {return [list $sock $ip $port]}}
UPPER_EXPORTED 0 SECOND_PRESENT 1
CONSTRUCTOR_BODY_UNCHANGED 0
RELATIVE_CLASS 1
HTTPD_CODE 0 RESULT 4.3.6
HTTPD_SERVER_CLASS 1 CONNECT_PRESENT 1
HTTPD_CONNECT_PARAMETERS {sock ip port}
```

- Standard error: empty.

### Tcl 9.0.4

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.0.4
CLAY_CODE 0 RESULT 0.8.8
PROBE_CLASS 1 SAME_NAMESPACE 1
CONNECT_DEFINITION {{sock ip port} {return [list $sock $ip $port]}}
UPPER_EXPORTED 0 SECOND_PRESENT 1
CONSTRUCTOR_BODY_UNCHANGED 0
RELATIVE_CLASS 1
HTTPD_CODE 0 RESULT 4.3.6
HTTPD_SERVER_CLASS 1 CONNECT_PRESENT 1
HTTPD_CONNECT_PARAMETERS {sock ip port}
```

- Standard error: empty.

### Tcl 9.1.0

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.1.0
CLAY_CODE 0 RESULT 0.8.8
PROBE_CLASS 1 SAME_NAMESPACE 1
CONNECT_DEFINITION {{sock ip port} {return [list $sock $ip $port]}}
UPPER_EXPORTED 0 SECOND_PRESENT 1
CONSTRUCTOR_BODY_UNCHANGED 0
RELATIVE_CLASS 1
HTTPD_CODE 0 RESULT 4.3.6
HTTPD_SERVER_CLASS 1 CONNECT_PRESENT 1
HTTPD_CONNECT_PARAMETERS {sock ip port}
```

- Standard error: empty.

### Jim 0.84-10-g82fb4d0

- Launch argv: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 1
- Standard output:

```text-visible
VERSION 0.84-10-g82fb4d0
```

- Standard error:

```text-visible
stdin:5: Error: bad option "-types": must be -directory, -nocomplain, -tails, or --
Traceback (most recent call last):
  File "stdin", line 5
    glob -nocomplain -directory /workspace/tcl-lsp/tmp/lsp-providers/tmp/tcllib-2.0/modules -types d *
```

## Observation

C Tcl 8.6.18, 9.0.4 and 9.1.0 load Clay 0.8.8 and HTTPD 4.3.6. The plain lowercase `method` retains its parameters and body; a repeated definition adds `second` while retaining the reported class namespace. `Upper` is unexported by default. The constructor body differs from the authored body, so it must not be presented as an unchanged entered method body. The relative class is created in the caller namespace. Each supported run reports the HTTPD server class and its `connect` parameters. Tcl 8.4 and 8.5 cannot find this Clay release; Tcl 8.4 also reports the recorded package-index errors. Jim's setup failure establishes none of the Clay behaviours.
