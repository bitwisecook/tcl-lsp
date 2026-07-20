export const meta = {
  name: 'tcl-lsp-differential-audit',
  description: 'Oracle-vs-LSP differential audit of 105 mined real-world Tcl patterns',
  phases: [
    { title: 'Audit' },
  ],
}

const N = 105
const FINDINGS_FILE = '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/trimmed_findings.json'
const AUDIT_DIR = '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/audit'
const REPO = '/home/user/tcl-lsp'

const VERDICT_SCHEMA = {
  type: 'object',
  properties: {
    idx: { type: 'number' },
    corpus: { type: 'string' },
    feature: { type: 'string' },
    verdict: { type: 'string', enum: ['CONFIRMED', 'PLAUSIBLE', 'REFUTED', 'INCONCLUSIVE'] },
    summary: { type: 'string' },
    repro_path: { type: 'string' },
    oracle_output: { type: 'string' },
    lsp_output: { type: 'string' },
    failure_scenario: { type: 'string' },
    root_cause_hint: { type: 'string' },
    severity: { type: 'string', enum: ['none', 'low', 'medium', 'high', 'critical'] },
  },
  required: ['idx', 'corpus', 'feature', 'verdict', 'summary', 'severity'],
}

function auditPrompt(i) {
  return `You are differentially auditing the Tcl Language Server in /home/user/tcl-lsp (native Rust server, already built at target/debug/tcl-lsp-server) against a specific real-world Tcl pattern mined from an open-source project, to find genuine LSP correctness bugs (or confirm the behaviour is actually fine).

## Setup already done for you (do not redo)
- Real tclsh oracles are on PATH: \`tclsh9.0\` (Tcl 9.0.4, built from source with the right LD_LIBRARY_PATH baked into a wrapper) and \`tclsh8.6\` (Debian package). Use tclsh9.0 as the default truth oracle; use tclsh8.6 too if the pattern is version-sensitive.
- The LSP server is built: ${REPO}/target/debug/tcl-lsp-server (debug profile). Do NOT rebuild it.
- The test client skill at ${REPO}/.claude/skills/lsp-client/lsp_client.py is FIXED and reliable — it correctly answers the server's \`workspace/configuration\` pull now, so you do not need to work around any hangs. Use it via:
  \`cd <your-scratch-workspace-dir> && python3 ${REPO}/.claude/skills/lsp-client/lsp_client.py <subcommand> <relative-file> <line 0-based> <col 0-based>\`
  Subcommands you'll likely need: \`definition\`, \`references\`, \`hover\`, \`rename\` (if it exists — check \`--help\`; if not available use \`code-actions\`/\`diagnostics\`/\`symbols\` as relevant), \`diagnostics\`, \`symbols\`. Always \`cd\` into the workspace root first (rootUri = cwd) and pass file paths relative to it — do not use \`--server-dir\`.
- For a fast, no-JSON-RPC look at how the compiler resolves something, you can also use: \`cd ${REPO} && TCL_EXPLORE_BIN=${REPO}/target/debug/tcl python3 .claude/skills/compiler-explorer/explore.py <view> --source '...'\` (views: cst, ir, cfg, ssa, opt, etc. — see \`--help\`).

## Your assigned finding
Read the JSON array at ${FINDINGS_FILE} (a plain JSON file, use Read or \`python3 -c "import json;print(json.load(open('${FINDINGS_FILE}'))[${i}])"\`) and take the object at index ${i}. It has: corpus (project name), corpusPath (absolute path to the real project source on disk — read-only, do not modify), feature (the tricky-Tcl-surface category), file + lineStart/lineEnd (where the pattern lives in the real project), and note (why a static resolver could get this wrong). Read that real file/line-range with the Read tool to see the actual surrounding code in full context — the note alone may be incomplete.

## What to do
1. Understand the real pattern from its actual source context (not just the note).
2. Build a MINIMAL, SELF-CONTAINED, RUNNABLE reproduction under a fresh directory ${AUDIT_DIR}/${i}/ — one or a small handful of .tcl files that capture the *essential* tricky mechanic (namespace shape, indirection, TclOO mixin, trace, rename, etc.), stripped of unrelated domain logic (SPICE/optimisation/charting specifics). Keep it as small as you can while still being a faithful, runnable analogue of the real pattern — it must actually execute correctly under tclsh and produce a clear, checkable result (e.g. \`puts\` a value, or a proc/command that either exists or doesn't).
3. Run your repro under \`tclsh9.0\` (and tclsh8.6 if version-sensitive) to establish ground truth: what does the command/variable/namespace actually resolve to at runtime, what real command actually gets called, etc.
4. Drive the LSP against the identical repro for whichever feature(s) are actually meaningful to check here — usually go-to-definition and/or find-references at the relevant call/declaration site, but use hover/diagnostics/symbols/rename if those are what the pattern actually stresses (e.g. a diagnostic false-positive, a rename correctness question, a missing/wrong symbol in the outline).
5. Compare: does the LSP's answer match what real tclsh execution proves is correct? Note: plenty of these patterns are *intentionally* undecidable statically (dynamic command names built from runtime data with no literal anywhere, e.g. index 12/13/31/32/33/49 in earlier notes) — for those, the CORRECT LSP behaviour is to abstain (no false claim), not to somehow guess right. Only call something CONFIRMED if the LSP produces a definitively WRONG answer (wrong location, false "not found" when a *statically determinable* candidate exists, a false diagnostic, a rename that would break running code) or MISSES something that is genuinely statically resolvable (even if only after simulating one level of namespace/mixin/ensemble mechanics — most of these findings are that: hard but not undecidable). A confident, deliberate abstention on a truly dynamic case is CORRECT behaviour, not a bug — mark REFUTED (or note it as such) rather than CONFIRMED.

## Verdict rules
- CONFIRMED: you reproduced a concrete, deterministic mismatch between LSP output and tclsh-oracle-proven ground truth, on a case that IS statically resolvable. Give the exact repro, the exact LSP command + its output, the exact oracle output, and — if you can tell from a quick look at the relevant Rust source (rust/tcl-compiler/src/analyser/, rust/tcl-lsp-core/, rust/tcl-lsp-server/src/lib.rs, rust/tcl-registry/) — a root-cause hint (file/function).
- PLAUSIBLE: you see a strong reason to suspect a bug but couldn't fully pin it down (e.g. inconsistent results, or you ran out of time to isolate a truly minimal repro). Explain what's suspicious and what you'd check next.
- REFUTED: you built the repro and the LSP handled it correctly (including correctly abstaining on a truly dynamic case). This is a valid, useful outcome — don't strain to manufacture a problem.
- INCONCLUSIVE: you could not get a clean repro running (e.g. the mechanic depends on something you couldn't isolate in a small repro) — say why.

Be adversarial toward your own findings before calling something CONFIRMED — re-run it, double check line/column numbers are 0-based and land exactly on the right token, and rule out that you're mis-using the harness (e.g. wrong cwd) rather than hitting a real bug.

Report via the required structured schema. Keep summary/failure_scenario concise but concrete (exact commands + outputs beat prose).`
}

phase('Audit')
const indices = Array.from({length: N}, (_, i) => i)
const results = await parallel(indices.map(i => () =>
  agent(auditPrompt(i), { label: `audit:${i}`, schema: VERDICT_SCHEMA })
))

const flat = results.filter(Boolean)
const confirmed = flat.filter(r => r.verdict === 'CONFIRMED')
const plausible = flat.filter(r => r.verdict === 'PLAUSIBLE')
log(`Audit done: ${flat.length}/${N} returned, ${confirmed.length} CONFIRMED, ${plausible.length} PLAUSIBLE`)

return { all: flat, confirmed, plausible }
