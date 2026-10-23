from pathlib import Path
import hashlib,json,datetime,difflib,subprocess
root=Path('/workspace/naming-migration-active/tcl-lsp');out=Path('/tmp/ccg-n17-live-first-gate-v1');changes=[]
mac='''macro_rules! n17_live_trace {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        if std::env::var("TCL_LSP_TRACE_N17_FOLD").as_deref() == Ok("first-gate-v1") {
            eprintln!("N17G_LIVE {}", format_args!($($arg)*));
        }
    };
}

macro_rules! n17_live_some {
    ($value:expr, $gate:literal) => {{
        let value = $value;
        n17_live_trace!("gate={} present={}", $gate, value.is_some());
        value
    }};
}

'''
def replace(s,a,b):
 assert s.count(a)==1,(a[:100],s.count(a));return s.replace(a,b,1)
def save(rel,old,new):
 assert old!=new
 for name,txt in [('base',old),('work',new)]:
  p=out/name/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(txt)
 changes.append({'path':rel,'base_sha256':hashlib.sha256(old.encode()).hexdigest()})
# Const substitution's actual outer and argument gates.
rel='rust/tcl-compiler/src/const_subst.rs';old=(root/rel).read_text();s=old;at=s.index('impl ConstSubstCtx<');s=s[:at]+mac+s[at:]
a=s.index('    fn fold_retained_at_depth(');b=s.index('    /// Fold the command-substitution interior',a);section=s[a:b]
section=replace(section,'        if depth > MAX_CONST_SUBST_DEPTH {\n            return None;','        n17_live_trace!("fold-entry span={:?} depth={depth}", call.span);\n        if depth > MAX_CONST_SUBST_DEPTH {\n            n17_live_trace!("decline=fold-depth");\n            return None;')
for x,g in [('call.tokens.as_ref()','fold-tokens'),('tokens.source_binding.as_ref()','fold-binding'),('invocation.dialect','fold-dialect'),('binding.invocation_realm()','fold-realm'),('self.retained_arguments_at_depth(call, calls, depth, context)','fold-arguments'),('arg_refs.get(invocation.facts.argument_offset..)','fold-offset'),('invocation.facts.subcommand.canonical_name()','fold-subcommand-name')]:
 # binding/call repeated in argument function; label every identical original ? independently.
 section=section.replace(x+'?',f'n17_live_some!({x}, "{g}")?')
section=replace(section,'''        binding
            .proved_execution_target()
            .filter(|target| target.registry_backed)?;''','''        n17_live_some!(
            binding.proved_execution_target().filter(|target| target.registry_backed),
            "fold-actual-registry-target"
        )?;''')
section=replace(section,'''        let invocation =
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                self.registry,
                Some(context),
                tokens,
            )?;''','''        let invocation = n17_live_some!(
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                self.registry, Some(context), tokens,
            ), "fold-handler"
        )?;
        n17_live_trace!("selected-command={}", invocation.facts.canonical_command);''')
section=replace(section,'''            && invocation.with_metadata_schema(
                self.registry,
                context,
                n17_live_some!(binding.invocation_realm(), "fold-realm")?,
                |schema| Some(schema.semantics.world_effects.is_declared()),
            )?''','''            && n17_live_some!(invocation.with_metadata_schema(
                self.registry,
                context,
                n17_live_some!(binding.invocation_realm(), "fold-realm")?,
                |schema| Some(schema.semantics.world_effects.is_declared()),
            ), "fold-world-schema")?''')
section=replace(section,'''        {
            return None;
        }
        if invocation.facts.operation''','''        {
            n17_live_trace!("decline=declared-world-footprint");
            return None;
        }
        if invocation.facts.operation''')
section=replace(section,'''        let spec = self.registry.get_for_surface(
            &invocation.facts.canonical_command,
            Some(context.context().authoring_query()),
        )?;''','''        let spec = n17_live_some!(self.registry.get_for_surface(
            &invocation.facts.canonical_command,
            Some(context.context().authoring_query()),
        ), "fold-spec")?;''')
