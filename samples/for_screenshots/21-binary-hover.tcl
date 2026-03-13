set out [binary format c3S4i@20f \
    65 66 67 \
    1024 2048 3072 4096 \
    1234 3.25]
#                      ^--- cursor
binary scan $out c3S4i@20f c1 c2 c3 s1 s2 s3 s4 i1 f1
