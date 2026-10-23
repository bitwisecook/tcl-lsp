# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""One bounded real-LSP measurement case; the existing direct executor owns its process group."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,threading,time,sys,shutil
BASE=Path(__file__).resolve().parent

def require(v,m):
 if not v:raise RuntimeError(m)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def load(name,path):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);sys.modules[name]=m;spec.loader.exec_module(m);return m
p=argparse.ArgumentParser();p.add_argument('--server',required=True,type=Path);p.add_argument('--server-sha256',required=True);p.add_argument('--protocol-sha256',required=True);p.add_argument('--case',required=True);p.add_argument('--out',required=True,type=Path);p.add_argument('--project',type=Path);p.add_argument('--relative-file');a=p.parse_args();require(sha(BASE/'manifest.json')==a.protocol_sha256,'protocol manifest changed');protocol=json.loads((BASE/'manifest.json').read_text());
for name,pin in protocol['files'].items():require(sha(BASE/name)==pin,'protocol input changed '+name)
sys.path.insert(0,str(BASE))
from semantic_checks import span,locations,diagnostic_items,diagnostic_report,marker_tokens,rename_edits
from bounded_client import client_class
require(a.server.is_absolute() and a.server.resolve()==a.server and sha(a.server)==a.server_sha256,'actual server path/hash mismatch before spawn');a.out.mkdir();pins=json.loads((BASE/'input-pins.json').read_text());root=Path(pins['root'])
for rel,pin in pins['pins'].items():require(sha(root/rel)==pin,'measurement input changed '+rel)
lsp=load('shipping_lsp_client',root/'.claude/skills/lsp-client/lsp_client.py');bench=load('shipping_bench',root/'scripts/perf/bench.py');workspace=a.out/'workspace';workspace.mkdir();case=a.case;source='proc p00000000 {} {return 42}\np00000000\n';count=1
if case.startswith('flat-'):
 count=int(case.split('-')[1]);source=''.join(f'proc p{i:08} {{}} {{}}\np{i:08}\n' for i in range(count))
elif case.startswith('source-'):
 count=int(case.split('-')[1]);source='source {hop00000.tcl}\n'
 for i in range(count):
  text=f'source {{hop{i+1:05}.tcl}}\n' if i+1<count else 'proc deep_leaf {} {return 42}\n'
  (workspace/f'hop{i:05}.tcl').write_text(text)
elif case.startswith('project-'):
 count=int(case.split('-')[1])
 for i in range(count):(workspace/f'file{i:05}.tcl').write_text(f'proc project_{i:05} {{}} {{return {i}}}\n')
elif case=='large-file':
 source=''.join(f'set value{i:05} {i}\n' for i in range(20000))
elif case=='corpus-file':
 require(a.project and a.project.is_dir() and a.relative_file,'actual project and relative file required')
 rel=Path(a.relative_file);require(not rel.is_absolute() and '..' not in rel.parts,'relative source path required')
 shutil.copytree(a.project,workspace,dirs_exist_ok=True,ignore=shutil.ignore_patterns('.git','target','node_modules','.vscode-test'),symlinks=True)
 require(not any(p.is_symlink() for p in workspace.rglob('*')),'external symlink input refused')
 source=(workspace/rel).read_bytes().decode('utf-8');count=sum(p.is_file() for p in workspace.rglob('*'))
else:require(case=='typical','unknown case')
path=workspace/Path(a.relative_file) if case=='corpus-file' else workspace/'main.tcl';path.write_text(source);fixture_pins={str(p.relative_to(workspace)):sha(p) for p in workspace.rglob('*') if p.is_file()};uri=path.as_uri();events=[];client=client_class(lsp.LspClient)(str(workspace),[str(a.server)],capture_directory=a.out);sampler=None;result={'case':case,'workload_files':len(fixture_pins),'main_bytes':len(source.encode()),'fixture_pins':fixture_pins,'events':events,'status':'INCOMPLETE','server_path':str(a.server),'server_sha256_before':sha(a.server),'worker_pid':os.getpid(),'worker_pgid':os.getpgrp()}
def timed(label,method,params):
    started=time.perf_counter_ns()
    try:
        value=client.send_request(method,params,timeout=30)
        require(not client.capture_error,'bounded capture failed: '+str(client.capture_error))
        events.append({'event':label,'elapsed_ms':(time.perf_counter_ns()-started)/1e6,'outcome':'response','response_sha256':hashlib.sha256(json.dumps(value,sort_keys=True).encode()).hexdigest()})
        return value
    except Exception as error:
        events.append({'event':label,'elapsed_ms':(time.perf_counter_ns()-started)/1e6,'outcome':'error','error':str(error)})
        raise