section=replace(section,'''        if spec.subcommands.is_empty() {
            spec.run_const_fold_in(arguments, dialect)
        } else {
            spec.subcommand(n17_live_some!(invocation.facts.subcommand.canonical_name(), "fold-subcommand-name")?)?
                .run_const_fold_in(arguments, dialect)
        }''','''        n17_live_trace!("hook-call command={} argc={}", invocation.facts.canonical_command, arguments.len());
        let answer = if spec.subcommands.is_empty() {
            spec.run_const_fold_in(arguments, dialect)
        } else {
            n17_live_some!(spec.subcommand(n17_live_some!(invocation.facts.subcommand.canonical_name(), "fold-subcommand-name")?), "fold-subcommand")?
                .run_const_fold_in(arguments, dialect)
        };
        n17_live_trace!("hook-return present={}", answer.is_some());
        answer''')
section=replace(section,'''        if !context.matches_registry(self.registry) {
            return None;''','''        if !context.matches_registry(self.registry) {
            n17_live_trace!("decline=argument-registry-context");
            return None;''')
# Remaining argument entry ?s, preserving original lazy fallback.
section=section.replace('''        let target = binding.proved_execution_target().or_else(|| {''','''        let target = n17_live_some!(binding.proved_execution_target().or_else(|| {''').replace('''                .map(|receipt| receipt.target)
        })?;''','''                .map(|receipt| receipt.target)
        }), "argument-target")?;''')
for x,g in [('binding.variable_context.invocation_dialect','argument-dialect'),('crate::registry_invocation::effective_words_for_target(tokens, target)','argument-effective-words'),('tokens.variable_access_at(source, spelling)','argument-variable-access'),('selected.next()','argument-child')]:section=section.replace(x+'?',f'n17_live_some!({x}, "{g}")?')
section=replace(section,'''                match word {
                    crate::ir::WordExpr::Literal''','''                let answer = match word {
                    crate::ir::WordExpr::Literal''')
section=replace(section,'''                    _ => None,
                }
            })
            .collect()''','''                    _ => None,
                };
                n17_live_trace!("argument-value index={index} present={}", answer.is_some());
                answer
            })
            .collect()''')
# Explicit refusal branches: insert marker, no new predicate.
section=section.replace('''                    return None;''','''                    n17_live_trace!("decline=argument-effect-or-child-uniqueness");
                    return None;''')
s=s[:a]+section+s[b:];save(rel,old,s)
# Optimiser entry/roster gates, required to distinguish a call never offered to const_subst.
rel='rust/tcl-compiler/src/optimiser/propagation.rs';old=(root/rel).read_text();s=old;at=s.index('pub fn run(');s=s[:at]+mac+s[at:]
s=replace(s,'pub fn run(ctx: &mut PassContext<\'_>, cu: &CompilationUnit) {','pub fn run(ctx: &mut PassContext<\'_>, cu: &CompilationUnit) {\n    n17_live_trace!("propagation-entry procedures={}", cu.procedures.len());')
s=replace(s,'''        let Some(proc) = cu.ir_module.procedures.get(qname) else {
            continue;''','''        let Some(proc) = cu.ir_module.procedures.get(qname) else {
            n17_live_trace!("decline=propagation-procedure-missing name={qname}");
            continue;''')
s=replace(s,'''        run_function(ctx, cu, fu, &proc.body, &namespace, &no_extra_escaping);''','''        n17_live_trace!("propagation-procedure name={qname} namespace={namespace}");
        run_function(ctx, cu, fu, &proc.body, &namespace, &no_extra_escaping);''')
# Namespace else site is exact and retains original query once.
needle='''            .and_then(crate::command_binding::SourceNamespaceKey::advisory_key)
        else {
            continue;
        };''';s=replace(s,needle,needle.replace('            continue;','            n17_live_trace!("decline=propagation-namespace name={qname}");\n            continue;'))
a=s.index('fn retained_substitution_calls(');b=s.index('\nfn retained_call_at(',a);t=s[a:b]
t=t.replace('''    // naming.compiler''','''    n17_live_trace!("substitution-inventory-entry");
    // naming.compiler''',1)
