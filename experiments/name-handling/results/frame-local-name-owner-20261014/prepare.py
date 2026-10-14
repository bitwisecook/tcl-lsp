from pathlib import Path
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-frame-local-name-owner-20261014');changes={}
def edit(path,old,new):
 before=root.joinpath(path).read_text();after=changes.get(path,(before,before))[1];assert after.count(old)==1,(path,after.count(old),old[:80]);changes[path]=(before,after.replace(old,new))
p='rust/tcl-syntax/src/naming.rs';edit(p,'''/// The TextMate-compatible variable-name body used by the generated editor
''','''/// Whether a source variable name belongs to an ordinary activation's table.
/// C qualifiers select namespace cells; Jim only roots a name with initial `::`.
/// The empty name remains local. The caller owns the policy and input extent;
/// this supplies no Native frame, cell, lookup or variable-presence receipt.
#[must_use]
pub fn source_variable_name_is_frame_local(
    name: &[u8],
    policy: tcl_dialect::VariableLookupPolicy,
) -> bool {
    match policy {
        tcl_dialect::VariableLookupPolicy::Tcl => !is_qualified(name),
        tcl_dialect::VariableLookupPolicy::Jim => !name.starts_with(b"::"),
    }
}

/// The TextMate-compatible variable-name body used by the generated editor
''')
p='rust/tcl-compiler/src/inlining/frame.rs';edit(p,'''    pub(super) config: LexerConfig,
    image: &'a tcl_lexer::SourceImage,''','''    pub(super) config: LexerConfig,
    variable_lookup: tcl_dialect::VariableLookupPolicy,
    image: &'a tcl_lexer::SourceImage,''')
edit(p,'''        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
''','''        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
        let variable_lookup = module.source_entry.invocation_dialect?.variable_lookup_policy?;
''')
edit(p,'''                config: module.lexer_config,
                image: &module.source,''','''                config: module.lexer_config,
                variable_lookup,
                image: &module.source,''')
edit(p,'''/// A name that is not a local of the frame — one a namespace qualifies — is not
/// the frame's to bind or to read.
fn is_local(name: &str) -> bool {
    !name.is_empty() && !name.contains("::")
}

fn bind(bound: &mut HashSet<String>, name: &str) {
    if is_local(base(name)) {''','''fn bind(
    bound: &mut HashSet<String>,
    name: &str,
    policy: tcl_dialect::VariableLookupPolicy,
) {
    if tcl_syntax::naming::source_variable_name_is_frame_local(base(name).as_bytes(), policy) {''')
edit(p,'''fn read(name: &str, bound: &HashSet<String>) -> Option<()> {
    let name = base(name);
    (!is_local(name) || bound.contains(name)).then_some(())
}''','''fn read(
    name: &str,
    bound: &HashSet<String>,
    policy: tcl_dialect::VariableLookupPolicy,
) -> Option<()> {
    let name = base(name);
    (!tcl_syntax::naming::source_variable_name_is_frame_local(name.as_bytes(), policy)
        || bound.contains(name)).then_some(())
}''')
# Only these Frame member calls consume the retained policy; free helpers remain pure.
b,a=changes[p];import re
a,n=re.subn(r'\bbind\((bound|&mut inner), name\)',r'bind(\1, name, self.source.variable_lookup)',a);assert n==7,n
a,n=re.subn(r'\bread\(name, bound\)',r'read(name, bound, self.source.variable_lookup)',a);assert n==4,n
changes[p]=(b,a)
edit(p,'    fn original_frame_context_keeps_selected_aliases_and_shadow_refusal() {',
'''    fn original_frame_empty_and_dollar_names_require_actual_local_bindings() {
        // Public C8.4–9.1/Jim source controls: experiments/name-handling/results/frame-local-name-geometry.
        for (source, expected) in [
            ("proc p {} {return ${}}", false),
            ("proc p {} {set {} VALUE; return ${}}", true),
            ("proc p {{$name}} {return ${$name}}", true),
            ("proc p {name} {return ${$name}}", false),
            ("proc p {{$name}} {return $name}", false),
        ] {
            let (context, module) = crate::inlining::tests::logical_module_for(source);
            let selected = SourceContext::for_module(&module, context.commands()).unwrap();
            let procedure = module.procedures.get(&fixture_id(&module.procedures, "::p")).unwrap();
            assert_eq!(reads_only_bound_names(procedure, selected), expected, "{source}");
        }
    }

    #[test]
    fn original_frame_context_keeps_selected_aliases_and_shadow_refusal() {''')
