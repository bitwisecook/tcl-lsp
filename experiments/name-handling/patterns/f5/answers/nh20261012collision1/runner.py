#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Measure separately identified collision-avoiding spellings without touching unowned names."""
import datetime,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[6];sys.path.insert(0,str(ROOT/'scripts/dev/bigip-probes/name-patterns'))
import capture,cases
RUN='nh20261012collision1';DEST=Path(__file__).resolve().parent;REMOTE='/var/tmp/tcl_lsp_'+RUN;capture.TAG=RUN
orig_rpc=capture.rpc

def rpc(cmd,data=None,timeout=180):
 r=orig_rpc(cmd,data,timeout);r.stderr=b'\n'.join(x for x in r.stderr.split(b'\n') if not (x.startswith(b'** ') or b'openssh.com/pq.html' in x));return r
capture.rpc=rpc
assert rpc('test ! -e '+REMOTE+' && mkdir -m 700 '+REMOTE).returncode==0
for label,cmd in [('version','tmsh show sys version'),('platform','tmsh show sys hardware'),('tmm-roster','tmsh show sys tmm-info')]:
 r=rpc(cmd);(DEST/(label+'.stdout')).write_bytes(b'\n'.join(x for x in r.stdout.split(b'\n') if b'FIPS Module' not in x));(DEST/(label+'.stderr')).write_bytes(r.stderr)
(DEST/'provenance.json').write_text(json.dumps({'run_id':RUN,'configuration_saved':False,'timestamp_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'transport':'authentication metadata omitted','scope':'Original guard failures retained independently. This run changes only enumerated procedure/variable spellings, never removes unowned variables or the native forward command.'},indent=2)+'\n')
originals={c['id']:c for c in cases.hosted_cases()}
for ci,ctx in enumerate(['tmm-RULE_INIT','tmm-procedure','tmm-traffic-event']):
 ids=['frame-wrapper','frame-wrapper-near-miss']
 if ctx=='tmm-RULE_INIT':ids+=['alias-prefix-lifetime','computed-binding-mutation','computed-qualified','list-prefix','string-prefix-near-miss','oo-prefix-deleted-near-miss']
 selected=[]
 for index,cid in enumerate(ids):
  c=dict(originals[cid]);src=c['operation'];mapping={};procs=[];variables=[]
  for old in c['owned_frame_procedures']:
   new='__tcl_lsp_probe_'+RUN+'_'+str(ci)+'_'+str(index)+'_'+old
   src=re.sub(r'(?<![A-Za-z0-9_:$])'+re.escape(old)+r'(?![A-Za-z0-9_:])',new,src)
   mapping['procedure:'+old]=new;procs.append(new)
  for old in c['owned_frame_variables']:
   new='nh_owned_'+RUN+'_'+str(index)+'_'+old
   src=src.replace('${'+old+'}','${'+new+'}')
   src=re.sub(r'\$'+re.escape(old)+r'(?![A-Za-z0-9_])','$'+new,src)
   src=re.sub(r'(?<=set )'+re.escape(old)+r'(?![A-Za-z0-9_])',new,src)
   src=re.sub(r'(?<=\} )'+re.escape(old)+r'(?=[\]\n])',new,src)
   mapping['variable:'+old]=new;variables.append(new)
  guard,cleanup=cases._hosted_guard(c['owned_namespaces'],procs,variables,c['owned_absolute_commands'])
  if c['owned_namespaces']:
   # Guard exact fallback owners as well as the requested namespace.
   fallback=['::'+x for x in ['emit','red','blue'] if re.search(r'proc '+x+r'\b',src)]
   for f in fallback:guard='if {[llength [info commands '+f+']]} {error [list ownership-collision fallback '+f+']}\n'+guard
   cleanup=cleanup.replace('    set nh_guard_remaining {}','    foreach nh_guard_fallback {'+' '.join(fallback)+'} {if {[llength [info commands $nh_guard_fallback]]} {rename $nh_guard_fallback {}}}\n    set nh_guard_remaining {}')
  c.update(question_id=c['question_id']+'-collision-avoiding-whole-program',canonical_question_id=None,requested_canonical_question_id=c.get('canonical_question_id'),variant_id='collision-avoiding-spellings-whole-program-v1',operation=src,setup=guard,cleanup=cleanup,exact_source=False,context=ctx,scope='Complete separately adapted program in actual host. Intended original final lookup is not inferred from whole-program completion. Enumerated spelling substitutions avoid unowned native commands/frame variables.',adaptation={'kind':'procedure-and-frame-variable-spelling-substitution','mapping':mapping,'original_source_unchanged':False,'unowned_state_modified':False})
  selected.append(c)
 (DEST/(ctx+'-cases.json')).write_text(json.dumps(selected,indent=2)+'\n')
 for index,c in enumerate(selected):
  job=DEST/(ctx+'-'+str(index));name='__tcl_lsp_probe_'+RUN+'_'+str(ci)+'_'+str(index)
  capture.run_job(job,REMOTE,ctx,name,capture.body([c],ctx,RUN),wait=5)
  meta=json.loads((job/'driver.json').read_text());meta.pop('argv',None);meta['command']='bash -s';meta['authentication_metadata']='omitted';(job/'driver.json').write_text(json.dumps(meta,indent=2)+'\n')
  print('captured',ctx,c['id'],flush=True)
(DEST/'capture-complete.json').write_text(json.dumps({'completed':True})+'\n')
