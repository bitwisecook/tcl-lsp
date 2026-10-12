#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Measure the native call boundary in each non-TMM interpreter separately."""
import datetime,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[6];sys.path.insert(0,str(ROOT/'scripts/dev/bigip-probes/name-patterns'));import capture
RUN='nh20261012nativecap1';DEST=Path(__file__).resolve().parent;REMOTE='/var/tmp/tcl_lsp_'+RUN;capture.TAG=RUN
old_rpc=capture.rpc

def rpc(cmd,data=None,timeout=180):
 r=old_rpc(cmd,data,timeout);r.stderr=b'\n'.join(x for x in r.stderr.split(b'\n') if not (x.startswith(b'** ') or b'openssh.com/pq.html' in x));return r
capture.rpc=rpc
assert rpc('test ! -e '+REMOTE+' && mkdir -m 700 '+REMOTE).returncode==0
for label,cmd in [('version','tmsh show sys version'),('platform','tmsh show sys hardware'),('tmm-roster','tmsh show sys tmm-info')]:
 r=rpc(cmd);(DEST/(label+'.stdout')).write_bytes(b'\n'.join(x for x in r.stdout.split(b'\n') if b'FIPS Module' not in x));(DEST/(label+'.stderr')).write_bytes(r.stderr)
(DEST/'provenance.json').write_text(json.dumps({'run_id':RUN,'configuration_saved':False,'timestamp_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'transport':'Authentication metadata omitted','scope':'Exact non-TMM command surface and actual lookup for five native question forms. No absent native command is replaced by a stub or stock behavior.'},indent=2)+'\n')
for ci,ctx in enumerate(['tmsh-cli-script','iapp-implementation','icall','stock-8.4','stock-8.5']):
 target='/Common/__tcl_lsp_probe_'+RUN+'_provider::target'
 specs=[('corpus-f5-rule-path','set nh_call call','$nh_call '+target+' {a b} {c d}'),('corpus-f5-computed-target','set nh_target '+target,'eval [list call $nh_target {a b} {c d}]'),('corpus-f5-prefix-arguments','set nh_prefix [list call '+target+' {a b}]','eval [linsert $nh_prefix end {{x} ::tail}]'),('corpus-f5-rule-lifetime','set nh_call call','$nh_call '+target+' {a b} {c d}'),('corpus-f5-storage-context','set nh_call call','$nh_call '+target+' {a b} {c d}')]
 selected=[{'id':'native-call-command-surface','setup':'','operation':'list [info patchlevel] [info tclversion] [namespace current] commands [info commands call] procs [info procs call] resolved [namespace which -command call]'}]
 for q,setup,op in specs:selected.append({'id':q,'question_id':q,'setup':setup,'operation':op,'scope':'Native call capability boundary for this question/context. No rule provider can be selected through an absent native command; source-site idiom and provider semantics remain separate from stock Tcl.'})
 for c in selected:c.update(context=ctx,variant_id='native-call-availability-and-dispatch-v1',exact_source=False)
 (DEST/(ctx+'-cases.json')).write_text(json.dumps(selected,indent=2)+'\n')
 job=DEST/(ctx+'-0');capture.run_job(job,REMOTE,ctx,'__tcl_lsp_probe_'+RUN+'_'+str(ci),capture.body(selected,ctx,RUN),wait=5)
 for filename in ['driver.json','invocation.json']:
  p=job/filename
  if p.exists():
   meta=json.loads(p.read_text());argv=meta.pop('argv',[]);meta['command']='bash -s' if filename=='driver.json' else argv[-1];meta['authentication_metadata']='omitted';p.write_text(json.dumps(meta,indent=2)+'\n')
 print('captured',ctx,flush=True)
(DEST/'capture-complete.json').write_text(json.dumps({'completed':True})+'\n')