p='docs/design/contracts/shared-utility-contracts-rust.md';edit(p,'| Original procedure summary ownership |',
'''| Source activation-local variable geometry | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-compiler/src/inlining/frame.rs` | `source_variable_name_is_frame_local` | actual retained VariableLookupPolicy and independently owned source-name extent; the empty name is local, C qualifiers select namespace storage, Jim relative qualified names stay local, and initial Jim root markers select global storage; inlining retains the unchanged original Module/metadata/driver policy and declines missing ownership; no Native frame, lookup, input extent, presence or edit permission is issued | `xtask-owner-resolution` |
| Original procedure summary ownership |''')
# Preserve the complete fixed source and raw native records, including unsuccessful assembly.
exp='experiments/name-handling/results/frame-local-name-geometry'
source=Path('/tmp/name-frame-empty-oracle-20261014')
for name in ['probe.tcl','results.json','first-six-case-probe.tcl','first-six-case-results.json','eight-case-probe.tcl','eight-case-results.json','producer-error.txt']:
 path=exp+'/'+name;assert not root.joinpath(path).exists();changes[path]=('',source.joinpath(name).read_text())
changes[exp+'/README.md']=('', '''# Frame-local variable-name geometry

These fixed public source cases use C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4,
9.1.0 and the Jim executable built from source revision
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. `results.json` retains exact
argv, executable/source SHA256, exit codes and complete stdout/stderr. The
source bytes are in `probe.tcl`; no tcltest or substitute support library is
used.

Every process rejects an empty formal, errors on an unbound empty variable,
and returns VALUE after writing the empty local. A procedure's empty local
can shadow an existing empty global without modifying it; an unbound procedure
read does not inherit that global. Literal `$name` formals bind `$name`, while
`name` and `$name` remain distinct. C rejects a relative `inner::value` write
when its namespace is absent; Jim stores that exact relative name locally.
The C8.4 empty-formal error includes the procedure name, unlike the other
captured processes.

The shared `source_variable_name_is_frame_local` consumes the independently
selected source variable-lookup policy. Original Logical inlining controls
check empty and dollar-name binding before eligibility. These observations
establish no Native allocation identity, compiler opcode, activation, rewrite
or inlining equivalence. F5/TMM behavior is unobserved.

The six-case and eight-case source/raw captures remain separate. The
source-retention producer error and unchanged eight-case rerun are recorded in
`producer-error.txt`; the final capture adds the independent global/local
empty-name controls.
''')
q='experiments/name-handling/f5/questions/frame-local-name-geometry.md';assert not root.joinpath(q).exists();changes[q]=('', '''# F5 procedure-local name geometry

Status: unanswered. No BIG-IP/TMM behavior is inferred from C Tcl or Jim.

Run the fixed cases in
`experiments/name-handling/results/frame-local-name-geometry/probe.tcl` only
in an isolated test iRule/harness, with unique harness procedure names. Record
the exact adapted source and any setup refusal. A tmsh shell or Linux tclsh
answer is a separate context and cannot answer TMM procedure behavior.

Record BIG-IP release/build, interpreter context, event and rule/namespace
scope, deployment/harness channel, exact source/argv, exit/completion codes and
full output/error text. Answer these questions separately:

- Does an empty procedure-local name shadow an empty global without changing it?
- Does an unbound procedure read of `${}` ignore an existing empty global?
- Are an empty formal, `$name` formal and `name` formal accepted, and which exact
  local does each body reference read?
- Is a relative `inner::value` name activation-local or namespace-owned?
- Do interpreter and compiled iRule channels return the same observed results?

Write answers and untouched transcripts under
`experiments/name-handling/f5/answers/frame-local-name-geometry/` on this branch.
Do not substitute results from another context or treat an unavailable case as
success.
''')
for path,(before,after) in changes.items():
 for side,value in [('before',before),('after',after)]:
  dest=out/side/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(value)
print(list(changes))
