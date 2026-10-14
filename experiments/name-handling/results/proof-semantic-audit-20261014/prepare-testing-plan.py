from pathlib import Path
from collections import defaultdict,Counter
import json,tomllib
root=Path('/workspace/tcl-lsp');out=Path('/tmp/name-proof-semantic-audit-20261014');work=json.loads((out/'currency-review-worklist.json').read_text());groups={};unmapped=[];total=0
for row in work:
 for test in row['tests']:
  total+=1;path=Path(test['path']);parts=path.parts
  if path.suffix=='.py':key=('python',str(path));meta={'kind':'python','script':str(path)}
  elif parts[:2]==('runtime','rust'):crate=Path('runtime/rust');source=Path(*parts[2:]);meta=None
  elif len(parts)>2 and parts[0]=='rust':crate=Path(*parts[:2]);source=Path(*parts[2:]);meta=None
  else:unmapped.append({'proof':row['id'],**test,'reason':'No supported crate/script ingress'});continue
  if meta is None:
   cargo=root/crate/'Cargo.toml'
   if not cargo.is_file():unmapped.append({'proof':row['id'],**test,'reason':'Cargo package manifest missing'});continue
   package=tomllib.loads(cargo.read_text());name=package['package']['name']
   if source.parts[0]=='src':target='lib';kind='lib'
   elif source.parts[0]=='tests':
    kind='test';target=source.parts[1].removesuffix('.rs')
    explicit=[x for x in package.get('test',[]) if x.get('path')==str(source)]
    if explicit:target=explicit[0]['name']
    elif len(source.parts)>2 and not any((root/crate/p).is_file() for p in [f'tests/{target}.rs',f'tests/{target}/main.rs',f'tests/{target}/mod.rs']):
     unmapped.append({'proof':row['id'],**test,'reason':'Nested source needs actual Cargo target/module routing verification'});continue
   else:unmapped.append({'proof':row['id'],**test,'reason':'Not a supported lib/test ingress'});continue
   key=(str(crate),kind,target);meta={'kind':kind,'package':name,'manifest_path':str(crate/'Cargo.toml'),'target':target,'build_scope':'Root serial build lease; dedicated worktree target, exact compiled dependency closure.'}
  group=groups.setdefault(key,{'metadata':meta,'selectors':{},'questions':set()});group['questions'].add(row['id']);bound=group['selectors'].setdefault(test['selector'],{'selector':test['selector'],'source_paths':set(),'questions':set()});bound['source_paths'].add(test['path']);bound['questions'].add(row['id'])
result=[]
for key,g in sorted(groups.items()):
 meta=g['metadata'];rows=[{'selector':name,'source_paths':sorted(x['source_paths']),'questions':sorted(x['questions'])} for name,x in sorted(g['selectors'].items())];r={**meta,'questions':sorted(g['questions']),'selectors':rows,'execution_state':'Unexecuted plan; require exact runtime --list match before execution.'}
 if meta['kind']!='python':
  args=['cargo','test','--offline','-j1','--manifest-path',meta['manifest_path'],'-p',meta['package'],'--no-run','--message-format=json'];args+=['--lib'] if meta['kind']=='lib' else ['--test',meta['target']];r['compile_arguments']=args
 result.append(r)
report={'schema_version':1,'scope':'One compile per stable actual Cargo target, exact original selector deduplication; no run or acceptance follows from this planning artifact. Source/compiled/provider closure and oracle admission remain independent.','original_binding_references':total,'unique_selectors':sum(len(x['selectors']) for x in result),'groups':result,'unmapped_bindings':unmapped}
(out/'current-testing-target-plan.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'original_references':total,'unique_selectors':report['unique_selectors'],'target_groups':len(result),'unmapped':len(unmapped),'targets':[{'package':r.get('package'),'target':r.get('target',r.get('script')),'selectors':len(r['selectors'])} for r in result]},indent=2))
