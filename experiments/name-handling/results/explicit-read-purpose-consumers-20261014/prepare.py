from pathlib import Path
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-explicit-read-purpose-consumers-20261014');changes={}
def edit(path,old,new):
 before=root.joinpath(path).read_text();after=changes.get(path,(before,before))[1];assert after.count(old)==1,(path,after.count(old),old[:80]);changes[path]=(before,after.replace(old,new))
p='rust/tcl-compiler/src/var_refs.rs';s=root.joinpath(p).read_text();start=s.index('/// Every argument of `cmd` whose registry role names a variable');end=s.index('#[must_use]\n#[cfg(test)]',start)
edit(p,s[start:end],'''/// Test-only catalogue convenience for detached command layouts.
/// Production roles require [`original_variable_name_role_words`] ownership.
''')
edit(p,'''    /// Whether this scanner reports element-qualified names
''','''    /// Whether this caller requested detached catalogue read roles.
    /// The flag supplies no original source or receiver authority.
    pub(crate) const fn standalone_roles_requested(&self) -> bool {
        self.options.include_var_read_roles
    }

    /// Whether this scanner reports element-qualified names
''')
edit(p,'''    /// Shared cache lookup/insert for both scan modes.
''','''    /// Script syntax alone; command heads grant no receiver read roles.
    pub fn scan_lexical_script(&mut self, source: &str, registry: &CommandRegistry) -> BTreeSet<String> {
        self.scan_cached_roles(source, registry, false, false)
    }

    /// Detached script catalogue advice may protect stores but proves no binding.
    pub fn scan_standalone_script(&mut self, source: &str, registry: &CommandRegistry) -> BTreeSet<String> {
        self.scan_cached_roles(source, registry, false, true)
    }

    /// Shared cache lookup/insert for both scan modes.
''')
p='rust/tcl-compiler/src/inlining/frame.rs';edit(p,'.scan_word(text, self.source.registry)','.scan_lexical_word(text, self.source.registry)')
p='rust/tcl-compiler/src/optimiser/elimination/source_reads.rs';edit(p,'scanner.scan_script(text, registry)','scanner.scan_lexical_script(text, registry)')
p='rust/tcl-compiler/src/ir_helpers/materialized_footprint.rs';edit(p,'scanner.scan_script(text, context.registry)','scanner.scan_lexical_script(text, context.registry)');edit(p,'scanner.scan_word(&expression, context.registry)','scanner.scan_lexical_word(&expression, context.registry)')
p='rust/tcl-compiler/src/ssa.rs'
edit(p,'''    fn normal(
''','''    /// Supplied source retains its own role owner; detached requested roles
    /// are confined to the independently selected standalone utility route.
    fn word_reads(
        self,
        scanner: &mut VarReferenceScanner,
        text: &str,
    ) -> BTreeSet<String> {
        if self.standalone && scanner.standalone_roles_requested() {
            scanner.scan_standalone_word(text, self.registry)
        } else {
            scanner.scan_lexical_word(text, self.registry)
        }
    }

    fn normal(
''')
# The source-aware context owns every production word-read call.
s=changes[p][1];end=s.index('\n#[cfg(test)]\nmod tests {');production=s[:end];tests=s[end:]
old='''    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> BTreeSet<String> {
    // naming.variable.selected-setter-expression-reads''';new='''    scanner: &mut VarReferenceScanner,
    selection: SsaInvocationContext<'_>,
) -> BTreeSet<String> {
    let registry = selection.registry;
    // naming.variable.selected-setter-expression-reads''';assert production.count(old)==1;production=production.replace(old,new)
old='set_value_reads(value, tokens.as_ref(), scanner, registry)';assert production.count(old)==1;production=production.replace(old,'set_value_reads(value, tokens.as_ref(), scanner, selection)')
import re
calls=re.findall(r'scanner\.scan_word\(([^\n]*?), registry\)',production);assert len(calls)==18,len(calls)
production=re.sub(r'scanner\.scan_word\(([^\n]*?), registry\)',r'selection.word_reads(scanner, \1)',production)
# A retained setter's children already own by-name roles independently.
old='''    if tokens.is_some_and(|tokens| tokens.source_binding.is_some()) {
        return selection.word_reads(scanner, value);
    }''';assert production.count(old)==1;production=production.replace(old,'''    if tokens.is_some_and(|tokens| tokens.source_binding.is_some()) {
        return scanner.scan_lexical_word(value, registry);
    }''')
# Cache-isolation/control test exercises real withheld source input and both modes.
old='''        assert_eq!(
            scanner.scan_word("$lexical [info exists nominal]", registry),
            BTreeSet::from(["lexical".to_owned()])
        );''';assert tests.count(old)==1
tests=tests.replace(old,old+'''
        let mut requested = VarReferenceScanner::with_config(
            VarScanOptions { include_var_read_roles: true, ..Default::default() },
            function.source_lexer_config(),
        );
        assert_eq!(unavailable.word_reads(&mut requested, "$lexical [info exists nominal]"),
            BTreeSet::from(["lexical".to_owned()]));
        assert!(SsaInvocationContext::standalone(registry)
            .word_reads(&mut requested, "$lexical [info exists nominal]").contains("nominal"));
        assert_eq!(unavailable.word_reads(&mut requested, "$lexical [info exists nominal]"),
            BTreeSet::from(["lexical".to_owned()]));''')
changes[p]=(changes[p][0],production+tests)
p='docs/design/contracts/shared-utility-contracts-rust.md'
edit(p,'`VarReferenceScanner::scan_lexical_word`; `VarReferenceScanner::scan_standalone_word`',
'`VarReferenceScanner::scan_lexical_word`; `VarReferenceScanner::scan_standalone_word`; `VarReferenceScanner::scan_lexical_script`; `VarReferenceScanner::scan_standalone_script`')
for path,(before,after) in changes.items():
 for side,value in [('before',before),('after',after)]:
  dest=out/side/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(value)
print(list(changes))
