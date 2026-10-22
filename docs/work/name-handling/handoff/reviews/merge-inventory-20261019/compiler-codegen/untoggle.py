import re
R='/workspace/naming-migration-active/tcl-lsp/'
def edit(p, pairs):
    s=open(R+p).read()
    for a,b in pairs:
        assert a in s, (p, a[:80]); s=s.replace(a,b,1)
    open(R+p,'w').write(s)
T='// TEMP measurement toggle (compiler-codegen), remove before hand-back.\n'
edit('rust/tcl-compiler/src/command_binding/compiled_preflight.rs',[
('''    let native_entry_unavailable = state.baseline.native_entry.is_none()
        '''+T+'''        && (state.baseline.unknown_entry || std::env::var_os("ZZ_CCG_BASELINE").is_some());''',
'''    let native_entry_unavailable =
        state.baseline.native_entry.is_none() && state.baseline.unknown_entry;'''),
('''    #[track_caller]
    fn require_provider(&mut self) {
        if std::env::var_os("ZZ_CCG_PROVIDER").is_some() {
            eprintln!("ZZ_CCG_PROVIDER {}", std::panic::Location::caller());
        }
        self.possible_error = true;''','''    fn require_provider(&mut self) {
        self.possible_error = true;'''),
])
edit('rust/tcl-compiler/src/command_binding/compiled_preflight/original_preparation.rs',[
('''            '''+T+'''            None if !baseline.unknown_entry && std::env::var_os("ZZ_CCG_BASELINE").is_none() => baseline''',
'''            None if !baseline.unknown_entry => baseline'''),
])
edit('rust/tcl-compiler/src/codegen/control_flow.rs',[
('''        '''+T+'''        if std::env::var_os("ZZ_CCG_BASELINE").is_some() {
            return None;
        }
''',''),
])
edit('rust/tcl-spectcl/src/install.rs',[
('''    '''+T+'''    if std::env::var_os("ZZ_CCG_BASELINE").is_some() {
        return;
    }
''',''),
('''    '''+T+'''    if std::env::var_os("ZZ_CCG_BASELINE").is_some() {
        return None;
    }
''',''),
])
edit('rust/tcl-registry/src/native_string_operand_compilation.rs',[
('''    '''+T+'''    if operation == NativeStringOperandOperation::Cat
        && std::env::var_os("ZZ_CCG_BASELINE").is_some()
    {
        return Selection::Unknown;
    }
''',''),
])
edit('rust/tcl-compiler/src/optimiser/manager.rs',[
('''            '''+T+'''            .or_else(|| {
                std::env::var_os("ZZ_CCG_BASELINE")
                    .is_none()
                    .then(|| registry.profile())
                    .flatten()
            }),''','''            .or_else(|| registry.profile()),'''),
])
p='rust/tcl-compiler/src/compile_service.rs'
s=open(R+p).read()
pairs=[
('''            Some(_) => self.registry.registry()?,
            '''+T+'''            None if std::env::var_os("ZZ_CCG_BASELINE").is_some() => self.registry.registry()?,
''','''            Some(_) => self.registry.registry()?,
'''),
('''        '''+T+'''        let zz_baseline = std::env::var_os("ZZ_CCG_BASELINE").is_some();
        let fresh_profile = if zz_baseline {
            profile.unwrap_or_else(DialectProfile::plain_tcl)
        } else {
            profile.unwrap_or_else(profile_less_engine)
        };
        let fresh_input = (native_entry.is_none()
            && (!zz_baseline || profile.is_some())
            && policy.scope == ModuleEmissionScope::WholeModule)''','''        let fresh_profile = profile.unwrap_or_else(profile_less_engine);
        let fresh_input = (native_entry.is_none()
            && policy.scope == ModuleEmissionScope::WholeModule)'''),
('''        let compiled = |ir| {
            Self::finish_module_compile(ir, target, registry, config, profile, policy, zz_baseline)
        };''','''        let compiled =
            |ir| Self::finish_module_compile(ir, target, registry, config, profile, policy);'''),
]
for a,b in pairs:
    assert a in s, (p,a[:80]); s=s.replace(a,b,1)
# remove zz_baseline arm
i=s.index('''            Some(input) if zz_baseline => {''')
j=s.index('''            Some(input) => {''', i)
s=s[:i]+s[j:]
pairs=[
('''        policy: CompileSourcePolicy<'_>,
        zz_baseline: bool,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {''','''        policy: CompileSourcePolicy<'_>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {'''),
('''        if zz_baseline {
            if let Some(profile) = profile {
                module.profile = profile;
            }
        } else if !std::ptr::eq(module.profile, profile.unwrap_or_else(DialectProfile::plain_tcl))
        {''','''        let carried = profile.unwrap_or_else(DialectProfile::plain_tcl);
        if !std::ptr::eq(module.profile, carried) {'''),
('''            module.profile = profile.unwrap_or_else(DialectProfile::plain_tcl);
            module.manifest''','''            module.profile = carried;
            module.manifest'''),
]
for a,b in pairs:
    assert a in s, (p,a[:80]); s=s.replace(a,b,1)
# remove probe tests
for name in ['zz_ccg_release_probe','zz_ccg_o129_probe']:
    i=s.index('    #[test]\n    fn '+name+'()')
    j=s.index('\n    }\n', s.index('    fn '+name))
    # find end of fn: next line "    }\n" at indentation 4 after body; use brace matching
    k=s.index('{', s.index('fn '+name))
    depth=0
    for m in range(k, len(s)):
        if s[m]=='{': depth+=1
        elif s[m]=='}':
            depth-=1
            if depth==0: end=m+1; break
    s=s[:i].rstrip(' ')+s[end:].lstrip('\n')
s=s.rstrip()+'\n'
open(R+p,'w').write(s)
import os
if os.path.exists(R+"rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs"): os.remove(R+"rust/tcl-spectcl/tests/zz_ccg_fold_probe.rs")
print("ok")
