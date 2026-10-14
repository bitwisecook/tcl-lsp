from pathlib import Path
out=Path('/tmp/name-variable-protection-owner-20261014/proposal')
def edit(path, old, new, count=1):
 p=out/path;s=p.read_text();assert s.count(old)==count,(path,s.count(old),old[:80]);p.write_text(s.replace(old,new))
prefix='crate::variable_observation_names::'
helper=prefix+'variable_name_may_be_observed'
module=prefix+'VariableProtectionPurpose::ModuleObservation'
mixed=prefix+'VariableProtectionPurpose::MixedSourceProtection'
native=prefix+'VariableProtectionPurpose::NativeLoweringLedger'
edit('rust/tcl-compiler/src/lib.rs','pub mod variable_bindings;','pub mod variable_bindings;\npub mod variable_observation_names;')
edit('rust/tcl-compiler/src/native_lowering/elide.rs','''let traced = self.traced.contains(base)
            || self
                .traced
                .contains(base.strip_prefix("::").unwrap_or(base))
            || self.traced.contains(&format!("::{base}"))''',f'''let traced = {helper}(self.traced, base, false, {native})''')
edit('rust/tcl-compiler/src/optimiser/manager.rs','traced.contains(var.trim_start_matches("::"))',f'{helper}(traced, var, false, {module})')
edit('rust/tcl-compiler/src/optimiser/elimination.rs','t.contains(base.trim_start_matches("::"))',f'{helper}(t, base, false, {module})')
edit('rust/tcl-compiler/src/optimiser/elimination.rs','names.contains(base.trim_start_matches("::"))',f'{helper}(names, base, false, {module})')
edit('rust/tcl-compiler/src/optimiser/chain_fold.rs','''!protected.contains(write_var(&other))
                    && !protected.contains(write_var(&other).trim_start_matches("::"))''',f'''!{helper}(protected, write_var(&other), false, {mixed})''')
edit('rust/tcl-compiler/src/optimiser/chain_fold.rs','''protected.contains(&var)
        || protected.contains(var.trim_start_matches("::"))''',f'''{helper}(protected, &var, false, {mixed})''')
edit('rust/tcl-compiler/src/analyser/bounds_checks.rs','.map(|name| name.trim_start_matches("::").to_owned())','.map(|name| crate::variable_observation_names::module_observation_key(name).to_owned())')
edit('rust/tcl-compiler/src/analyser/bounds_checks.rs','self.any || self.names.contains(var.trim_start_matches("::"))',f'{helper}(&self.names, var, self.any, {module})')
edit('rust/tcl-compiler/src/cfg_builder/global_write_info.rs','''    if let Some(relative) = name.strip_prefix("::")
        && !relative.is_empty()
    {
        names.insert(relative.to_owned());
    }
    names.insert(name.to_owned());''','''    crate::variable_observation_names::record_variable_write_name(
        names,
        name,
        crate::variable_observation_names::VariableWriteObservationPurpose::OuterScope,
    );''')
edit('rust/tcl-compiler/src/deferred_writes.rs','''        let name = name.strip_prefix("::").unwrap_or(name);
        if name.is_empty() {
            return;
        }
        out.names.insert(name.to_owned());
        let base = crate::sccp::place_base(name);
        if base != name {
            out.names.insert(base.to_owned());
        }''','''        crate::variable_observation_names::record_variable_write_name(
            &mut out.names,
            name,
            crate::variable_observation_names::VariableWriteObservationPurpose::DeferredGlobal,
        );''')
edit('rust/tcl-compiler/src/lowering/mod.rs','''        let canonical = crate::naming::normalise_var_name(&format!("${target}"))
            .trim_start_matches("::")
            .to_owned();
        if !canonical.is_empty() {
            module.traced_variables.insert(canonical);
        }''','''        let name = crate::variable_observation_names::module_observation_key(target);
        module.traced_variables.insert(name.to_owned());''')
edit('rust/tcl-compiler/src/cfg_builder/mod.rs','normalise_var_name(name).trim_start_matches("::").to_owned()','crate::variable_observation_names::module_observation_key(normalise_var_name(name)).to_owned()')
edit('rust/tcl-compiler/src/sccp.rs','''    name.split_once('(')
        .filter(|_| name.ends_with(')'))
        .map_or(name, |(base, _)| base)''','''    tcl_syntax::naming::split_element_ref(name).map_or(name, |(base, _)| base)''')
print('updated 11 consumers')
