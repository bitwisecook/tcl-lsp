from pathlib import Path
import collections,datetime,hashlib,json,re
ansi=re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')
rx=re.compile(r'^\s*(PASS|FAIL|TIMEOUT|LEAK|SKIP|SIGABRT|SEGFAULT)\s+\[([^\]]+)\]\s+\(\s*(\d+)/(\d+)\)\s+(\S+)\s+(\S+)\s*$')
def parse(path):
 raw=Path(path).read_bytes();lines=ansi.sub('',raw.decode('utf-8-sig')).splitlines();ordinals={};duplicates=[];conflicts=[];unparsed=[];total=None
 for no,line in enumerate(lines,1):
  text=re.sub(r'^\d{4}-\d\d-\d\dT\S+Z\s?','',line)
  start=re.search(r'^\s*Starting (\d+) tests across (\d+) binaries\s*$',text)
  if start:total=int(start[1])
  m=rx.fullmatch(text)
  if m:
   state,duration,ordinal,denominator,binary,name=m.groups();ordinal=int(ordinal);denominator=int(denominator)
   row={'ordinal':ordinal,'denominator':denominator,'binary_id':binary,'test_name':name,'status':state,'duration':duration.strip(),'line':no,'line_text':text}
   if ordinal in ordinals:
    prior=ordinals[ordinal]
    if any(prior[k]!=row[k] for k in ['denominator','binary_id','test_name','status']):conflicts.append({'previous':prior,'next':row})
    else:duplicates.append({'ordinal':ordinal,'first_line':prior['line'],'repeated_line':no})
   else:ordinals[ordinal]=row
  elif re.search(r'\(\s*\d+/\d+\)',text) and re.match(r'^\s*(?:PASS|FAIL|TIMEOUT|LEAK|SKIP|SIGABRT|SEGFAULT|EXEC|TRY)',text):unparsed.append({'line':no,'text':text})
 rows=sorted(ordinals.values(),key=lambda x:x['ordinal']);bykey={};keyconflicts=[]
 for row in rows:
  key=(row['binary_id'],row['test_name'])
  if key in bykey:keyconflicts.append({'key':key,'previous':bykey[key],'next':row})
  else:bykey[key]=row
 summary=[{'line':i+1,'text':re.sub(r'^\S+Z\s?','',line)} for i,line in enumerate(lines) if re.search(r'^\S+Z\s*Summary \[',line)]
 info={'path':str(path),'sha256':hashlib.sha256(raw).hexdigest(),'declared_start_total':total,'unique_ordinal_rows':len(rows),'status_counts':dict(collections.Counter(x['status'] for x in rows)),'duplicate_result_rows_excluded':len(duplicates),'duplicates':duplicates,'conflicting_ordinals':conflicts,'duplicate_binary_name_keys':keyconflicts,'unparsed_result_like_rows':unparsed,'missing_ordinals':sorted(set(range(1,(total or 0)+1))-ordinals.keys()),'denominators':sorted(set(x['denominator'] for x in rows)),'summary_context_only':summary,'rows':rows}
 assert not conflicts and not keyconflicts and not unparsed and not info['missing_ordinals'],info
 assert total==len(rows) and info['denominators']==[total]
 return info,bykey,lines
current,now,lines=parse('/tmp/work-ci5205/heavy.log');baseline,old,_=parse('/tmp/work-ci5205/baseline-heavy.log')
joined=[]
for key,row in now.items():
 previous=old.get(key)
 state='absent-baseline' if previous is None else ('matched-PASS-to-FAIL' if previous['status']=='PASS' and row['status']=='FAIL' else 'matched-'+previous['status']+'-to-'+row['status'])
 joined.append({'binary_id':key[0],'test_name':key[1],'classification':state,'current':row,'baseline':previous})
failures=[r for r in joined if r['current']['status']=='FAIL']
receipt={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'Independent heavy CI job only, not part of main24312 baseline ledger','method':'Authoritative nextest ordinal completion rows once per ordinal; exact binary_id+test_name join. Repeated failure-display rows excluded, conflicting ordinals/duplicate keys/unparsed completion-like rows are terminal errors. No package/name-only or job-aggregate fallback.','current_job':'CI5205 heavy (root-supplied raw)','baseline_job':'actual rust heavy119956195910 (root-supplied raw)','current':current,'baseline':baseline,'all_current_join_counts':dict(collections.Counter(x['classification'] for x in joined)),'current_failure_join_counts':dict(collections.Counter(x['classification'] for x in failures)),'current_failures':failures,'absent_current':[r for r in baseline['rows'] if (r['binary_id'],r['test_name']) not in now],'joined':joined,'limits':'Matched result is a regression observation, not cause. Failure stderr/source path attribution follows separately; no builds or source/assertion changes.'}
p=Path('/tmp/ccg-heavy-ci-comparison/comparison.json');p.write_text(json.dumps(receipt,indent=2)+'\n')
print('CURRENT',current['unique_ordinal_rows'],current['status_counts'],'duplicates',current['duplicate_result_rows_excluded']);print('BASELINE',baseline['unique_ordinal_rows'],baseline['status_counts'],'duplicates',baseline['duplicate_result_rows_excluded']);print('JOIN',receipt['all_current_join_counts']);print('FAIL_JOIN',receipt['current_failure_join_counts']);print('BINARIES',dict(collections.Counter(x['binary_id'] for x in failures)));print('SHA256',hashlib.sha256(p.read_bytes()).hexdigest())
for r in failures:print(r['classification'],r['binary_id'],r['test_name'])
