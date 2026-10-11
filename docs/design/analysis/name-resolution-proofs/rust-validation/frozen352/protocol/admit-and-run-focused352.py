from pathlib import Path
import hashlib,json,os,re,subprocess,sys,time
from concurrent.futures import ThreadPoolExecutor
p=Path('/workspace/.proofs');sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
source_perf=Path('/tmp/naming-vm-current-numeric-entry-owner-final/REQUESTS.json')
requests=json.loads(source_perf.read_text())['targets']
for kind,package in [('vm','tcl-vm'),('runtime','tcl-runtime')]:
 inputs=[];wanted=set()
 def add(path,values):
  path=Path(path);inputs.append({'path':str(path),'sha256':sha(path),'selectors':sorted(values)});wanted.update(values)
 add(source_perf,next(r['selectors'] for r in requests if r['package']==package and r['target']=='lib'))
 grammar=p/f'grammar-owner-validation-current533-final/tcl-{kind}-lib.json'
 add(grammar,[r['selector'] for r in json.loads(grammar.read_text())['selectors']])
 if kind=='vm':
  path=p/'receiver-vm-original-lookups561b/selectors.json';add(path,json.loads(path.read_text())['VM'])
 else:
  path=p/'runtime-focused-consumer-admission349.json';add(path,json.loads(path.read_text())['selectors'])
  path=p/'consumer-c84-counted-append461b/request.json';add(path,next(r['normal_selectors'] for r in json.loads(path.read_text())['targets'] if r['package']==package))
  path=p/'runtime-variable-search-formal546d1b/selectors.json';add(path,json.loads(path.read_text())['runtime'])
  path=p/'runtime-full-name-birth546g/selectors.json';d=json.loads(path.read_text());add(path,d['runtime']+d['preserved_affected_regressions'])
 own_path=p/f'pinned-{kind}352.json';own=json.loads(own_path.read_text());assert sha(Path(own['executable']))==own['sha256']
 list_path=p/f'integration-{kind}-own-list352.log';inventory={line[:-6] for line in list_path.read_text().splitlines() if line.endswith(': test')}
 absent=sorted(wanted-inventory);cfg=[]
 if kind=='runtime':cfg=json.loads((p/'runtime-focused-consumer-admission349.json').read_text())['unavailable_cfg_selectors']
 cfg_names={r['selector'] if isinstance(r,dict) else r for r in cfg}
 assert set(absent)<=cfg_names,(kind,absent,cfg_names)
 admitted=sorted(wanted&inventory)
 a={'status':'Actual352 own-list selected assertions admitted; no execution outcomes yet','owned_image_record':own,'own_image_record_sha256':sha(own_path),'own_list':str(list_path),'own_list_sha256':sha(list_path),'own_list_count':len(inventory),'request_inputs':inputs,'selectors':admitted,'count':len(admitted),'request_count':len(wanted),'unavailable_cfg_selectors':cfg,'absent':absent,'source_snapshot_sha256':own['source_snapshot_sha256'],'limits':'Actual source352 image only; excludes later533b and private successors. Earlier request selectors are joined literally to this own inventory, not credited with earlier outcomes. Genuine no-tommath missing configuration remains unexecuted.'}
 path=p/f'{kind}-focused-consumer-admission352.json';assert not path.exists();path.write_text(json.dumps(a,indent=2)+'\n')
 print(kind,len(inventory),len(admitted),absent)
