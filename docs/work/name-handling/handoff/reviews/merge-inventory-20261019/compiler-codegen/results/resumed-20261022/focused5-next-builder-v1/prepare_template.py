from pathlib import Path
import ast,json,hashlib
B=Path('/tmp/ccg-focused5-next-builder-v1')
s=Path('/tmp/ccg-post46-direct3-v6/suite.py').read_text()
a=s.index('def source_view(');b=s.index('def source_manifest(',a);s=s[:a]+(B/'source_view.py.fragment').read_text()+s[b:]
s=s.replace("Path('/tmp/ccg-post46-direct3-v6')","Path(__file__).resolve().parent")
s=s.replace("Path('/tmp/ccg-post46-direct3-v6-runs')","Path(str(BASE)+'-runs')")
s=s.replace("if plan['expected_filters'] != 3 or plan['expected_selected'] != 3:","if plan['expected_filters'] != 5 or plan['expected_selected'] != 5:")
s=s.replace('exact native1 + N17two inventory is 3/3','exact original direct3 + hook2 inventory is 5/5')
s=s.replace("source_after_startup = source_manifest()\nif source_after_startup != before:\n    raise SystemExit('STOP: source changed while creating startup evidence')", "# Full source/provider equality is measured once before and once after the group.")
a=s.index('    if source_manifest() != before:',s.index('for target in'));b=s.index('    check_legacy_pins()',a);s=s[:a]+"    # Full group before/after source/provider seals; artifact and pins remain per-command.\n"+s[b:]
a=s.index('    after = source_manifest()',s.index('for index, row in'));b=s.index('    artifact_stable',a);s=s[:a]+"    source_stable = None\n    providers_stable = None\n"+s[b:]
s=s.replace("if not source_stable or not providers_stable or not artifact_stable:\n        raise SystemExit('STOP: source/provider/artifact drift')","if not artifact_stable:\n        raise SystemExit('STOP: per-row artifact drift')")
s=s.replace("'source_stable': source_stable, 'providers_stable': providers_stable,","'source_stable': source_stable, 'providers_stable': providers_stable,\n                  'source_provider_scope': 'group before/after; pending until final.json',")
s=s.replace('    source_stable = before == after\n','')
s=s.replace("if len(results) != 3 or sum(x['actual_selected'] for x in results) != 3:","if len(results) != 5 or sum(x['actual_selected'] for x in results) != 5:")
s=s.replace('terminal diagnostic 3/3 inventory mismatch','terminal original focused5 5/5 inventory mismatch').replace("'all3_direct_child_timers_valid'","'all5_direct_child_timers_valid'")
# JSON control and Cargo event objects fail closed on duplicate keys.
s=s.replace('plan = json.loads(plan_bytes)','plan = strict_json(plan_bytes)')
s=s.replace('json.loads(', 'strict_json(')
# Initial plan parse occurs before helper definitions: prepend strict decoder.
helper='''def strict_json(raw):
    def unique(pairs):
        result={}
        for key,value in pairs:
            if key in result:raise ValueError('duplicate JSON key '+key)
            result[key]=value
        return result
    return json.loads(raw,object_pairs_hook=unique)

'''
s=s.replace("ROOT = Path(",helper+"ROOT = Path(",1)
ast.parse(s);(B/'suite.py.template.txt').write_text(s)
# Codegen guarded Cargo adapter is byte-for-byte same behavior, only source BASE routing changes.
c=Path('/tmp/ccg-post46-direct3-v6/guarded-cargo.py').read_text().replace("BASE=Path('/tmp/ccg-post46-direct3-v6')","BASE=Path(__file__).resolve().parent")
(B/'guarded-cargo.py.template.txt').write_text(c);ast.parse(c)
print('template AST passes',hashlib.sha256(s.encode()).hexdigest())