def write_document(doc_uri,text,version,opening=False):
    document={'uri':doc_uri,'version':version}
    if opening:client.send_notification('textDocument/didOpen',{'textDocument':{**document,'languageId':'tcl','text':text}})
    else:client.send_notification('textDocument/didChange',{'textDocument':document,'contentChanges':[{'text':text}]})

def check_semantic_probe(doc_uri,name,version):
    expected_decl=(doc_uri,(0,5,0,5+len(name)));expected_call=(doc_uri,(1,0,1,len(name)))
    params={'textDocument':{'uri':doc_uri},'position':{'line':1,'character':3}}
    definition=timed('probe-definition-v'+str(version),'textDocument/definition',params)
    require(locations(definition)=={expected_decl},'probe definition did not select exact current declaration')
    references=timed('probe-references-v'+str(version),'textDocument/references',{**params,'context':{'includeDeclaration':True}})
    require(locations(references)=={expected_decl,expected_call},'probe references differ from actual declaration/call set')
    prepare=timed('probe-prepareRename-v'+str(version),'textDocument/prepareRename',params)
    require(isinstance(prepare,dict),'prepareRename refused exact fixture')
    region=prepare.get('range',prepare);require(span(region)==expected_decl[1],'prepareRename does not anchor exact current local declaration')
    if 'placeholder' in prepare:require(prepare['placeholder']==name,'prepareRename placeholder is stale')
    edits=timed('probe-rename-v'+str(version),'textDocument/rename',{**params,'newName':name+'_renamed'})
    rename_edits(edits,doc_uri,name,name+'_renamed',version)
    lenses=timed('probe-codeLens-v'+str(version),'textDocument/codeLens',{'textDocument':{'uri':doc_uri}})
    require(isinstance(lenses,list),'codeLens unavailable');selected=[]
    for lens in lenses:
        require(isinstance(lens,dict),'invalid lens');geometry=span(lens.get('range'))
        if geometry[:2]<=(0,5)<geometry[2:]:selected.append(lens)
    require(len(selected)==1,'exact fixture declaration lens absent/ambiguous')
    lens=selected[0];require(bool(lens.get('data')) or isinstance(lens.get('command'),dict),'lens has no issued data/command')
    if lens.get('data'):lens=timed('probe-codeLens-resolve-v'+str(version),'codeLens/resolve',lens)
    command=lens.get('command',{}) if isinstance(lens,dict) else {}
    require(command.get('command')=='tcl-lsp.showReferences','resolved lens has no actual reference command')
    require(command.get('title') in ['1 reference','at least 1 reference'],'lens title does not report the actual one-call census')
    arguments=command.get('arguments');require(isinstance(arguments,list) and len(arguments)==3,'reference command arguments missing')
    require(arguments[0]==doc_uri and locations(arguments[2])=={expected_call},'resolved lens does not retain actual one-call census')
    events.append({'event':'semantic-probe-validated','version':version,'name':name,'expected_declaration':expected_decl,'expected_call':expected_call,'rename_edit_count':2})

