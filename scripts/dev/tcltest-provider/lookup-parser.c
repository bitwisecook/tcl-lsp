// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <tcl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static Tcl_HashTable names;
static void parse_script(const char *s,int n,int depth) {
    if (depth > 256) { fprintf(stderr,"depth exceeded\n"); exit(2); }
    while (n > 0) {
        Tcl_Parse p;
        int code=Tcl_ParseCommand(NULL,s,n,0,&p);
        if (code != TCL_OK) { Tcl_FreeParse(&p); return; }
        if (p.numWords && p.tokenPtr[0].numComponents==1 && p.tokenPtr[1].type==TCL_TOKEN_TEXT) {
            Tcl_Token t=p.tokenPtr[1];char *name=malloc(t.size+1);memcpy(name,t.start,t.size);name[t.size]=0;
            int fresh;Tcl_CreateHashEntry(&names,name,&fresh);free(name);
        }
        for (int i=0;i<p.numTokens;i++) {
            Tcl_Token *t=&p.tokenPtr[i];
            if(t->type==TCL_TOKEN_COMMAND && t->size>=2)parse_script(t->start+1,t->size-2,depth+1);
            if((t->type==TCL_TOKEN_WORD||t->type==TCL_TOKEN_SIMPLE_WORD)&& t->numComponents==1 && t->start[0]=='{' && p.tokenPtr[i+1].type==TCL_TOKEN_TEXT) {
                Tcl_Token v=p.tokenPtr[i+1];if(v.size<n)parse_script(v.start,v.size,depth+1);
            }
        }
        int consumed=(int)(p.commandStart-s)+p.commandSize;Tcl_FreeParse(&p);
        if(consumed<=0||consumed>n)return;s+=consumed;n-=consumed;
    }
}
int main(int argc,char **argv) {
    if(argc!=2)return 2;FILE *f=fopen(argv[1],"rb");if(!f)return 2;
    fseek(f,0,SEEK_END);long len=ftell(f);rewind(f);char *s=malloc(len+1);fread(s,1,len,f);s[len]=0;fclose(f);
    Tcl_FindExecutable(argv[0]);Tcl_InitHashTable(&names,TCL_STRING_KEYS);
    int n;const char **elements;if(Tcl_SplitList(NULL,s,&n,&elements)!=TCL_OK)return 2;
    for(int i=0;i<n;i++)parse_script(elements[i],strlen(elements[i]),0);
    Tcl_HashSearch search;for(Tcl_HashEntry *e=Tcl_FirstHashEntry(&names,&search);e;e=Tcl_NextHashEntry(&search))puts(Tcl_GetHashKey(&names,e));
    Tcl_Free((char*)elements);Tcl_DeleteHashTable(&names);free(s);Tcl_Finalize();return 0;
}
