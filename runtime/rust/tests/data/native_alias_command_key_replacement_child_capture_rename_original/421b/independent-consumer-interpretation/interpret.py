from pathlib import Path
import json,hashlib
root=Path('/workspace/.proofs/consumer-native-alias-key-request421b');out=Path(__file__).resolve().parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
closure=json.loads((root/'closure.json').read_text());request=json.loads((root/'request.json').read_text())
assert closure['request_sha256']==sha(root/'request.json') and closure['required_inputs_unchanged'] and closure['all_capture_processes_closed']
assert len(closure['records'])==19
for item in closure['records']:
 assert sha(item['path'])==item['sha256'];r=json.loads(Path(item['path']).read_text());assert r['status']=='closed' and r['returncode']==0 and not r['timeout']
providers=[]
for provider in request['providers']:
 d=root/'providers'/provider;r=json.loads((d/'execute.receipt.json').read_text())
 assert r['source_sha256']==request['probe_sha256'] and r['request_sha256']==sha(root/'request.json')
 assert r['stdout_sha256']==sha(d/'execute.stdout') and r['stderr_sha256']==sha(d/'execute.stderr')
 assert r['executable_unchanged'] and r['closed_cases']==5 and r['closed_marker']
 lines=(d/'execute.stdout').read_bytes().split(b'\n')
 assert [int(x.split(b'|')[1]) for x in lines if x.startswith(b'CLOSED_CASE|')]==[1,7,8,9,10]
 assert sum(x==b'CLOSED_ORIGINAL_ALIAS_OBJECTS' for x in lines)==1
 parsed=[]
 for number,line in enumerate(lines,1):
  if not line:continue
  words=line.decode('ascii').split('|');fields=dict(x.split('=',1) for x in words if '=' in x)
  parsed.append({'line':number,'kind':words[0],'components':words[1:],'fields':fields})
 cases=[]
 for case in [1,7,8,9,10]:
  rows=[x for x in parsed if x['kind'] in ['ROW','EVENT','LIST','READ','CALL','RELEASE'] and x['components'][0]==str(case)]
  def stage(label):return next(x for x in rows if x['kind']=='ROW' and x['components'][1]==label)
  def roles(row):return [int(row['fields'][f'r{k}_refs']) for k in range(5)]
  name_stages=[('after-capture' if case==1 else 'first-name-after-result-clear')]
  if case in [7,9]:name_stages+=['replacement-name-after-result-clear']
  if case==10:name_stages+=['rename-name-after-result-clear']
  calls=[x for x in rows if x['kind']=='CALL'];assert len(calls)==2
  for c in calls:
   assert c['fields']=={'argc':'4','head_original':'1','p1_original':'1','p2_original':'1'}
  reads=[x['fields']['arg'] for x in rows if x['kind']=='READ'];assert reads==['2','1','2','1']
  errors=[x for x in rows if x['kind']=='ROW' and x['fields']['code']!='0']
  if provider=='jim' and case in [8,9]:
   assert {x['components'][1] for x in errors}==({'current-query-returned'} if case==8 else {'first-query-returned','current-query-returned'})
   assert all(x['fields']['code']=='1' for x in errors)
  else:assert not errors
  updates=[x for x in rows if x['kind']=='EVENT' and x['components'][2]=='update']
  delete=stage('alias-deleted')
  lists=[x for x in rows if x['kind']=='LIST']
  for lst in lists:
   assert lst['fields']['count']=='3' and all(lst['fields'][f'm{k}_original']=='1' for k in range(3))
  if case==1:
   q=[x for x in lists if x['components'][1]=='query-two-raw'];assert len(q)==1
   assert q[0]['fields']['same_other']==('1' if provider=='jim' else '0')
  if case in [7,9] and not(provider=='jim' and case==9):
   q=[x for x in lists if x['components'][1]=='current-query-raw'];assert len(q)==1 and q[0]['fields']['same_other']=='0'
  cases.append({'case':case,'name_stages':{x:roles(stage(x)) for x in name_stages},'first_update_roles_and_references':[[int(x['fields']['role']),int(x['fields']['refs'])] for x in updates], 'calls':len(calls),'read_arguments':reads,'Guest_nonzero_rows':[{'line':x['line'],'stage':x['components'][1],'code':x['fields']['code']} for x in errors], 'alias_deleted_free_rep':[int(delete['fields'][f'r{k}_free_rep']) for k in range(5)],'lists':[{'line':x['line'],'stage':x['components'][1],'fields':x['fields']} for x in lists],'raw_observer_rows':rows})
 providers.append({'provider':provider,'execute_stdout':str(d/'execute.stdout'),'execute_stdout_sha256':sha(d/'execute.stdout'),'execute_receipt':str(d/'execute.receipt.json'),'execute_receipt_sha256':sha(d/'execute.receipt.json'),'actual_version_rows':r['actual_version_rows'],'cases':cases,'API_and_unavailable':[x for x in parsed if x['kind'] in ['API','UNAVAILABLE']]})
