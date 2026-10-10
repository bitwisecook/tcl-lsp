from pathlib import Path
import hashlib,json,re
P=Path(__file__).resolve().parent;root=Path('/workspace/tcl-lsp');jim=Path('/workspace/.proofs/native-providers/jimtcl')
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
required={};windows=[];providers=[]
def pin(path):required[str(path)]=sha(path)
def window(provider,path,anchor,start,end):
 raw=path.read_bytes().splitlines(keepends=True);d=P/'source-windows'/provider;d.mkdir(parents=True,exist_ok=True)
 file=d/(path.name+'.'+anchor+'.txt');file.write_bytes(b''.join(raw[start-1:end]));pin(path);pin(file)
 windows.append({'provider':provider,'source':str(path),'source_sha256':sha(path),'anchor':anchor,'first_line':start,'last_line':end,'window_file':str(file),'window_sha256':sha(file)})
for version in ['8.4.20','8.5.19','8.6.18','9.0.4','9.1.0']:
 sdk=root/'tmp'/('tcl'+version);executable=sdk/'unix/tclsh'
 providers.append({'provider':version,'command':[str(executable),str(P/'probe.tcl')],'executable':str(executable),'environment':{'TCL_LIBRARY':str(sdk/'library')},'bootstrap':'Original distribution tclsh entry and pinned TCL_LIBRARY; public version observed anew in this process.'})
 for path in [executable,sdk/'unix/Makefile',sdk/'unix'/('libtcl'+'.'.join(version.split('.')[:2])+'.a'),sdk/'generic/tcl.h',sdk/'generic/tclInt.h',sdk/'generic/tclIntDecls.h',sdk/'generic/tclNamesp.c',sdk/'generic/tclProc.c']:
  pin(path)
 for path in sorted((sdk/'library').glob('*.tcl')):pin(path)
 namesp=sdk/'generic/tclNamesp.c';lines=namesp.read_text().splitlines()
 for anchor in ['TclGetOriginalCommand','NamespaceOriginCmd']:
  starts=[i for i,l in enumerate(lines,1)if l.startswith(anchor+'(')];assert len(starts)==1,(version,anchor,starts)
  start=starts[0];window(version,namesp,anchor,max(1,start-23),min(len(lines),start+80))
 if version!='8.4.20':
  ensemble=sdk/'generic/tclEnsemble.c' if (sdk/'generic/tclEnsemble.c').exists() else namesp
  ensemble_lines=ensemble.read_text().splitlines()
  starts=[i for i,l in enumerate(ensemble_lines,1)if l.startswith('Tcl_FindEnsemble(')];assert len(starts)==1
  start=starts[0];window(version,ensemble,'Tcl_FindEnsemble',start-22,min(len(ensemble_lines),start+115))
 proc=sdk/'generic/tclProc.c';lines=proc.read_text().splitlines();starts=[i for i,l in enumerate(lines,1)if l.startswith('TclFindProc(')];assert len(starts)==1
 start=starts[0];window(version,proc,'TclFindProc',start-23,start+110)
providers.append({'provider':'jim','command':[str(jim/'jimsh'),str(P/'probe.tcl')],'executable':str(jim/'jimsh'),'environment':{},'bootstrap':'Original current Jim distribution CLI static namespace/nshelper publication; actual helper bodies emitted independently. No C ensemble configuration support inferred.'})
for name in ['jimsh','libjim.a','Makefile','jimautoconf.h','jim.h','jim.c','jim-subcmd.c','jim-namespace.c','nshelper.tcl','_nshelper.c','stdlib.tcl','_stdlib.c','_load-static-exts.c','jimsh.c']:
 pin(jim/name)
ns=jim/'nshelper.tcl';lines=ns.read_text().splitlines()
for anchor,needle in [('namespace-origin','proc {namespace origin}'),('namespace-import','proc {namespace import}'),('namespace-ensemble','proc {namespace ensemble}')]:
 start=next(i for i,l in enumerate(lines,1)if l.startswith(needle));later=[i for i,l in enumerate(lines,1)if i>start and l.startswith('proc ')];end=min(later)-1 if later else len(lines);window('jim',ns,anchor,max(1,start-3),end)
ns=jim/'jim-namespace.c';lines=ns.read_text().splitlines();start=next(i for i,l in enumerate(lines,1)if l.startswith('static int JimNamespaceCmd('));window('jim',ns,'JimNamespaceCmd',start-3,len(lines))
for path in [P/'probe.tcl',P/'launch.py']:pin(path)
request={'question_id':'naming.namespace.original-deep-import-chain','document':'docs/design/analysis/name-resolution-proofs/namespace-original-deep-import-chain.md','fixture_root':'rust/tcl-registry/tests/data/native_deep_import_chain_original','problem_statement':'Can original command/procedure/ensemble introspection follow 200 imported bindings, including depths 63,64,65 and200, without a fixed64 hop truncation? What do actual providers report after source rename, an unrelated replacement at the vacated spelling, middle import replacement, source deletion/recreation and ensemble configuration through a deep import? Genuine generation/token lifetimes must not be replaced by textual spelling heuristics.','scope':'Six independent original distribution CLI processes. Exact ASCII/LF public source, no source NUL or Unicode. Four independent200-link groups isolate rename, middle replacement, deletion/recreation and ensemble queries. Queries report actual catch status/result hex only. Setup status is mandatory before attributing depth observations. C8.4/Jim unsupported ensemble operations and Jim alias-vs-import procedure queries remain distinct observed purpose limitations; no physical header, compiled opcode, generic handler or exact token identity is sampled. BIG-IP not-tested.','probe_file':str(P/'probe.tcl'),'probe_sha256':sha(P/'probe.tcl'),'launcher_file':str(P/'launch.py'),'launcher_sha256':sha(P/'launch.py'),'providers':providers,'required_sha256':required,'source_windows':windows,'case_count':95,'source_channel':'Physical probe.tcl passed as original CLI script argv; sourcebytes pinned. Public emit catches uplevel source scripts produced by normal Tcl list operations and hex-encodes their original results. Query script construction is explicitly part of this probe, not a preserved earlier payload.','status':'PREPARED, no process launched by subagent; Root owns all original provider launches'}
(P/'request.json').write_text(json.dumps(request,indent=2)+'\n')
print('Prepared',len(required),'exact required input pins,',len(windows),'original source windows; no original process launched.');print('request SHA',sha(P/'request.json'))
