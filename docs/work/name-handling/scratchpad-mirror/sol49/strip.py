import re,sys
def sub(p, old, new, count=1, must=True):
    s=open(p).read()
    if old not in s:
        if must: print("MISSING", p, old[:80]); 
        return
    s=s.replace(old,new,count); open(p,'w').write(s)
def rmline(p, pat):
    s=open(p).read().split('\n')
    out=[l for l in s if not re.search(pat,l)]
    open(p,'w').write('\n'.join(out))
# single-line eprintln guards
for p in ['src/irules_checks.rs','src/registry_invocation.rs','src/taint.rs','src/variable_bindings.rs','src/command_binding.rs']:
    rmline(p, r'^\s*if std::env::var_os\("ZZ_SOL49"\)\.is_some\(\) \{ eprintln!.*\}\s*$')
    rmline(p, r'^\s*if std::env::var_os\("ZZ_SOL49"\)\.is_some\(\) && !self\.opaque_domain \{ eprintln!.*\}\s*$')
# dynamic_names
p='src/dynamic_names.rs'
s=open(p).read()
s=s.replace("{ zz_opaque(line!()); *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT); }","*barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);")
s=s.replace("Statement::NativeCall { .. } => { zz_opaque(line!()); *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT) }","Statement::NativeCall { .. } => *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT),")
s=s.replace("zz_opaque(line!()); barrier = DynamicNameBarrier::OPAQUE_SCRIPT;","barrier = DynamicNameBarrier::OPAQUE_SCRIPT;")
s=s.replace("\nfn zz_opaque(line: u32) { if std::env::var_os(\"ZZ_SOL49\").is_some() { eprintln!(\"ZZ dynopaque line {line}\"); } }\n","")
open(p,'w').write(s)
