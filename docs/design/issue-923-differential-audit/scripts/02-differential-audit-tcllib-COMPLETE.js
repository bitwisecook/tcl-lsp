export const meta = {
  name: 'tcl-lsp-differential-audit-tcllib',
  description: 'Oracle-vs-LSP differential audit of 25 mined tcllib patterns (wave 2)',
  phases: [
    { title: 'Audit' },
  ],
}

const N = 25
const FINDINGS_FILE = '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/tcllib_trimmed.json'
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

function auditPrompt(localIdx) {
  return `You are differentially auditing the Tcl Language Server in /home/user/tcl-lsp (native Rust server, already built at target/debug/tcl-lsp-server) against a specific real-world Tcl pattern mined from tcllib (the Tcl community standard library), to find genuine LSP correctness bugs (or confirm the behaviour is actually fine).

## Setup already done for you (do not redo)
- Real tclsh oracles are on PATH: \`tclsh9.0\` (Tcl 9.0.4, built from source, correct LD_LIBRARY_PATH baked into the wrapper) and \`tclsh8.6\` (Debian package). Use tclsh9.0 as the default truth oracle; use tclsh8.6 too if the pattern is version-sensitive.
- The LSP server is built: ${REPO}/target/debug/tcl-lsp-server (debug profile). Do NOT rebuild it.
- The test client skill at ${REPO}/.claude/skills/lsp-client/lsp_client.py is FIXED and reliable — it correctly answers the server's \`workspace/configuration\` pull now. Use it via:
  \`cd <your-scratch-workspace-dir> && python3 ${REPO}/.claude/skills/lsp-client/lsp_client.py <subcommand> <relative-file> <line 0-based> <col 0-based>\`
  Subcommands: \`definition\`, \`references\`, \`hover\`, \`diagnostics\`, \`symbols\`, \`code-actions\`. Always \`cd\` into the workspace root first (rootUri = cwd) and pass file paths relative to it — do not use \`--server-dir\`.
- For a fast, no-JSON-RPC look at how the compiler resolves something: \`cd ${REPO} && TCL_EXPLORE_BIN=${REPO}/target/debug/tcl python3 .claude/skills/compiler-explorer/explore.py <view> --source '...'\` (views: cst, ir, cfg, ssa, opt, etc.).

## Your assigned finding
Read the JSON array at ${FINDINGS_FILE} (a plain JSON file) and take the object at index ${localIdx}. It has: corpus, corpusPath (absolute path to tcllib's modules/ directory on disk — read-only), feature (the tricky-Tcl-surface category), file + lineStart/lineEnd (where the pattern lives in the real tcllib source), and note (why a static resolver could get this wrong). Read that real file/line-range with the Read tool to see the actual surrounding code in full context — the note alone may be incomplete.

## What to do
1. Understand the real pattern from its actual source context (not just the note).
2. Build a MINIMAL, SELF-CONTAINED, RUNNABLE reproduction under a fresh directory ${AUDIT_DIR}/${105 + localIdx}/ — one or a small handful of .tcl files that capture the *essential* tricky mechanic, stripped of unrelated tcllib-specific logic. Keep it small but faithful and runnable.
3. Run your repro under \`tclsh9.0\` (and tclsh8.6 if version-sensitive) to establish ground truth.
4. Drive the LSP against the identical repro for whichever feature(s) are meaningful here (definition/references/hover/diagnostics/rename as appropriate).
5. Compare: does the LSP's answer match tclsh-oracle-proven ground truth? Many tcllib patterns are *intentionally* undecidable statically (dynamic dispatch tables, computed command names with no literal anywhere) — for those, correct LSP behaviour is to abstain, not guess. Only call something CONFIRMED if the LSP produces a definitively WRONG answer on a case that IS statically resolvable, or misses something genuinely resolvable (even if only after simulating one level of namespace/ensemble/mixin mechanics).

## Verdict rules
- CONFIRMED: reproduced a concrete, deterministic mismatch between LSP output and oracle-proven ground truth on a statically resolvable case. Give the exact repro, exact LSP command + output, exact oracle output, and if possible a root-cause hint (Rust file/function in rust/tcl-compiler/src/analyser/, rust/tcl-lsp-core/, rust/tcl-lsp-server/src/lib.rs, rust/tcl-registry/).
- PLAUSIBLE: strong suspicion, not fully pinned down — explain what's suspicious and what to check next.
- REFUTED: LSP handled it correctly (including correctly abstaining on a genuinely dynamic case) — a valid, useful outcome.
- INCONCLUSIVE: couldn't get a clean repro running — say why.

Be adversarial toward your own findings before calling something CONFIRMED — re-run it, double-check 0-based line/col land exactly on the right token, and rule out harness misuse before concluding it's a real bug.

Report via the required structured schema. Keep summary/failure_scenario concise but concrete.`
}

phase('Audit')
const indices = Array.from({length: N}, (_, i) => i)
const results = await parallel(indices.map(i => () =>
  agent(auditPrompt(i), { label: `audit-tcllib:${105 + i}`, schema: VERDICT_SCHEMA })
))

const flat = results.filter(Boolean)
const confirmed = flat.filter(r => r.verdict === 'CONFIRMED')
const plausible = flat.filter(r => r.verdict === 'PLAUSIBLE')
log(`tcllib audit done: ${flat.length}/${N} returned, ${confirmed.length} CONFIRMED, ${plausible.length} PLAUSIBLE`)

return { all: flat, confirmed, plausible }
