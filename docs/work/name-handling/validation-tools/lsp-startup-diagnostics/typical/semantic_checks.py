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

"""Pure checks for the protocol's own ASCII Tcl fixture; never synthesizes LSP success."""

def require(value,message):
    if not value:raise RuntimeError(message)

def span(value):
    require(isinstance(value,dict),'range is not an object')
    values=tuple(value.get(side,{}).get(key) for side,key in [('start','line'),('start','character'),('end','line'),('end','character')])
    require(all(type(x) is int and x>=0 for x in values) and values[:2]<=values[2:],'invalid range schema')
    return values

def locations(value):
    if isinstance(value,dict):value=[value]
    require(isinstance(value,list),'locations are not an array')
    rows=[]
    for row in value:
        require(isinstance(row,dict),'location is not an object')
        uri=row.get('uri',row.get('targetUri'));region=row.get('range',row.get('targetSelectionRange'))
        require(isinstance(uri,str),'location URI unavailable');rows.append((uri,span(region)))
    require(len(rows)==len(set(rows)),'duplicate locations')
    return set(rows)

def diagnostic_items(items,unknown_line):
    require(isinstance(items,list),'diagnostic items are not an array')
    for item in items:
        require(isinstance(item,dict) and isinstance(item.get('message'),str),'invalid diagnostic schema');span(item.get('range'))
    require(any(str(x.get('code'))=='E003' and span(x['range'])[0]==unknown_line for x in items),'current-revision E003 set-arity sentinel unavailable')

def diagnostic_report(value,unknown_line):
    require(isinstance(value,dict) and value.get('kind')=='full','full pull report required without previousResultId')
    diagnostic_items(value.get('items'),unknown_line)

def marker_tokens(value,line,version):
    require(isinstance(value,dict) and isinstance(value.get('data'),list),'semantic token response unavailable')
    data=value['data'];require(len(data)%5==0 and all(type(x) is int and x>=0 for x in data),'invalid semantic token encoding')
    rows=[];at_line=0;at_column=0
    for i in range(0,len(data),5):
        dl,dc,length,kind,mods=data[i:i+5];at_line+=dl;at_column=dc if dl else at_column+dc;rows.append((at_line,at_column,length))
    require((line,4,len('revision_marker')) in rows,'current edit marker variable token missing')
    require((line,4+len('revision_marker')+1,len(str(version))) in rows,'current edit numeral geometry missing')

def rename_edits(value,uri,old_name,new_name,version):
    require(isinstance(value,dict),'rename did not return WorkspaceEdit');edits=[]
    if 'changes' in value:
        changes=value['changes'];require(isinstance(changes,dict) and set(changes)=={uri},'rename targets foreign/missing document');edits+=changes[uri]
    if 'documentChanges' in value:
        require(isinstance(value['documentChanges'],list),'documentChanges invalid')
        for change in value['documentChanges']:
            require(isinstance(change,dict) and 'textDocument' in change,'rename contains resource operation')
            doc=change['textDocument'];require(doc.get('uri')==uri and doc.get('version') in [None,version],'rename document/version mismatch');edits+=change.get('edits',[])
    require(len(edits)==2,'rename must edit exactly the fixture declaration and call')
    require(all(x.get('newText')==new_name for x in edits),'rename replacement differs')
    require({span(x.get('range')) for x in edits}=={(0,5,0,5+len(old_name)),(1,0,1,len(old_name))},'rename edit identity/ranges differ')
