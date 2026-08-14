# SpecTcl end-to-end validation: `tclinterp` consumed by `SpiceGenTcl`

> Validation date: 14 August 2026
>
> Product branch: `rust`-line implementation under test
>
> Automated-test boundary: the GeorgeTree repositories are evidence only and are not test dependencies

## Result

A project `.tclspec` changes the Rust CLI and VS Code language features for a
real library command. Without the pack, the command has no registry metadata;
with it, the CLI reports the command and diagnoses an incomplete call, while
VS Code gains hover and signature help and reclassifies the three option names
from `string` to `decorator` semantic tokens.

The permanent regression tests do not clone or execute external code. They use
the small library/consumer pair in
[`tests/fixtures/spec-packs/tiny-project`](../../../../tests/fixtures/spec-packs/tiny-project/).
The pinned GeorgeTree example in this report is a reproducible documentation
case for the same path through the product.

## Pinned library and consumer

The library is
[`georgtree/tclinterp` at `ccd894bbcf759607cec37cb308e12fca639b17b0`](https://github.com/georgtree/tclinterp/tree/ccd894bbcf759607cec37cb308e12fca639b17b0).
Its `lin1d` implementation and inline documentation are pinned at
[`tclinterp.tcl:139`](https://github.com/georgtree/tclinterp/blob/ccd894bbcf759607cec37cb308e12fca639b17b0/tclinterp.tcl#L139).
The consumer is
[`georgtree/SpiceGenTcl` at `e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec`](https://github.com/georgtree/SpiceGenTcl/tree/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec).

The consumer requires the dependency and imports its interpolation namespace
([`diode_extract.tcl:5-8`](https://github.com/georgtree/SpiceGenTcl/blob/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec/examples/ngspice/advanced/diode_extract.tcl#L5-L8)):

```tcl
package require tclinterp
namespace import ::tclinterp::interpolation::*
```

It then calls `lin1d` with measured voltage/current lists
([`diode_extract.tcl:55`](https://github.com/georgtree/SpiceGenTcl/blob/e8aa45cee7053ebbd92af029f27ee2d6d31ed6ec/examples/ngspice/advanced/diode_extract.tcl#L55)):

```tcl
set iInterp [lin1d -x $vRaw -y $iRaw -xi $vInterp]
```

The library declares a variadic Tcl procedure and describes the actual option
shape in its `argparse` block:

```tcl
proc ::tclinterp::interpolation::lin1d {args} {
    argparse -help {Does linear one-dimensional interpolation.} {
        {-x!=  -help {Strictly increasing independent values}}
        {-y!=  -help {Dependent values}}
        {-xi!= -help {Interpolation points}}
    }
}
```

## Pack generation and validation

The SpecTcl authoring workflow produced structured procedure documentation
from the source, including the `-x`, `-y`, and `-xi` parameter descriptions and
the list return. A registry lookup returned `found: false`, confirming that the
fully-qualified command did not collide with a shipped command. Validation of
the generated pack reported:

```text
commands: 1
notices: 0
collisions: 0
fields: arity, dialects, forms, hover, options, required_package, return_type
```

The generated pack is committed as
[`tclinterp.tclspec`](tclinterp.tclspec). Its executable contents are:

```tcl
speclib tclinterp 0.15 {

command ::tclinterp::interpolation::lin1d {
    dialects {tcl9.0 tcl9.1}
    arity 6
    required_package tclinterp
    return_type List

    option -x -takes xValues \
        -detail {List of independent values; values must be strictly increasing.}
    option -y -takes yValues \
        -detail {List of dependent values corresponding one-for-one with -x.}
    option -xi -takes interpolationPoints \
        -detail {List of independent values at which to interpolate.}

    form Default {::tclinterp::interpolation::lin1d -x xValues -y yValues -xi interpolationPoints}

    hover {
        summary {Perform one-dimensional linear interpolation.}
        synopsis {::tclinterp::interpolation::lin1d -x xValues -y yValues -xi interpolationPoints}
        description {Interpolate dependent values at each point in -xi from paired -x and -y samples. The -x list must be strictly increasing, and the -x and -y lists must have equal lengths.}
        source {georgtree/tclinterp tclinterp.tcl:139-171 at ccd894bbcf759607cec37cb308e12fca639b17b0}
        returns {A list of interpolated dependent values, one for each value in -xi.}
    }
}

}
```

The current DSL has no option-level `required exactly once` declaration.
`arity 6` captures this command's three option/value pairs, but it cannot prove
that each distinct option occurs exactly once. That limitation is kept explicit
instead of inventing unsupported syntax.

## CLI proof

The same `tcl` binary was run in two working directories against Tcl 9.0. The
“without” directory had no `.tcl-lsp`; the “with” project root contained the
validated pack at `.tcl-lsp/tclinterp.tclspec`.

| Query | Without `.tclspec` | With `.tclspec` |
|---|---|---|
| `command-info ::tclinterp::interpolation::lin1d --json` | exit 1, `found: false` | exit 0, `found: true` |
| Summary | absent | `Perform one-dimensional linear interpolation.` |
| Synopsis | absent | qualified `lin1d -x … -y … -xi …` form |
| Switches | absent | `-x`, `-xi`, `-y` |
| Incomplete call, `lin1d -x $x` | exit 0, no diagnostics | exit 1, `E002` and `W120` |
| Complete call after `package require tclinterp` | opaque but clean | recognised and clean |

The two diagnostics are meaningful analysis that is impossible while the
command is unknown:

- `E002`: the call does not satisfy the declared command shape and reports the
  pack-authored usage form.
- `W120`: the call is missing `package require tclinterp`.

## VS Code proof

The extension integration harness opened this qualified probe using VS Code
1.133.0 on arm64 macOS:

```tcl
::tclinterp::interpolation::lin1d -x $vRaw -y $iRaw -xi $vInterp
```

It first set `tclLsp.specPacks` to an empty list, captured providers, then set
it to the generated pack. The extension waited for `getEffectiveConfig` to
confirm that `tclinterp` and its one command were loaded before capturing the
second result.

| Language feature | Without `.tclspec` | With `.tclspec` |
|---|---|---|
| Hover | empty | summary plus the qualified synopsis |
| Signature help | empty | `::tclinterp::interpolation::lin1d -x xValues -y yValues -xi interpolationPoints` |
| `-x` token | `string` | `decorator` |
| `-y` token | `string` | `decorator` |
| `-xi` token | `string` | `decorator` |

The complete token sequence on the call line was:

| Text | Without type | With type |
|---|---|---|
| `::tclinterp::interpolation::` | `namespace` | `namespace` |
| `lin1d` | `function` | `function` |
| `-x` | `string` | `decorator` |
| `$vRaw` | `variable` | `variable` |
| `-y` | `string` | `decorator` |
| `$iRaw` | `variable` | `variable` |
| `-xi` | `string` | `decorator` |
| `$vInterp` | `variable` | `variable` |

The harness passed in 2.2 seconds. This also exposed and verified a product
fix: semantic-token database queries had used the base dialect registry even
after hover and signature help had loaded the workspace overlay. They now use
the registry keyed by the active spec-pack generation, so all providers see
the same command metadata.

## Permanent regression coverage

The repository-owned fixture defines `::tcl_lsp_fixture::collect`, whose first
argument is `VarWrite` and whose second is a Tcl `Body`. Its consumer is:

```tcl
package require tcl_lsp_fixture

set input 3
::tcl_lsp_fixture::collect output {
    set doubled [expr {$input * 2}]
    lappend output $doubled
}
puts $output
```

The CLI tests prove current-project `.tcl-lsp` discovery and compare diagnostics
with and without the pack. The VS Code test proves the hover/signature change,
that `output` becomes a `variable` token through `VarWrite`, and that `set` and
`lappend` inside the custom command become `function` tokens only when the
second argument is recognised as a `Body`.

Relevant tests and fixture:

- [`rust/tcl-cli/tests/cli.rs`](../../../../rust/tcl-cli/tests/cli.rs)
- [`editors/vscode/src/test/specPacks.test.ts`](../../../../editors/vscode/src/test/specPacks.test.ts)
- [`tests/fixtures/spec-packs/tiny-project`](../../../../tests/fixtures/spec-packs/tiny-project/)

## macOS launch-crash note

Two sandboxed launches of VS Code 1.132.0 and 1.133.0 aborted in
`HIServices::_RegisterApplication` while AppKit was creating
`NSApplication`. Both stacks end in Electron startup before an extension host
or tcl-lsp process starts. Launching the same 1.133.0 test build outside that
GUI sandbox started the extension host, ran this test, and exited normally.
Those reports therefore describe the test-launch environment, not a crash in
the Tcl extension or language server.