try:
    require(sha(a.server)==a.server_sha256,'server changed immediately before start')
    client.start();require(client.process.args==[str(a.server)],'client launched another command')
    require(os.getpgid(client.process.pid)==os.getpgrp(),'server escaped inherited owned process group')
    result['server_pid']=client.process.pid;result['server_pgid']=os.getpgid(client.process.pid)
    result['server_exe_sha256_at_start']=sha(Path(f'/proc/{client.process.pid}/exe'));require(result['server_exe_sha256_at_start']==a.server_sha256,'actual running native image hash differs')
    sampler=bench.Sampler(client.process.pid);sampler.start();lsp.initialize(client)
    opened=time.perf_counter_ns();write_document(uri,source,1,opening=True)
    first=timed('cold-first-tokens','textDocument/semanticTokens/full',{'textDocument':{'uri':uri}})
    require(isinstance(first,dict) and isinstance(first.get('data'),list) and bool(first['data']),'first token response unavailable/empty')
    result['didOpen_to_first_tokens_ms']=(time.perf_counter_ns()-opened)/1e6
    scan=time.perf_counter_ns();client.wait_for_workspace_scan(timeout=30)
    events.append({'event':'independent-workspace-scan-barrier','elapsed_ms':(time.perf_counter_ns()-scan)/1e6})
    # A separate known fixture makes navigation/edit identity checkable even in a corpus workload.
    probe_name='__latency_probe_v1';probe_path=workspace/'__latency_protocol_probe.tcl'
    require(not probe_path.exists(),'corpus conflicts with fixed protocol fixture')
    probe_text=f'proc {probe_name} {{}} {{return 42}}\n{probe_name}\n';probe_path.write_text(probe_text)
    fixture_pins[str(probe_path.relative_to(workspace))]=sha(probe_path);probe_uri=probe_path.as_uri();write_document(probe_uri,probe_text,1,opening=True)
    initial=timed('probe-initial-diagnostic-barrier','textDocument/diagnostic',{'textDocument':{'uri':probe_uri}})
    require(isinstance(initial,dict) and initial.get('kind')=='full' and isinstance(initial.get('items'),list),'initial full diagnostic report unavailable')
    require(not any(str(item.get('code'))=='E003' for item in initial['items']),'initial fixture unexpectedly has arity sentinel')
    check_semantic_probe(probe_uri,probe_name,1)
    # Rename the declaration AND call and add one uniquely positioned diagnostic sentinel.
    current_name='__latency_probe_v2';current_text=f'proc {current_name} {{}} {{return 42}}\n{current_name}\nset __latency_arity_sentinel 1 2\n'
    changed=time.perf_counter_ns();write_document(probe_uri,current_text,2)
    # Observe a version-tagged semantic notification, never poll naming queries to settle.
    deadline=time.monotonic()+30;publication=None
    while time.monotonic()<deadline:
        require(not client.capture_error,str(client.capture_error))
        with client._lock:notes=list(client._notifications)
        for note in notes:
            params=note.get('params',{})
            if note.get('method')=='textDocument/publishDiagnostics' and params.get('uri')==probe_uri and type(params.get('version')) is int and params.get('version')==2:
                try:diagnostic_items(params.get('diagnostics'),2)
                except RuntimeError:continue
                publication=params;break
        if publication is not None:break
        time.sleep(.01)
    require(publication is not None,'exact revision2 diagnostic convergence not observed')
    events.append({'event':'diagnostics-converged-notification','version':2,'elapsed_ms':(time.perf_counter_ns()-changed)/1e6,'expected_code':'E003','expected_line':2})
    report=timed('probe-current-full-diagnostic-barrier','textDocument/diagnostic',{'textDocument':{'uri':probe_uri}})
    diagnostic_report(report,2);check_semantic_probe(probe_uri,current_name,2)
    # Warm tokens must contain the actual new edit's variable and numeral geometry.
    marker_line=source.count('\n')+1
    for version in range(2,22):
        text=source+f'\nset revision_marker {version}\n';started=time.perf_counter_ns();write_document(uri,text,version)
        value=timed('warm-edit-tokens','textDocument/semanticTokens/full',{'textDocument':{'uri':uri}})
        marker_tokens(value,marker_line,version);events[-1].update(edit_to_response_ms=(time.perf_counter_ns()-started)/1e6,version=version,marker_geometry_verified=True)
    # Retain exact ID and serialize writes while the edit overlaps a cancellable request.
    written=threading.Event();record={};send=client._send;send_lock=threading.Lock()
    def observed_send(data,**kwargs):
        with send_lock:answer=send(data,**kwargs)
        if data.get('method')=='textDocument/references' and 'id' in data:record['id']=data['id'];written.set()
        return answer
    client._send=observed_send
    def overlapping():
        try:
            record['response']=client.send_request('textDocument/references',{'textDocument':{'uri':probe_uri},'position':{'line':1,'character':3},'context':{'includeDeclaration':True}},timeout=30);record['outcome']='completed'
        except Exception as error:record.update(outcome='error',error=str(error))
    started=time.perf_counter_ns();thread=threading.Thread(target=overlapping,daemon=True);thread.start();require(written.wait(30),'overlap request not written')
    client.send_notification('$/cancelRequest',{'id':record['id']});write_document(uri,source+'\nset revision_marker 22\n',22)
    value=timed('post-cancel-current-tokens','textDocument/semanticTokens/full',{'textDocument':{'uri':uri}});marker_tokens(value,marker_line,22)
    thread.join(timeout=30);require(not thread.is_alive(),'overlap request did not terminate')
    if record.get('outcome')=='completed':require(locations(record['response'])=={(probe_uri,(0,5,0,5+len(current_name))),(probe_uri,(1,0,1,len(current_name)))},'completed overlap response targets wrong revision')
    else:require('LSP error -32800:' in record.get('error','') or 'LSP error -32801:' in record.get('error',''),'overlap failed other than cancellation/content-modified')
    record.pop('response',None);events.append({'event':'cancellation-overlap','elapsed_ms':(time.perf_counter_ns()-started)/1e6,**record})
    for cycle in range(3):
        client.send_notification('textDocument/didClose',{'textDocument':{'uri':uri}})
        client.send_notification('workspace/didChangeConfiguration',{'settings':{'tclLsp':{'dialect':'tcl9.0' if cycle%2 else 'tcl8.6'}}})
        text=source+f'\nset revision_marker {100+cycle}\n';write_document(uri,text,100+cycle,opening=True)
        value=timed('reopen-current-tokens','textDocument/semanticTokens/full',{'textDocument':{'uri':uri}});marker_tokens(value,marker_line,100+cycle)
    status=Path(f'/proc/{client.process.pid}/status').read_text();result['linux_vm_hwm_kib']=next(int(line.split()[1]) for line in status.splitlines() if line.startswith('VmHWM:'))
    result['retained_rss_after_churn']=sampler.latest();require(not client.capture_error,str(client.capture_error));result['status']='MEASURED_NOT_ADMITTED'
