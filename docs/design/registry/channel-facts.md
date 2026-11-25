# Channel option and callback facts

`rust/tcl-registry/src/channel_options.rs` owns `CHANNEL_OPTIONS`, the sole
authored configuration list. `CONFIGURE_OPTIONS` is its const projection
into the existing option interface; the `chan` command, `chan configure`
and `fconfigure` share that exact slice. `options_for` selects generic rows
and rows for a specified channel driver, under a Tcl release. Driver kind,
query/set access, value grammar and availability are separate facts.

A driver catalogue does not prove a live channel's driver. The original
source validator checks generic option grammars only. A reflected driver
may implement a setter for a spelling that a builtin driver makes query-only.
Driver-specific checking needs a separately proved instance kind. A union of
driver catalogues also cannot establish a driver-sensitive abbreviation. The
generic `-t` translation priority is retained as an explicit alias; other
ambiguous source selections abstain rather than claiming a driver. Unknown
custom encoding names remain unknown because an encoding search path may
supply them; recognised names use the generated release encoding catalogue.

Generic value grammars preserve Tcl's actual accepted forms:

- `-buffering` accepts unique prefixes of `full`, `line` and `none`.
- `-buffersize` parses a wide integer; the setter clamps it to 1 through
  1,048,576 rather than rejecting an out-of-range value.
- `-translation` takes one mode or an input/output pair, including `platform`
  and an empty member meaning no change. Unrecognised members remain unknown
  without channel-direction facts: an inactive side is ignored, and reflected
  channels can have neither direction. Malformed lists and lengths other than
  one or two remain provable errors.
- Tcl 8 `-eofchar` accepts up to two list members and uses their first ASCII
  byte. Tcl 9 accepts one ASCII character or an empty value, retaining the
  reference build's deprecated `x {}` spelling.
- Channel creation accepts empty mode lists since Tcl 8.6; Tcl 8.5 rejects
  them. Posted event masks require at least one member. Both accept unique
  prefixes of `read` and `write`. An unselected release cannot settle an empty
  creation mode list. Closed scalar value checking is insufficient for these
  list grammars.

`channel_protocols.rs` describes reflected channels, transformations, accepted
socket notifications and background copy completion. Each method records
its appended words, release availability, requirement and companion methods.
Reflected channels require `initialize`, `finalize`, `watch`, and the method
for each requested I/O direction; `cget` and `cgetall` occur together.
Transformations include `limit?`; `drain` requires `read`, and `flush` requires
`write`. A transformation must retain an accessible I/O direction.

Required callback arities are derived from method signatures. Reflected
channels and transformations require counts 2 and 3; optional methods such
as reflected `seek` and `configure` have count 4 and are checked only after
their advertised presence is known. A socket accept callback receives 3
words; a copy callback receives 1 or 2. Optional methods do not impose
additional mandatory arguments on a handler that never advertises them.

Command/subcommand `callback_protocols` and option-value `callback_protocol`
carry these descriptors. `ResolvedInvocation::authored_source_callback_protocol`
reads them only at an independently classified original command-prefix
operand, preserving ensemble offsets, selected options and unknown layouts.
This query provides documentation and signature metadata. It proves neither
handler installation, native entry, a future invocation, nor edit completeness.
The `chan event` body uses the shared deferred Channel-trigger descriptor.
Socket accept and channel-copy prefixes declare deferred global evaluation
contexts. Reflected handlers include immediate `initialize` calls and later
method calls, so no single deferred timing is assigned to the whole protocol.

The original-source literal validator is consumed by the existing generic
W146 path. Half-close directions use the same prefix vocabulary and lifecycle
on both spellings; their release restriction belongs to W135. No second
argument-value release gate is authored for that direction.

The SpecTcl loader and Studio do not yet author channel protocol selection.
The exact `callback_protocols` and `callback_protocol` gaps are recorded in
`render_spectcl::GAPS` and Studio coverage. Existing roles and derived appended
arities remain available to their existing authoring interfaces. Protocol-aware
completion inside handlers, hover and callback diagnostics require subsequent
consumers of these facts; the descriptors do not claim those features exist.

The [execution evidence](../../../evidence/tcl/channel-callback-protocols.md)
records callback counts and measured order. Separate
[mode and option evidence](../../../evidence/tcl/channel-mode-and-option-values.md)
records empty masks and direction-dependent translation checks. Registry conformance tests also
run the retained c06-c09 channel fixtures under the reference interpreters,
checking each observed reflected method count against its descriptor. Set
`TCL_LSP_REQUIRE_TCLSH=1` to fail on any missing reference interpreter.
