from pathlib import Path
import json,hashlib,re
P=Path('/workspace/.proofs/consumer-native-alias-objects415');O=Path('/workspace/.proofs/consumer-native-alias-interpretation415');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
req=json.loads((P/'request.json').read_text());closure=json.loads((P/'closure.json').read_text());assert closure['request_sha256']==sha(P/'request.json') and closure['all_capture_processes_closed'] and closure['required_inputs_unchanged']
for r in closure['records']:
 assert sha(r['path'])==r['sha256'];a=json.loads(Path(r['path']).read_text());assert a['status']=='closed' and a['returncode']==0 and not a['timeout']
def values(parts):
 x={}
 for p in parts:
  if '='in p:
   k,v=p.split('=',1);x[k]=int(v)if k not in ['hex','payload'] and re.fullmatch(r'-?\d+',v) else v
 return x
out={}
for provider in req['providers']:
 p=P/'providers'/provider;receipt=json.loads((p/'execute.receipt.json').read_text());assert receipt['stdout_sha256']==sha(p/'execute.stdout') and receipt['stderr_sha256']==sha(p/'execute.stderr') and receipt['executable_unchanged']
 lines=(p/'execute.stdout').read_text().splitlines();records=[];meta=[]
 for i,l in enumerate(lines,1):
  parts=l.split('|');kind=parts[0]
  if kind in ['BUILD','VERSION','API']:meta.append(dict(line=i,raw=l));continue
  if kind not in ['ROW','EVENT','LIST','CALL','READ','RELEASE','CLOSED_CASE']:continue
  record=dict(kind=kind,case=int(parts[1]),line=i,raw=l,values=values(parts[2:]));
  if kind in ['ROW','LIST']:record['stage']=parts[2]
  if kind=='EVENT':record.update(sequence=int(parts[2]),stage=parts[3])
  records.append(record)
 rows=[r for r in records if r['kind']=='ROW'];assert len(rows)==(73 if provider=='jim'else 93)
 assert all(r['values']['code']==0 for r in rows)
 def one(kind,case,stage):
  rs=[r for r in records if r['kind']==kind and r['case']==case and r.get('stage')==stage];assert len(rs)==1,(provider,kind,case,stage);return rs[0]
 observed=[]
 for c in range(1,6 if provider=='jim'else 7):
  after=one('ROW',c,'after-capture');v=after['values'];events=[r for r in records if r['kind']=='EVENT'and r['case']==c and r['line']<after['line']]
  assert [r['values']['role']for r in events]==([0]if provider=='jim'else[1,0]if provider in ['8.4.20','8.5.19']else[0,1])
  assert v['r2_update']==v['r3_update']==0 and v['r2_refs']==v['r3_refs']==2
  assert v['r1_update']==(0 if provider=='jim'else 1)
  observation=dict(case=c,capture_events=[dict(line=r['line'],role=r['values']['role'],refs=r['values']['refs'],kind=r['stage'])for r in events],after_capture_line=after['line'],prefix_updates=[v['r2_update'],v['r3_update']],prefix_refs=[v['r2_refs'],v['r3_refs']],target_updates=v['r1_update'])
  if c in [1,2,4]:
   q1=one('LIST',c,'query-one-raw');q2=one('LIST',c,'query-two-raw');a,b=q1['values'],q2['values'];assert a['string']==b['string']==0 and a['count']==b['count']==3
   assert (a['header']==b['header'])==(provider=='jim') and b['same_other']==(1 if provider=='jim'else 0)
   for k in range(3):assert a[f'm{k}_original']==b[f'm{k}_original']==1 and a[f'm{k}_header']==b[f'm{k}_header']
   assert a['m1_string']==a['m2_string']==b['m1_string']==b['m2_string']==0
   observation['raw_query']=dict(lines=[q1['line'],q2['line']],same_header=provider=='jim',original_member_headers=True,query_getters=0,first_member_refs=[a[f'm{k}_refs']for k in range(3)],second_member_refs=[b[f'm{k}_refs']for k in range(3)])
  if c==2:
   q=one('LIST',c,'query-two-after-other-presentation');a=q['values'];assert a['string']==(1 if provider=='jim'else 0)and a['m1_string']==a['m2_string']==1
   observation['other_query_after_presentation']=dict(line=q['line'],list_string=a['string'],member_strings=[a[f'm{k}_string']for k in range(3)])
  if c in [1,2,3,6]:
   calls=[r for r in records if r['kind']=='CALL'and r['case']==c];assert len(calls)==2 and all(r['values']['head_original']==r['values']['p1_original']==r['values']['p2_original']==1 for r in calls)
   reads=[r for r in records if r['kind']=='READ'and r['case']==c];assert [r['values']['arg']for r in reads]==[2,1,2,1]
   assert all(r['values']['hex']==('626574612079'if r['values']['arg']==2 else'616c70686120002078')for r in reads)
   a=one('ROW',c,'first-invocation')['values'];b=one('ROW',c,'second-invocation')['values'];assert a['r2_update']==b['r2_update']==a['r3_update']==b['r3_update']==1
   observation['invocation']=dict(call_lines=[r['line']for r in calls],original_head_and_prefix=True,read_lines=[r['line']for r in reads],read_order=[2,1,2,1],counted_opaque_nul_preserved=True,cache_updates_each_member_total=1)
  if c==4:
   a=one('ROW',c,'alias-deleted')['values'];b=one('ROW',c,'query-one-released')['values'];d=one('ROW',c,'query-two-released')['values'];assert a['r1_free_rep']==a['r2_free_rep']==a['r3_free_rep']==b['r1_free_rep']==b['r2_free_rep']==b['r3_free_rep']==0 and d['r1_free_rep']==d['r2_free_rep']==d['r3_free_rep']==1
   observation['lifetime']=dict(alias_deletion_preserves_query_owned_members=True,first_query_release_preserves_members=True,last_query_release_internal_rep_callbacks_each=1,meaning='Actual custom internalRep retirement; no inference of header destruction')
  if c==5:
   a=one('ROW',c,'alias-deleted')['values'];assert a['r1_free_rep']==a['r2_free_rep']==a['r3_free_rep']==1 and a['r2_update']==a['r3_update']==0
   observation['lifetime']=dict(no_query_owner_alias_delete_internal_rep_callbacks_each=1,unread_prefix_getters=0)
  observed.append(observation)
 out[provider]=dict(execute_receipt=dict(path=str(p/'execute.receipt.json'),sha256=sha(p/'execute.receipt.json')),stdout=dict(path=str(p/'execute.stdout'),sha256=sha(p/'execute.stdout')),metadata=meta,row_count=len(rows),case_records=observed,raw_record_locations=records)
