#include "jimregexp.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static const char *patterns[]={"a","\xff","\xc0\x80","\xf0\x9f\x98\x80","(","(a|b)+","(?:a)(b)","a{2,3}?","[z-a]","\\d+","\\w+","\\mfoo\\M","^a$","a.*b","[[:digit:]]+","\\u0061","\\1","a?","[a","a**","a\\","\\","[\\x00]","a{100}","a{2,}","a.+b.*c","((a)?b)+","a b #comment","a\\ b","[[:cntrl:]]"};
static const char *subjects[]={"a","\xff","\xc0\x80","\xf0\x9f\x98\x80","aaab abc foo 12\n", "aba", "abbbc", "xyz"};
int main(int argc,char **argv){int n=argc>1?atoi(argv[1]):0,flags=argc>2?atoi(argv[2]):0;regex_t re;int code=jim_regcomp(&re,patterns[n],flags);char msg[100];jim_regerror(code,&re,msg,sizeof msg);printf("compile\t%d\t%d\t%s\n",code,re.re_nsub,msg);fflush(stdout);if(!code){printf("program\t");for(int k=0;k<re.p;k++)printf("%s%d",k?",":"",re.program[k]);puts("");fflush(stdout);for(int k=0;k<8;k++){regmatch_t match[4];code=jim_regexec(&re,subjects[k],4,match,0);printf("match\t%d\t%d",k,code);if(code==0)for(int j=0;j<4;j++)printf("\t%d,%d",match[j].rm_so,match[j].rm_eo);puts("");fflush(stdout);}}jim_regfree(&re);return 0;}
