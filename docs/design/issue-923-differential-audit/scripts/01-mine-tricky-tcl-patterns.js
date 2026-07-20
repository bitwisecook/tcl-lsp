export const meta = {
  name: 'mine-tricky-tcl-patterns',
  description: 'Mine real-world tricky Tcl patterns from 8 reference corpora for LSP verification',
  phases: [
    { title: 'Mine' },
  ],
}

const FEATURES = `
- namespaces (nested namespace eval, namespace path, namespace ensemble, namespace export/import, namespace parent/current/qualifiers)
- rename (renaming builtins, renaming procs, rename-based method/hook shims, rename to {} to delete)
- unknown (custom ::unknown overrides, auto-loading fallback shims)
- aliasing (interp alias, proc-based aliasing, command aliasing via rename+wrapper)
- safe interpreters / sub-interpreters (interp create, interp create -safe, interp eval, interp expose/hide, interp alias across interps)
- tracing (trace add variable/command/execution, trace remove)
- tricky indirection (command names stored in variables and invoked via $cmd, dispatch tables/arrays mapping strings to commands, [subst]-built command names, {*}-expansion of a computed command+args list)
- TclOO (oo::class create, oo::define, method/constructor/destructor, inheritance across files, mixins, filters, oo::objdefine per-object methods, next/nextto, self, my)
- upvar (upvar to caller frame, upvar #0, upvar N with computed N, linking to array elements)
- uplevel (uplevel #0, uplevel 1, uplevel with computed level, using uplevel to splice caller-scope commands)
- eval (eval of constructed strings, eval {*}$list, eval in a different namespace)
- the ::tcl namespace and ::tcl::mathop / ::tcl::mathfunc namespaces (fully-qualified use of tcl::mathop::+ etc, mathfunc procs used from expr)
- source (sourcing files into specific namespaces, computed/conditional source paths, source inside namespace eval)
- package loading (package require/provide, pkgIndex.tcl ifneeded, package ifneeded with version constraints, lazy package loading)
- autoIndex / auto_load (tclIndex generation or reliance, auto_path manipulation, auto_import)
- args in proc definitions (variadic 'args' parameter, default parameter values, unusual parameter names, argument count validation patterns)
`

const CORPORA = [
  { key: 'SpiceGenTcl', path: '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/corpora/georgtree_SpiceGenTcl', note: "georgtree's project — the issue-923 reporter's own codebase; pure-Tcl 9.0, heavy TclOO for circuit-element classes" },
  { key: 'argparse', path: '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/corpora/georgtree_argparse', note: "georgtree's project — argument-parsing preprocessor, likely namespace-ensemble and indirection heavy" },
  { key: 'tclopt', path: '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/corpora/georgtree_tclopt', note: "georgtree's project — Tcl wrapper for optimisation routines" },
  { key: 'ticklecharts', path: '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/corpora/nico-robert_ticklecharts', note: "nico-robert's project — large pure-Tcl wrapper around Apache ECharts, many namespaces/procs" },
  { key: 'pix', path: '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/corpora/nico-robert_pix', note: "nico-robert's project — Tcl/Tk wrapper around a graphics library, cffi-based, likely TclOO type wrappers" },
  { key: 'tomato', path: '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/corpora/nico-robert_tomato', note: "nico-robert's project — math::geometry package" },
  { key: 'tk', path: '/tmp/claude-0/-home-user-tcl-lsp/b4c77bf3-47fd-5ba4-822a-000fa233bcc1/scratchpad/corpora/tcltk_tk/library', note: "Tk's own Tcl-level library scripts (library/*.tcl, library/demos/) — canonical megawidget/dialog code, heavy on namespace ensembles, rename-based rebinding, bind-based indirection" },
  { key: 'tcllib', path: '/home/user/tcl-lsp/tmp/tcllib-2.0/modules', note: "tcllib 2.0 modules/ — the Tcl community standard library, extremely broad namespace/OO usage across ~100 packages" },
]

phase('Mine')
const results = await parallel(CORPORA.map(c => () => agent(
  `You are mining a real-world Tcl codebase for concrete, realistic examples of hard-to-analyse Tcl language surfaces, to build a verification corpus for a Tcl Language Server's name-resolution engine.

Corpus: "${c.key}" at directory: ${c.path}
Context: ${c.note}

Search this corpus (.tcl files; for large corpora like tcllib, sample broadly across subdirectories rather than exhaustively reading every file — use Grep for feature signatures first, then Read the surrounding context for the best examples) for real, non-trivial usages of each of these feature dimensions:
${FEATURES}

For each feature dimension where you find a genuinely interesting real example (skip a dimension entirely if the corpus has no real usage of it — do not invent examples), report the 1-4 BEST examples: the ones that are hardest for a static analyser to get right (e.g. a proc called across files by a namespace-relative name, a command name computed and stored in a variable then dispatched, a TclOO method resolved through multiple inheritance/mixins, a trace callback added dynamically, a namespace ensemble built at runtime, rename used to layer a wrapper over a builtin, args with defaults consumed via upvar).

For each example, capture:
- feature: the dimension key from the list above (short slug, e.g. "tricky_indirection", "tclOO", "namespaces", "rename", "unknown", "aliasing", "safe_interp", "tracing", "upvar", "uplevel", "eval", "tcl_mathop", "source", "package_loading", "autoindex", "proc_args")
- file: path relative to the corpus root
- lineStart / lineEnd: the line range of the interesting snippet
- snippet: the actual code (trimmed to the essential ~5-25 lines, include enough surrounding context to be self-explanatory)
- note: one or two sentences on exactly why this is tricky for a static Tcl analyser (what would a naive lexical/textual resolver get wrong here, e.g. "the target proc is only reachable by simulating namespace path fallback across two files" or "the dispatch table is built with [namespace code] so the callback's home namespace is not lexically obvious")

Be selective and concrete — quality over quantity. Do not pad with trivial or uninteresting examples just to cover every dimension. Return your findings via the required structured schema.`,
  {
    label: `mine:${c.key}`,
    schema: {
      type: 'object',
      properties: {
        corpus: { type: 'string' },
        findings: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              feature: { type: 'string' },
              file: { type: 'string' },
              lineStart: { type: 'number' },
              lineEnd: { type: 'number' },
              snippet: { type: 'string' },
              note: { type: 'string' },
            },
            required: ['feature', 'file', 'snippet', 'note'],
          },
        },
      },
      required: ['corpus', 'findings'],
    },
  }
)))

return results.filter(Boolean)
