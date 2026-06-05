#include <stdio.h>
#include <stddef.h>
#include "tclTomMath.h"
int main(void){
  printf("sizeof(mp_digit)=%zu sizeof(mp_int)=%zu  used@%zu alloc@%zu sign@%zu dp@%zu  ptr=%zu\n",
    sizeof(mp_digit), sizeof(mp_int),
    offsetof(mp_int,used), offsetof(mp_int,alloc),
    offsetof(mp_int,sign), offsetof(mp_int,dp), sizeof(void*));
  return 0;
}