t=t.replace('''        return crate::word_subst::checked_lifted_calls(''','''        return n17_live_some!(crate::word_subst::checked_lifted_calls(''',1).replace('''        );
    }
    if !module''','''        ), "substitution-standalone-inventory");
    }
    if !module''',1)
t=t.replace('''.as_deref()?
        .matches_module''','''.as_deref().or_else(|| {
            n17_live_trace!("decline=substitution-retained-bindings");
            None
        })?
        .matches_module''',1)
t=t.replace('''    {
        return None;
    }
    let metadata''','''    {
        n17_live_trace!("decline=substitution-module-currency");
        return None;
    }
    let metadata''',1)
t=t.replace('''crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?''','''n17_live_some!(crate::registry_invocation::InvocationMetadataContext::for_module(registry, module), "substitution-metadata")?''')
t=t.replace('''    crate::word_subst::checked_original_lifted_calls_with_metadata_context(''','''    n17_live_some!(crate::word_subst::checked_original_lifted_calls_with_metadata_context(''').replace('''        metadata,
    )
}''','''        metadata,
    ), "substitution-original-inventory")
}''')
s=s[:a]+t+s[b:]
a=s.index('fn fold_retained_builtin(');b=s.index('\nfn retained_expr_call(',a);t=s[a:b];t=t.replace('''    let metadata = metadata?;''','''    n17_live_trace!("builtin-entry span={:?}", call.span);
    let metadata = n17_live_some!(metadata, "builtin-metadata")?;''',1);s=s[:a]+t+s[b:]
a=s.index('fn visit_call_cmd_subst_folds(');b=s.index('\n/// Fold a pure-proc',a);t=s[a:b];t=t.replace('''    let Some(registry) = ctx.registry else {''','''    n17_live_trace!("substitution-visit span={statement_span:?}");
    let Some(registry) = ctx.registry else {''',1);t=t.replace('''    for word in tokens.words() {''','''    n17_live_trace!("substitution-count count={}", calls.len());
    for word in tokens.words() {''',1);s=s[:a]+t+s[b:];save(rel,old,s)
# Registry dispatch marks actual no-host/cache/host boundary, without reevaluation.
rel='rust/tcl-registry/src/pack_hooks.rs';old=(root/rel).read_text();s=old;at=s.index('/// Invoke a pack hook after');s=s[:at]+mac.split('macro_rules! n17_live_some')[0]+s[at:]
a=s.index('pub fn dispatch(');b=s.index('\n/// Intern a hook-produced',a);t=s[a:b];t=replace(t,'''    if !ANY_HOST.load(Ordering::Relaxed) && INSTALLER.get().is_none() {
        return''','''    n17_live_trace!("dispatch-entry slot={slot:?}");
    if !ANY_HOST.load(Ordering::Relaxed) && INSTALLER.get().is_none() {
        n17_live_trace!("decline=dispatch-no-host slot={slot:?}");
        return''');t=t.replace('''    let Some(host) = ensure_host() else {
        return''','''    let Some(host) = ensure_host() else {
        n17_live_trace!("decline=dispatch-no-receiver slot={slot:?}");
        return''',1);t=t.replace('''        return hit;''','''        n17_live_trace!("dispatch-cache-hit slot={slot:?} answer={hit:?}");
        return hit;''',1);t=t.replace('''    let answer = host.invoke(slot, call);''','''    n17_live_trace!("dispatch-host-call slot={slot:?}");
    let answer = host.invoke(slot, call);
    n17_live_trace!("dispatch-host-return slot={slot:?} answer={answer:?}");''',1);s=s[:a]+t+s[b:];save(rel,old,s)
