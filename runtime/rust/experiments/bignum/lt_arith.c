#include <stdio.h>
#include "tclTomMath.h"
int main(void){
  mp_int a,b,q,r;
  mp_init_multi(&a,&b,&q,&r,NULL);
  mp_set_u64(&a,2); mp_expt_n(&a,100,&b);          /* 2**100 */
  char buf[80]; size_t w; mp_to_radix(&b,buf,sizeof buf,&w,10);
  printf("2**100 = %s  (limbs used=%d, fits_i64=%d)\n", buf, b.used, mp_count_bits(&b)<=63);
  /* floor-div check: -7 / 2 should floor to -4 with remainder +1 */
  mp_int n7,d2; mp_init_multi(&n7,&d2,NULL);
  mp_set_i64(&n7,-7); mp_set_i64(&d2,2);
  mp_div(&n7,&d2,&q,&r);
  printf("trunc: -7/2 q=%lld r=%lld\n",(long long)0,(long long)0); /* show raw mp_div (trunc) */
  char qb[32],rb[32]; size_t qw,rw; mp_to_radix(&q,qb,sizeof qb,&qw,10); mp_to_radix(&r,rb,sizeof rb,&rw,10);
  printf("mp_div(-7,2): q=%s r=%s (C-trunc; Tcl floor-adjusts to q=-4 r=1)\n",qb,rb);
  mp_clear_multi(&a,&b,&q,&r,&n7,&d2,NULL);
  return 0;
}