# Capture additional original public-command pretest windows from already-pinned full source.
source=[]
for v in req['providers']:
 if v=='jim':continue
 p=Path('/workspace/tcl-lsp/tmp/tcl'+v+'/generic/tclInterp.c');s=p.read_text();lines=s.splitlines();start=next(i for i,l in enumerate(lines)if 'case OPT_ALIAS:'in l);end=next(i for i in range(start+1,len(lines))if 'case OPT_ALIASES:'in lines[i]);expected=next(b for b in req['provider_builds']if b['provider']==v)['required_sha256'][str(p)];assert sha(p)==expected
 dst=O/f'public-alias-pretest-{v}.txt';dst.write_text('\n'.join(f'{i+1}: {lines[i]}'for i in range(start,end))+'\n');source.append(dict(provider=v,path=str(p),sha256=sha(p),window=str(dst),window_sha256=sha(dst),lines=[start+1,end]))
result=dict(status='Independent interpretation of genuine Root-launched closed Native415 observations; no Rust/software assertion or hosted BigIP result',request_manifest=dict(path=str(P/'manifest.json'),sha256=sha(P/'manifest.json')),closure=dict(path=str(P/'closure.json'),sha256=sha(P/'closure.json')),additional_pinned_source_windows=source,providers=out,limited_contracts=['C84/85 target String pretest occurs before original alias capture; C86+ capture then alias-name String/publication then original-target loop lookup.','All five C raw queries allocate separate unstringified List headers containing the exact retained original members; member cache is shared, List String cache is not.','Jim core alias captures without target/prefix String; info alias returns the same stored List header and List String cache.','Invocation callbacks receive original members and select p2 then p1. Later invocation observes cached original members.','Held genuine query Lists preserve original members after original caller holds and alias ownership are released. Last query release retires unread internalRep without requesting String.','C child case6 uses original members in actual target parent invocation. Jim C-style child-query purpose is explicitly unavailable, separate from its measured core alias API.'],no_claims=['No general extension failure chronology is inferred beyond selected source plus recorded updater events.','No OO same-header contract is borrowed for C aliases.','No actual software admission/assertions or broad loaded-name/Native execution permission is inferred.'])
(O/'interpretation.json').write_text(json.dumps(result,indent=2)+'\n')
files=[dict(path=str(p),sha256=sha(p),bytes=p.stat().st_size)for p in sorted(O.iterdir())if p.is_file()and p.name!='manifest.json'];m=dict(status=result['status'],files=files,limited_contracts=result['limited_contracts'],source_input_request_sha256=sha(P/'request.json'),source_input_closure_sha256=sha(P/'closure.json'));(O/'manifest.json').write_text(json.dumps(m,indent=2)+'\n');print('interpretation',sha(O/'interpretation.json'));print('manifest',sha(O/'manifest.json'))