except Exception as error:result['error']=str(error);raise
finally:
    if sampler:sampler.stop();result['sampled_rss_cpu_timeline']=sampler.samples;result['sampled_peak_rss_kib']=max((x['rss_kib'] for x in sampler.samples),default=None)
    result['capture_error']=client.capture_error;result['server_wire_bytes']=client.wire_bytes;result['decoded_capture_bytes']=client.capture_bytes
    result['last_request_method']=client.last_request_method;result['last_request_id']=client.last_request_id
    client.shutdown();client.raw_stdout.close();client.raw_stderr.close();result['server_exit']=client.process.returncode if client.process else None
    result['fixture_pins_after']={str(path.relative_to(workspace)):sha(path) for path in workspace.rglob('*') if path.is_file()};result['fixture_bytes_stable']=result['fixture_pins_after']==fixture_pins
    result['server_sha256_after']=sha(a.server);result['capture_error']=client.capture_error
    if result['server_sha256_after']!=a.server_sha256 or not result['fixture_bytes_stable'] or client.capture_error:result['status']='INCOMPLETE'
    (a.out/'measurement.json').write_text(json.dumps(result,indent=2)+'\n')
    require(result['server_sha256_after']==a.server_sha256 and result['fixture_bytes_stable'] and not client.capture_error,'final server/fixture/capture guard failed')
