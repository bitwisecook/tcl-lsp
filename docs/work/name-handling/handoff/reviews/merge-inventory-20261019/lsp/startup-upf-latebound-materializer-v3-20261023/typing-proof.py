"""Authenticate actual closed all-target compilation evidence; performs no process launch."""
from pathlib import Path
import json,hashlib,datetime,re

def digest(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def require(value,message):
 if not value:raise RuntimeError('STOP: '+message)
def unique_object(pairs):
 result={}
 for key,value in pairs:
  if key in result:raise RuntimeError('STOP: duplicate JSON key: '+key)
  result[key]=value
 return result
def read(path):return json.loads(Path(path).read_text(),object_pairs_hook=unique_object)
def validate_typing(binding):
 # Fail closed on actual incomplete/failed compilation before inspecting mutable sources.
 final_path=Path(binding['final']['path']);launcher_path=Path(binding['launcher']['path'])
 for key in ('identity','plan','final','launcher'):
  item=binding[key];require(digest(item['path'])==item['sha256'],'typed evidence pin changed: '+key)
 final=read(final_path);launch=read(launcher_path);tp=read(binding['plan']['path']);identity=read(binding['identity']['path']);base=Path(binding['plan']['path']).parent
 require(final.get('status')=='PASS' and final.get('overall_exit')==0 and final.get('cargo_exit_code')==0,'actual typecheck not completed PASS0')
 require(all(final.get(k) is True for k in ['guard_valid','warning_free_json_stderr_stdout','source_stable','providers_stable','root_go_stable','head_docs_descendant_valid']),'actual typecheck guards incomplete')
 require(final_path.resolve().is_relative_to((base/'runs').resolve()) and launcher_path.resolve().is_relative_to((base/'launcher-runs').resolve()),'typecheck receipts outside actual protocol roots')
 require(launch.get('exit_code')==0 and launch.get('finished_utc') and all(launch.get(k) is True for k in ['inputs_stable','root_go_stable']),'actual typing launcher not closed valid PASS')
 now=datetime.datetime.now(datetime.timezone.utc);start=datetime.datetime.fromisoformat(launch['started_utc']);finish=datetime.datetime.fromisoformat(final['finished_utc']);closed=datetime.datetime.fromisoformat(launch['finished_utc']);require(start<=finish<=closed<=now,'actual completion timestamps inconsistent')
 head=identity['approved_integrated_head'];tree=identity['approved_integrated_tree'];count=identity['path_count'];require(count==122,'reviewed exact source122 path count changed')
 require(tp.get('approved_product_head')==head and tp.get('approved_product_tree')==tree and final.get('approved_product_head')==head and final.get('approved_product_tree')==tree,'identity/typecheck product tuple mismatch')
 require(final.get('plan_sha256')==binding['plan']['sha256'] and launch.get('plan_sha256')==binding['plan']['sha256'],'actual typing plan mismatch')
 require(final.get('selected_packages')==tp.get('packages')==['tcl-compiler','tcl-lsp-core','tcl-lsp-db','tcl-lsp-server','tcl-vm','tcl-fuzz','tcl-spectcl','bpf-tcl-ir'],'actual eight-package inventory mismatch')
 source=Path(identity['source_manifest_path']);require(digest(source)==identity['source_manifest_sha256']==tp['integrated_manifest_sha256'],'actual identity/source map hash mismatch');manifest=read(source)
 require(manifest['approved_product_head']==head and manifest['approved_product_tree']==tree and len(manifest['paths'])==len({x['path'] for x in manifest['paths']})==122,'source map tuple/count mismatch')
 mi=base/'runner-input-manifest.json';go=base/'root-go.json';pins=dict(read(mi)['inputs']);pins[str(mi)]=digest(mi);pins[str(go)]=digest(go)
 require(launch.get('input_pins_before')==pins and launch.get('input_pins_after')==pins,'actual launcher complete before/after input inventory mismatch')
 for path,pin in pins.items():require(digest(path)==pin,'actual typecheck input changed: '+path)
 require(launch.get('manifest_sha256')==digest(mi),'typing launcher manifest mismatch')
 require(final.get('root_go_sha256')==launch.get('root_go_sha256')==launch.get('root_go_sha256_after')==digest(go),'actual typing approval mismatch')
 approval=read(go);require(approval.get('plan_sha256')==digest(binding['plan']['path']) and approval.get('runner_input_manifest_sha256')==digest(mi) and approval.get('source122_manifest_sha256')==identity['source_manifest_sha256'] and approval.get('approved_integrated_head')==head and approval.get('approved_integrated_tree')==tree,'actual root approval tuple/map/input mismatch')
 parent=final_path.parent
 for side in ('before','after'):
  require(read(parent/('root-go-'+side+'.json'))==approval,'actual approval snapshot mismatch')
 require(read(parent/'plan.json')==tp,'actual plan snapshot mismatch')
 require(read(parent/'source-before.json')==read(parent/'source-after.json') and read(parent/'providers-before.json')==read(parent/'providers-after.json'),'actual source/provider closure mismatch')
 process=read(parent/'process.json');require(digest(parent/'process.json')==final.get('process_receipt_sha256'),'actual process pin mismatch')
 require(process.get('exit_code')==0 and process.get('stop_reason') is None and all(process.get(k) is True for k in ['complete','timer_armed_before_spawn','group_empty_before_parent_reap','artifact_unchanged']) and process.get('consecutive_complete_empty_group_scans',0)>=2,'actual typing process lifecycle incomplete')
 require(process.get('argv')==tp['cargo_argv'] and process.get('cwd')=='/workspace/naming-migration-active/tcl-lsp' and process.get('artifact_sha256_before')==process.get('artifact_sha256_after')==tp['cargo_sha256'],'actual typing Cargo command/artifact mismatch')
 finished=[]
 def warns(value):
  if isinstance(value,dict):return any((key in {'warning','warnings'} and bool(child)) or warns(child) for key,child in value.items())
  if isinstance(value,list):return any(warns(child) for child in value)
  return False
 for line in (parent/'stdout.bin').read_bytes().splitlines():
  if not line.strip():continue
  event=json.loads(line,object_pairs_hook=unique_object)
  if event.get('reason')=='build-finished':finished.append(event.get('success'))
  require(not(event.get('reason')=='compiler-message' and event.get('message',{}).get('level') in {'warning','error'}) and not warns(event),'raw actual warning/error')
 require(finished==[True] and not re.search(rb'\bwarning:',(parent/'stdout.bin').read_bytes()+(parent/'stderr.bin').read_bytes(),re.I),'raw actual clean build-finished absent')
 transcript=Path(launch['transcript']);require(transcript.resolve()==launcher_path.parent/'launcher.transcript' and transcript.is_file(),'actual launcher transcript path mismatch');text=transcript.read_text(errors='replace');require('COMPLETE receipt='+str(parent)+' overall=0' in text and 'LOCK_RELEASED' in text,'actual durable completion/unlock absent')
 return {'identity':identity,'type_plan':tp,'manifest':manifest,'source_manifest_path':str(source),'typing_base':str(base)}
