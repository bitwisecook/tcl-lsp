# BIG-IP parser probes

The iRules, drivers, controls and raw transcripts behind
[`docs/design/bigip-irule-parser-measurements.md`](../../../docs/design/bigip-irule-parser-measurements.md),
which supplies the live evidence that
[`dialect-and-package-registry-redesign-bigip-evidence-review.md`](../../../docs/design/dialect-and-package-registry-redesign-bigip-evidence-review.md)
§E3 was waiting on.

Everything here is third-party-free, appliance-specific, and disposable. Nothing
is a build input; nothing runs in CI.

## Layout

| Path | What it is |
| --- | --- |
| `suites/*.probes` | Probe definitions: `@@ id \| WRAP\|RAW \| description` followed by a body. `WRAP` wraps the body in `when HTTP_REQUEST` (compiled, never executed); `RAW` is the whole rule. |
| `suites/*.snippets` | Tab-separated `id<TAB>snippet` for word-formation probes; each becomes a `RULE_INIT` rule that logs `llength` and the value. |
| `suites/09-tcl85-features.tcl` | The 25 Tcl 8.5-feature cases, shared verbatim between the tclsh controls, a `cli script`, and an iApp implementation. |
| `irules/**/*.conf` | Materialised iRules, one per probe — 378 of them. Loadable directly with `tmsh load sys config merge file`. |
| `lib/runner.sh` | Merge → classify (`ACCEPT` / `WARN` / `REJECT`) → delete driver for a `.probes` suite. |
| `lib/wsrun.sh` | Same for word-formation suites, additionally scraping the logged word list. |
| `lib/materialise.sh` | Expands a `.probes` suite into `.conf` iRules locally, using the same rules as `runner.sh`. |
| `lib/gen-runtime-semantics.sh` | Emits the `RULE_INIT` rules that verify N-rule semantics by execution rather than compile acceptance. |
| `suites/10-context-parity.cases` | The single 34-case list behind the four-context parity probe. One source, four wrappers, so any transcript difference is a real context difference. |
| `lib/gen-context-parity.py` | Compiles that case list into an iRule, a `cli script`, an iApp template+service, and a `tclsh` script. |
| `lib/e4-context-probe.sh` | Runs all four contexts on the appliance under the §E4 contract: absence check before every create, `EXIT` trap, absence proof after every delete, virtual-server attachment check, APL recorded as `Unknown`, never `save sys config`. |
| `lib/tclcheck.tcl` | Stock-Tcl acceptance checker. Stubs only iRule-specific commands. |
| `controls/*.tcl` | Stock-Tcl controls, run with `tclsh8.4` and `tclsh8.5` **on the appliance itself**. |
| `results/*` | Raw transcripts from the 2026-08-26 run. |

## Running a suite

```sh
scp lib/runner.sh lib/tclcheck.tcl bigip1:/var/tmp/
scp suites/01-syntax.probes bigip1:/var/tmp/probes.txt
ssh bigip1 'bash /var/tmp/runner.sh'
```

Output is `id~~mode~~verdict~~tcl84~~tcl84msg~~description`, and rejection
messages accumulate in `/var/tmp/warnings.txt`.

## Appliance notes

Learned the hard way; all of these cost a cycle.

- `tmsh -f <file>` does not exist and piping a rule into `tmsh` on stdin fails.
  `tmsh load sys config merge file` is the only working path.
- A merge **creates** an iApp service without running its implementation. Force
  it with `tmsh modify sys application service <name> execute-action definition`.
- iApp implementations log via `tmsh::log <level> <msg>` — a level keyword, not
  iRules' `local0.` — and info-level messages never reach `/var/log/ltm`. Use
  `err`.
- syslog collapses repeated identical lines. Emit one joined line per probe, not
  one line per case.
- A literal unbalanced `}` breaks the *tmsh config parser* before the Tcl parser
  ever sees it. Reach those cases by building the script text at runtime and
  passing it to `eval` (see `irules/interpreter/evprobe.conf`).
- Stubbing `unknown` in a tclsh control silently swallows misuse of *builtins*
  too — an `else` command, for instance — and manufactures false agreement.
  Stub only the iRule-specific commands.
- `RULE_INIT` runs once per TMM, so expect four identical log lines.

## Traffic lab

`irules/traffic-lab/` needs a second host as client and backend. On the lab
appliance that is `dev` (192.168.9.80), on the same subnet as the `internal`
vlan. Four alternatives do **not** work:

- pool member = the BIG-IP self IP → `01070080: already in use as a self IP address`
- pool member = loopback → `01020061: loopback not allowed`
- VIP targeting another VIP on the same box → `LB_SELECTED` fires, connection never loops back
- a secondary IP on the host's `internal` interface → TMM's packets never reach it on a 1-NIC VE

`source-address-translation { type automap }` is required or the return path is
asymmetric. Start the backend with `setsid nohup … < /dev/null &`; a plain
`nohup … &` dies when the ssh session exits.

## Four-context parity

```sh
python3 lib/gen-context-parity.py suites/10-context-parity.cases irules/context-parity
scp irules/context-parity/* lib/e4-context-probe.sh bigip1:/var/tmp/
ssh bigip1 'bash /var/tmp/e4-context-probe.sh' > results/10-context-parity.txt
```

Captured as a single clean run across all four contexts (see
`results/10-context-parity.txt`). Two traps this probe walked into on the way,
both preserved as cases:

- `tcl_patchLevel` does not exist in a `cli script`, so an unguarded read aborts
  that context only.
- iRules resolve command names **at rule load, even inside `catch`**, so a
  literal reference to a command TMM lacks rejects the whole rule. Build such
  references dynamically (`set c ns::cmd; eval $c`). The standalone case is
  `irules/context-parity/ctx_unknown_cmd.conf`.

## Cleanup

The 2026-08-26 run used `probe_*` / `lab_*` prefixes and was verified clean
afterwards (no residual rules, virtuals, pools, or `/var/tmp` files), but it did
**not** implement the §E4 contract — no `__tcl_lsp_probe_*` prefix, no per-create
absence check, no `EXIT` trap, and the traffic lab deliberately attached rules to
virtual servers. `irules/f3-matrix/` is the exception: it uses the
`__tcl_lsp_probe_*` prefix with a collision check before every create and an
absence proof after every delete. `save sys config` was never run in any part of
the run. See the measurements doc's methodology section for the full delta.
