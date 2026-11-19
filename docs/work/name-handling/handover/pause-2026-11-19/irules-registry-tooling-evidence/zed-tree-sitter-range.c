#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tree_sitter/api.h"
const TSLanguage *tree_sitter_apl(void);
static void emit(TSNode n,const char *s) {
 if(strcmp(ts_node_type(n),"tcl_expression")==0)
   printf("capture %u..%u %.*s\n",ts_node_start_byte(n),ts_node_end_byte(n),(int)(ts_node_end_byte(n)-ts_node_start_byte(n)),s+ts_node_start_byte(n));
 for(uint32_t i=0;i<ts_node_named_child_count(n);++i)emit(ts_node_named_child(n,i),s);
}
int main(void){
 const char *cases[]={"[list simple]","[list {]} yes]","[list \"x]y\" value]","[list a\\]b]","[list [list x] tail]","[list {]}","[list {𐐀]} tail]"};
 TSParser *p=ts_parser_new(); if(!ts_parser_set_language(p,tree_sitter_apl()))return 2;
 for(size_t i=0;i<sizeof cases/sizeof *cases;++i){char s[512];snprintf(s,sizeof s,"section main {\n  optional ( %s ) { string sni }\n}\n",cases[i]); TSTree *t=ts_parser_parse_string(p,NULL,s,(uint32_t)strlen(s));printf("case %s\n",cases[i]);emit(ts_tree_root_node(t),s);ts_tree_delete(t);}
 ts_parser_delete(p);
}
