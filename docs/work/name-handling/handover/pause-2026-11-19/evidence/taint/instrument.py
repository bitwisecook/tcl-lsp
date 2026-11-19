import pathlib,sys,shutil
src=pathlib.Path(sys.argv[1])/'rust/tcl-compiler/src'
mod=src/'lib.rs';mod.write_text(mod.read_text()+'\nmod burn_down_observe;\n')
shutil.copyfile('/tmp/bd-taint-ab/observe.rs',src/'burn_down_observe.rs')
p=src/'compilation_unit/function_build.rs';s=p.read_text()
marker='            sccp: derived.sccp,'
assert marker in s
i=s.index('        Self {',s.index('pub(super) fn build_full('))
s=s[:i]+'        crate::burn_down_observe::strict(&cfg, &ssa, &derived.sccp);\n'+s[i:]
i=s.index('        Self {',s.index('pub fn trivial_guarded('))
s=s[:i]+'        crate::burn_down_observe::strict(&cfg, &ssa, &SccpResult::default());\n'+s[i:];p.write_text(s)
p=src/'sccp/solver.rs';s=p.read_text().replace("pub(super) fn solve(inputs: &SolveInputs<'_>, round: &Round, hazards: &RunHazards) -> Solved {","pub(super) fn solve(inputs: &SolveInputs<'_>, round: &Round, hazards: &RunHazards) -> Solved {\n    crate::burn_down_observe::solve(\"solve-execution\",inputs.cfg);");p.write_text(s)
p=src/'sccp/original_value_facts/value_solver.rs';s=p.read_text();i=s.index('    let ValueFactInputs {',s.index('fn solve_value_facts_once'));s=s[:i]+'    crate::burn_down_observe::solve(match purpose { SolverPurpose::ExecutionErasure=>\"solve-original-execution\", SolverPurpose::SemanticAnalysis=>\"solve-semantic\" },cfg);\n'+s[i:];p.write_text(s)
