from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib,json,os,re,subprocess,sys,time
p=Path('/workspace/.proofs');kind=sys.argv[1];assert kind in {'vm','runtime'}
a_path=p/f'{kind}-focused-consumer-admission352.json';a=json.loads(a_path.read_text());own=a['owned_image_record'];image=Path(own['executable']);sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(image)==own['sha256'];assert a['count']==len(a['selectors'])
out=p/f'{kind}-focused-isolated352';assert not out.exists();out.mkdir()
env={**os.environ,**json.loads((p/'native-test-env.json').read_text())};cwd='/workspace/.checkpoints/naming-current-validation352'
def run(row):
 ordinal,selector=row;args=[str(image),'--exact',selector,'--test-threads=1','--nocapture'];log_path=out/f'{ordinal:03}.log';started=time.monotonic();timed_out=False
 with log_path.open('wb') as log:
  try:result=subprocess.run(args,cwd=cwd,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=300);exit_code=result.returncode
  except subprocess.TimeoutExpired:exit_code=None;timed_out=True
 text=log_path.read_text(errors='replace');summaries=re.findall(r'(?m)^test result: (?:ok|FAILED)\.[^\n]*',text)
 passed=exit_code==0 and len(summaries)==1 and re.search(r'^test result: ok\. 1 passed; 0 failed; 0 ignored;',summaries[0]) is not None
 record={'ordinal':ordinal,'selector':selector,'argv':args,'cwd':cwd,'exit':exit_code,'timeout':timed_out,'timeout_seconds':300,'seconds':time.monotonic()-started,'passed_exact_assertion':passed,'summaries':summaries,'log':str(log_path),'log_sha256':sha(log_path)}
 (out/f'{ordinal:03}.json').write_text(json.dumps(record,indent=2)+'\n')
 if not passed:print(json.dumps({'ordinal':ordinal,'selector':selector,'exit':exit_code,'timeout':timed_out,'summaries':summaries}),flush=True)
 return record
started=time.monotonic()
with ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(run,enumerate(a['selectors'])))
assert sha(image)==own['sha256']
record={'status':f'Closed actual {kind}352 exact isolated assertions','admission':str(a_path),'admission_sha256':sha(a_path),'owned_image':str(image),'owned_image_sha256':own['sha256'],'selectors':len(rows),'passed':sum(r['passed_exact_assertion'] for r in rows),'failed_aborted_or_timed_out':sum(not r['passed_exact_assertion'] for r in rows),'seconds':time.monotonic()-started,'rows':rows,'limits':'Own source352 and actual selected configuration only. Each selector runs in its own process with exact anchored assertion summary required. Per-test 300s timeout is reported distinctly from assertion failure or abort. Outer run352 provides whole frozen-source before/after bookends. Three no-tommath-only controls remain outside this engine/have_tommath image.'}
(out/'results.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:v for k,v in record.items() if k not in {'rows','limits'}}),flush=True);sys.exit(0 if record['failed_aborted_or_timed_out']==0 else 1)
