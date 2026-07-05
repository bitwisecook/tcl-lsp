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

#include <stdio.h>
#include "tommath.h"
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