# Host selection and actual outcome. No new log between narrowing and actual invocation.
rel='rust/tcl-spec-hooks/src/host.rs';old=(root/rel).read_text();s=old;at=s.index('impl<E: Engine> HookHost<E> {');s=s[:at]+mac+s[at:]
a=s.index('    fn run(\n');b=s.index('\n/// The host\'s budget',a);t=s[a:b];t=t.replace('''        if pack.poisoned {
            return None;''','''        if pack.poisoned {
            n17_live_trace!("decline=host-poisoned");
            return None;''',1);t=t.replace('''        if pack.hooks[hook_index].quarantined {
            return None;''','''        if pack.hooks[hook_index].quarantined {
            n17_live_trace!("decline=host-quarantined");
            return None;''',1);t=t.replace('''        if program.family.requires_all_literal() && !call.all_literal() {
            return None;''','''        if program.family.requires_all_literal() && !call.all_literal() {
            n17_live_trace!("decline=host-nonliteral");
            return None;''',1);t=t.replace('''Some(self.pinned_engine(pack, call.dialect?))''','''Some(self.pinned_engine(pack, n17_live_some!(call.dialect, "host-release")?))''',1)
t=t.replace('''(engine.as_mut()?, handles.get(hook_index)?.as_ref()?)''','''(n17_live_some!(engine.as_mut(), "host-pinned-engine")?, n17_live_some!(n17_live_some!(handles.get(hook_index), "host-pinned-index")?.as_ref(), "host-pinned-handle")?)''',1)
# Existing budget error already logs; add marker only after failed setup and before original return.
t=t.replace('''            reading.clear();
            return None;
        }
        let invoked''','''            reading.clear();
            n17_live_trace!("decline=host-budget-setup");
            return None;
        }
        let invoked''',1)
t=t.replace('''        reading.clear();
        Some(match invoked {''','''        reading.clear();
        n17_live_trace!("host-engine-return outcome={}", match &invoked { Ok(Ok(_)) => "ok", Ok(Err(_)) => "error", Err(_) => "panic" });
        Some(match invoked {''',1);s=s[:a]+t+s[b:]
a=s.index('    fn invoke(&self, slot: HookSlot');b=s.index('\nenum Outcome',a);t=s[a:b];t=t.replace('''        let Some(&(pack_index, hook_index))''','''        n17_live_trace!("host-entry slot={slot:?}");
        let Some(&(pack_index, hook_index))''',1);t=t.replace('''            return HookAnswer::Abstain;''','''            n17_live_trace!("decline=host-slot-missing slot={slot:?}");
            return HookAnswer::Abstain;''',1);t=t.replace('''            Outcome::Errored(seen, message) => {''','''            Outcome::Errored(seen, message) => {
                n17_live_trace!("host-error message={message}");''',1);t=t.replace('''            Outcome::Crash(kind, detail) => {''','''            Outcome::Crash(kind, detail) => {
                n17_live_trace!("host-crash kind={kind:?} detail={detail}");''',1);s=s[:a]+t+s[b:];save(rel,old,s)
# Pinned formatter acts only on copied proposal.
fmt='/workspace/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/bin/rustfmt'
patch=[]
for rec in changes:
 rel=rec['path'];p=out/'work'/rel
 subprocess.run([fmt,'--edition','2024','--config','skip_children=true',str(p)],check=True)
 rec['proposed_sha256']=hashlib.sha256(p.read_bytes()).hexdigest()
 patch.extend(difflib.unified_diff((out/'base'/rel).read_text().splitlines(True),p.read_text().splitlines(True),fromfile='a/'+rel,tofile='b/'+rel))
(out/'proposal.patch').write_text(''.join(patch))
subprocess.run(['git','apply','--check',str(out/'proposal.patch')],cwd=root,check=True)
receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'OFF-TREE debug/env gated trace only; untyped/unrun and requires peer review','env':{'TCL_LSP_TRACE_N17_FOLD':'first-gate-v1'},'patch_sha256':hashlib.sha256((out/'proposal.patch').read_bytes()).hexdigest(),'files':changes,'scope':'Actual propagation entry/inventory, const_subst first gates/argument results, registry dispatch and actual host invocation/outcome. No query reevaluation, authority, result/assertion change. All logging/env access cfgdebug_assertions only. Existing narrowing/invoke/cost/restore/reading-clear order unchanged; host engine outcome observed only after original restoration/clear.','limits':'Temporary debug instrumentation can affect elapsed timing when enabled; not a latency/performance measurement. No release logging/env/trackcaller attributes. Ordinary original budget guards stay unchanged. Original hook positive must be run with exact env; caller census traces alone do not locate livehook.'}
(out/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
