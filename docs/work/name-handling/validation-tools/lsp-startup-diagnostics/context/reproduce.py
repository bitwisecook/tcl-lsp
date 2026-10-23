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

"""Native diagnostic reproduction only; root-owned 88125 must bound this worker."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,sys,time
BASE=Path(__file__).resolve().parent
ROOT=Path(os.environ['TCL_LSP_DIAGNOSTIC_ROOT'])
def require(v,m):
 if not v:raise RuntimeError(m)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def load(n,p):
 s=importlib.util.spec_from_file_location(n,p);m=importlib.util.module_from_spec(s);sys.modules[n]=m;s.loader.exec_module(m);return m
p=argparse.ArgumentParser();p.add_argument('--server',type=Path,required=True);p.add_argument('--server-sha256',required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--inputs-sha256',required=True);a=p.parse_args()
require(sha(BASE/'repro-inputs.json')==a.inputs_sha256,'reproduction inputmanifest pin')
for name,h in json.loads((BASE/'repro-inputs.json').read_text()).items():require(sha(name)==h,'reproduction inputchanged '+name)
require(a.server.is_absolute() and a.server.resolve()==a.server and sha(a.server)==a.server_sha256,'actualserverpin');require(not a.out.exists() and a.out.is_absolute() and a.out.resolve()==a.out,'freshoutput');a.out.mkdir();workspace=a.out/'workspace';workspace.mkdir()
settings={'proj-a':{'formatting':{'maxLineLength':160},'diagnostics':{'W111':False},'dialect':'tcl8.4','style':{'nonAscii':'strict'},'extraCommands':['folder-a-helper','shared-util'],'libraryPaths':['/opt/proj-a/tcl-lib']},'proj-b':{'formatting':{'maxLineLength':60},'dialect':'f5-irules','style':{'nonAscii':'off'},'extraCommands':['folder-b-helper'],'libraryPaths':['/opt/proj-b/tcl-lib','/usr/local/share/tcl']}}
for n,s in settings.items():
 d=workspace/n;d.mkdir();(d/'.vscode').mkdir();(d/'.vscode/settings.json').write_text(json.dumps({'tclLsp.'+k+'.'+kk:vv for k,v in s.items() if isinstance(v,dict) for kk,vv in v.items()} | {'tclLsp.'+k:v for k,v in s.items() if not isinstance(v,dict)}));(d/'foo.tcl').write_text('# placeholder\n');(d/'extra.tcl').write_text('# placeholder — each test replaces this through the editor\n')
fixture_pins={str(p.relative_to(workspace)):sha(p) for p in workspace.rglob('*') if p.is_file()};require(sum(p.stat().st_size for p in workspace.rglob('*') if p.is_file())<=128*1024,'fixture cap')
lsp=load('context_lsp',ROOT/'.claude/skills/lsp-client/lsp_client.py');bounded=load('context_bounded',BASE/'bounded_client.py');Parent=bounded.client_class(lsp.LspClient)
class Client(Parent):
 def _answer_server_request(self,method,rid,params):
  if method!='workspace/configuration':return super()._answer_server_request(method,rid,params)
  values=[]
  for item in (params or {}).get('items',[]):
   uri=item.get('scopeUri','');value=next((v for n,v in settings.items() if uri.rstrip('/')==(workspace/n).as_uri()),{})
   section=item.get('section','tclLsp')
   if section not in ('tclLsp',''):raise RuntimeError('unexpected configurationsection '+section)
   values.append(value)
  self._send({'jsonrpc':'2.0','id':rid,'result':values})
client=Client(str(workspace),[str(a.server)],capture_directory=a.out);result={'status':'INCOMPLETE','scope':'native reproduction of original proj-a diagnostic fixtures; not editor-host acceptance','server_before':sha(a.server),'events':[]}
def report(uri,source,version,want):
 deadline=time.monotonic()+6
 while True:
  remain=deadline-time.monotonic();require(remain>0,'original6s diagnostics budget exhausted')
  answer=client.send_request('workspace/executeCommand',{'command':'tcl-lsp.currentDiagnostics','arguments':[{'uri':uri,'version':version,'source':source}]},timeout=remain)
  result['events'].append({'status':answer.get('status'),'reason':answer.get('reason'),'response_sha256':hashlib.sha256(json.dumps(answer,sort_keys=True).encode()).hexdigest()})
  if answer.get('status')=='complete':
   require(answer.get('uri')==uri and answer.get('version')==version and answer.get('source')==source,'exact source/current diagnostic owner mismatch');diags=answer.get('diagnostics');require(isinstance(diags,list),'diagnostics schema');require(any(str(d.get('code'))=='W123' and want[0] in d.get('message','') for d in diags)==want[1],'original W123 assertion failed')
   require(answer.get('dialectId')=='tcl8.4','original folder dialect mismatch')
   for field in ['revision','inputsEpoch']:require(isinstance(answer.get(field),str) and answer[field].isdigit(),'current revision metadata')
   require(isinstance(answer.get('resultId'),str) and bool(answer['resultId']),'issued resultId')
   issued={key:answer[key] for key in ['uri','version','source','dialectId','revision','inputsEpoch','resultId']}
   remaining=deadline-time.monotonic();require(remaining>0,'original publication validation deadline')
   current=client.send_request('workspace/executeCommand',{'command':'tcl-lsp.validateDiagnosticPublication','arguments':[issued]},timeout=remaining)
   require(current.get('status')=='current' and all(current.get(k)==v for k,v in issued.items()),'original isStillPublished check failed')
   return
  require(answer.get('status')=='unavailable' and answer.get('reason') in ('pending','stale'),'terminal diagnostic refusal '+json.dumps(answer));time.sleep(min(.025,max(0,deadline-time.monotonic())))
try:
 client.start();require(client.process.args==[str(a.server)] and os.getpgid(client.process.pid)==os.getpgrp(),'exactserver/ownedPGID');require(sha(Path(f'/proc/{client.process.pid}/exe'))==a.server_sha256,'runningimagepin')
 client.send_request('initialize',{'processId':os.getpid(),'rootUri':workspace.as_uri(),'workspaceFolders':[{'uri':(workspace/n).as_uri(),'name':n} for n in settings],'capabilities':{'workspace':{'configuration':True,'workspaceFolders':True},'textDocument':{'publishDiagnostics':{'relatedInformation':False}}}},timeout=30);client.send_notification('initialized',{})
 # Same two-folder dialect-settled prerequisite as original suite, never a diagnostic success surrogate.
 deadline=time.monotonic()+15
 for name in settings:
  while True:
   remain=deadline-time.monotonic();require(remain>0,'configuration settle budget')
   cfg=client.send_request('workspace/executeCommand',{'command':'tcl-lsp.getEffectiveConfig','arguments':[(workspace/name).as_uri()]},timeout=remain)
   if isinstance(cfg,dict) and cfg.get('dialect')==settings[name]['dialect']:break
   time.sleep(.075)
 for name,source,needle in [('foo.tcl','proc greet {who} {\n    puts "hi $who"\n}\ngreet world\n','puts'),('extra.tcl','folder-a-helper foo bar\n','folder-a-helper')]:
  path=workspace/'proj-a'/name;uri=path.as_uri();initial=path.read_text();client.send_notification('textDocument/didOpen',{'textDocument':{'uri':uri,'languageId':'tcl','version':1,'text':initial}});client.send_notification('textDocument/didChange',{'textDocument':{'uri':uri,'version':2},'contentChanges':[{'text':source}]});report(uri,source,2,(needle,False))
 result['status']='COMPLETE_ORIGINAL_ASSERTIONS'
except BaseException as error:
 result['error']=str(error)[:8192];raise
finally:
 client.shutdown();client.raw_stdout.close();client.raw_stderr.close();result['server_after']=sha(a.server);result['server_unchanged']=result['server_before']==result['server_after'];result['capture_error']=client.capture_error
 result['fixture_unchanged']=fixture_pins=={str(p.relative_to(workspace)):sha(p) for p in workspace.rglob('*') if p.is_file()}
 if client.capture_error or not result['server_unchanged'] or not result['fixture_unchanged']:result['status']='INCOMPLETE'
 encoded=(json.dumps(result,indent=2)+'\n').encode();require(len(encoded)<=128*1024,'metadata cap');(a.out/'result.json').write_bytes(encoded)
 require(not client.capture_error and result['server_unchanged'] and result['fixture_unchanged'],'final capture/server/fixture guard failed')
