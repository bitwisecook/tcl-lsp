# Clay class inventory audit (#1956)

The exact Tcllib 2.0 HTTPD file has no class symbols in the held native server,
although native Tcl loads its server class and `connect` method. This is an
audit and an uncompiled source-inventory draft; no verified repair or passing
regression is claimed yet. #2243 remains a separate, parked coverage blocker.

The reviewed v2 source and tests are now included as an **UNBUILT/UNVERIFIED**
development checkpoint under the user's final push instruction. The
[permanent handoff](reviewed-draft-v2/README.md) preserves the exact patch,
base, and all 18 source hashes. Its nine semantic controls and code generation
remain unrun. Older stashes were not restored.

The audit receipt pins the files. `clay/clay.tcl` has
SHA-256 `150b48a3ddf4916cca5e182ce05274773ddcc3bc296c9b1e6f8c4d9afd1c49b0`;
`httpd/httpd.tcl` has
`36cdae3de3a8669d14deca6ac9ea6430670aefd82e28a163bb6d398f60c7288d`.
Clay's `dialect::create` starts at line 806, byte 20732. Its generated `define`
procedure at line 852 creates a missing class or configures an existing one.
`DefineThunk` at line 909 forwards inherited lowercase definition commands to
`oo::define`. The call at line 1008 creates the Clay dialect. HTTPD's server
header is at line 734, byte 20371; its `method connect` header is at line 773,
byte 21863. The callback prefix `[list my connect]` occurs at line 1011.

The [native capture](../../../evidence/tcl/lsp-1956-clay-definition-forwarding.md)
records fresh C Tcl 8.4–9.1 and Jim processes. C Tcl 8.6.18, 9.0.4 and 9.1.0
load Clay 0.8.8 and HTTPD 4.3.6, report the HTTPD server class and `connect`
parameters, and preserve the plain lowercase method's authored parameters and
body. Repeated `define` adds a method while retaining the reported namespace.
An uppercase method is unexported; a constructor body is rewritten. Tcl 8.4
and 8.5 cannot load this Clay release. Jim stops at the probe's unsupported
`glob -types` setup, so its Clay operations are untested in that capture.
The [portable follow-up](../../../evidence/tcl/lsp-1956-clay-package-loading.md)
uses exact known module paths. It reaches Jim's Clay header and records its
multi-requirement `package require` error. This Tcl 8.5 host reports neither
`::oo::class` nor an installed TclOO package; no claim is made about other
Tcl 8.5 hosts with that extension.

The baseline probe uses the existing native server from
clean source `cab14d1a4fe76da7e88eb5225b9533408d35e95b`, before the independent
seed fix. The binary SHA-256 is
`e4e4153afadf5906975a94fe661d81ef1040ecce0c630416c4cde9c0742d2ab7`, unchanged
before and after the protocol run. The protocol receipt
records a TclOO control with one class and two method symbols, no symbols for
the equivalent Clay or near-name command, and nine namespace/variable symbols
for the exact HTTPD file with no classes. `my connect` definition requests
return null even for the TclOO control. That route cannot become a claimed
Clay success by assuming an entered receiver. The server exits 0.

The native registry's `commands/tcllib` pack owns these library descriptors;
bundled SpecTcl packs own the EDA vendor libraries. The audited baseline has no Clay descriptor
or grammar. `DefinitionBodyGrammar` already owns member roles/effects, and
`DefinerFamily` selects the analyser's existing family walkers.
`original_provider_class_name`, `retain_original_provider_class` and
`retain_original_member_metadata` retain original source words independently
of handler execution. A provider family uses `SourceValue` member names;
Native TclOO method entry remains a separate naming purpose.

The uncompiled draft adds a `clay::define` descriptor and a Clay family
grammar, with a family-gated source-inventory handler using the shared body,
member and original-name owners. Only the exact `name script` form receives a
definition body; inline or expanded forms retain generic handling. Plain
lowercase `method` can retain its unmodified header, parameters and body.
Transforming Clay members remain incomplete source metadata. The family
supplies no installed dispatcher, manufacturer, native class receipt, complete
inheritance or member set. The existing source-class transition owner accepts
TclOO manufacturers or Snit/Itcl installation recipes; this proposal does not
extend it or borrow one for Clay. Package availability is advice from the
registry's package owner, never proof that the analyser loaded a package.

Verification must require the exact HTTPD class/method sites and original
source metadata, with ordinary TclOO and unknown-command controls. Static
braced/escaped names and namespace placement must use existing name owners;
computed or expanded names must not gain navigation authority. Native dispatch,
factory execution and rename remain independently selected. No Cargo,
`make rust-check` or `make prep-pr` has run for this audit.

The reviewed implementation uses existing exact-arity `CommandForm` data and
resolved `Name`/`Body` roles. The original conditional Registry metadata owns
full source argv, namespace coordinates, registry/input identity, and shadow
barriers. Four compiler controls, two registry controls, two language-server
outline/authority controls, and one exact HTTPD corpus test are authored but
unrun. The corpus test uses `FixedCorpusSource` and the current Tcllib bootstrap
contract; missing or changed bytes fail setup. The input remains upstream
Tcllib source with its original copyright and licence; no vendored file was
rewritten. See the [generated-file plan](generated-file-plan.md) for the pending
refresh and gate obligations. Full `rust-check` and `prep-pr` remain unrun.

The original Tcllib `license.terms` has SHA-256
`38e1f556213f4008cfd280457af57c76a1fe043dc026ece1c704c09b5f6e865c`.
The fixture pin links the original upstream file; no AGPL header is added to
those upstream bytes. The portable native follow-up reaches Jim package loading
without an unsupported glob option and records the actual header rejection.