observations={
 'C_core_replacement':'After clearing the replacement result, every C keeps old role0 at caller-only1 and new role4 at2 (caller+new alias record). Jim keeps old role0 at2 (caller+occupied table key) and new role4 at caller-only1.',
 'C_core_rename':'After rename and result clear C keeps original alias record role0 at2, destination role4 at caller-only1; Jim keeps old role0 at1 and actual new destination key role4 at2.',
 'child_capture':'All five C child-handle entries first update target role1 at1 then name role0 at2; Jim first updates only name role0 at2, with target/prefix Strings unread. After result clear C name has2; Jim original caller name has1.',
 'child_replacement':'C old caller name1/new original alias record name2 after result clear. Jim both original caller names1: child Jim_RegisterCmd creates an independent String key and stores the parent prefix List.',
 'query':'C public queries return fresh List headers retaining all three original members; separate queries/replacement queries have different headers. Jim core queries return the stored List; replacing the alias creates a new stored List while an old query retains its old List. Jim child info alias returns actual Guest1, command is not an alias; flags0 child has no public original List query.',
 'invocation':'Every case reaches two actual callbacks with original target/prefix identity, arg2 getter before arg1, counted NUL-bearing arg1 preserved. Later invocation uses reached caches. Original target changes type/free_rep independently of header lifetime on actual command lookup.',
 'retirement':'Held query Lists retain original members after alias deletion; FreeRep/query release rows remain actual observations. Rows with r*_held0 intentionally do not dereference the released role pointer; -1 is no longer caller-held, not a zero-reference assertion.'}
limits=['Finite original-object observations for these exact cases/providers/API channels; no overall alias equivalence, frame, generation, Host, compiler or application Native grant.','Jim public EvalObjVector body name-getter reference counts include independent public invocation roles; do not compare these to a Rust handler-only count.','C fresh query header contract is independent of TclOO same-stored-header behavior.','Launcher closure is process/source/executable admission only; actual Guest query errors remain explicit and no Rust test outcome is implied.']
result={'question_id':request['question_id'],'status':'Read-only interpretation of actual Root-owned original capture; no Rust assertion outcome.','outer_receipt':'/workspace/.proofs/native-alias-key421b-outer.json','outer_receipt_sha256':sha('/workspace/.proofs/native-alias-key421b-outer.json'),'closure':str(root/'closure.json'),'closure_sha256':sha(root/'closure.json'),'request':str(root/'request.json'),'request_sha256':sha(root/'request.json'),'probe':str(root/'probe.c'),'probe_sha256':sha(root/'probe.c'),'providers':providers,'observations':observations,'limits':limits,'source_windows':[{'path':str(p),'sha256':sha(p)} for p in sorted((root/'source-windows').rglob('*.txt'))]}
(out/'interpretation.json').write_text(json.dumps(result,indent=2)+'\n')
(out/'README.md').write_text('# Original alias command-key observations\n\n'+ '\n\n'.join(observations.values())+'\n\n'+ '\n\n'.join(limits)+'\n')
manifest={'packet':'ConsumerNativeAliasKeyInterpretation421b','status':result['status'],'artifacts':[{'path':str(p),'sha256':sha(p)} for p in [out/'interpretation.json',out/'README.md',out/'interpret.py']]}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'manifest_sha256':sha(out/'manifest.json'),'interpretation_sha256':sha(out/'interpretation.json'),'providers':len(providers),'cases':sum(len(p['cases']) for p in providers),'observations':observations},indent=2))
