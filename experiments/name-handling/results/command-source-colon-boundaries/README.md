# Command source fragments at substitution boundaries

Question `Q-COLON-WILDCARD-BOUNDARY-01`: can fixed source fragments on opposite
sides of an empty substitution form one namespace separator run?

The probe creates namespace `::a`, sets `x` to the empty string, and declares
`proc a:::$x:b {} {return HIT}`. All five original pinned C Tcl providers
(8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0) select `::a::b`: invocation returns
`HIT` with completion code 0. The independent `::a::OTHER` control returns
completion code 1 and has no selected command. Every final provider process
exits 0 with empty stderr. Exact inputs, invocation, executable/source/build
hashes, original source revisions, stdout, stderr and exit codes are in
[capture04/manifest.json](capture04/manifest.json).

An overlap analysis cannot canonicalise `a:::` and `:b` independently around
an unknown `$x` and then treat the resulting fragments as mandatory separated
runs. With an empty value the written name is `a::::b`; its separator is one
run. Conservative source advice may retain fixed literal interiors bounded by
non-colon bytes while dropping separator constraints at unknown boundaries.
This does not establish a namespace incarnation, reached command binding,
publication, complete effect model or edit permission.

The scripts run through the repository-provisioned original interpreter
binaries, without Tcltest or a replacement harness. The final probe reads the
actual runtime library's `init.tcl`; each sampled byte sequence matches the
corresponding original source file hash. This includes Tcl 9's embedded ZipFS
library. Temporary library samples, binaries, original source trees and build
outputs are not bundled here. Their exact hashes and source revisions remain
in the manifests; restore them through the repository toolchain owner. To
reproduce, copy the exact probe into an empty capture directory and execute it
with each required owner-provided interpreter; the probe writes its sampled
library bytes beside itself.

[capture01](capture01/manifest.json) retains the semantic-only probe before
the runtime-library byte comparison. [capture02](capture02/manifest.json)
records a Tcl 8.4 setup rejection of the `rb` access mode, before either
semantic command. [capture03](capture03/manifest.json) records reached C 8.x
lookups and Tcl 9 setup rejections of the removed `binary` encoding. These
setup failures are not lookup answers. The final capture uses the common
`-translation binary` form and reaches both controls on every provider.

These observations cover original C Tcl only. They supply no Jim, F5, Runtime,
VM or Rust consumer acceptance. Rust source-pattern and static-provenance
controls retain separate execution requirements.
