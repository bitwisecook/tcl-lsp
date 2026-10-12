set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012native1/evidence/__tcl_lsp_probe_nh20261012native1
E=/var/tmp/tcl_lsp_nh20261012native1/evidence/__tcl_lsp_probe_nh20261012native1
owned=()
run() { label=$1; shift; "$@" > "$E/$label.stdout" 2> "$E/$label.stderr"; echo $? > "$E/$label.exit"; }
cleanup() {
  for (( i=${#owned[@]}-1; i>=0; i-- )); do
    read -r -a words <<< "${owned[$i]}"
    run "delete-$i" tmsh delete "${words[@]}"
    run "after-$i" tmsh list "${words[@]}"
  done
}
trap cleanup EXIT
run before-0 tmsh list auth partition NH_nh20261012native1
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-1 tmsh list sys folder /Common/__tcl_lsp_probe_nh20261012native1_folder
if ! grep -q "was not found" "$E/before-1.stderr" "$E/before-1.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_provider
if ! grep -q "was not found" "$E/before-2.stderr" "$E/before-2.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-3 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_other
if ! grep -q "was not found" "$E/before-3.stderr" "$E/before-3.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-4 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded
if ! grep -q "was not found" "$E/before-4.stderr" "$E/before-4.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-5 tmsh list ltm rule /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign
if ! grep -q "was not found" "$E/before-5.stderr" "$E/before-5.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-6 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller
if ! grep -q "was not found" "$E/before-6.stderr" "$E/before-6.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run before-7 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller_vs
if ! grep -q "was not found" "$E/before-7.stderr" "$E/before-7.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run destination-inventory tmsh list ltm virtual one-line
if grep -q "destination 192.0.2.10:19912 " "$E/destination-inventory.stdout"; then echo "Destination collision" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012native1/__tcl_lsp_probe_nh20261012native1.conf
if ! grep -q "^0527ca5d720f1bdf96cc08b8efb10a4ffaa6da92c283bfa35ad2f74a8c0ca284 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('auth partition NH_nh20261012native1')
owned+=('sys folder /Common/__tcl_lsp_probe_nh20261012native1_folder')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native1_provider')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native1_other')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded')
owned+=('ltm rule /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign')
owned+=('ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller')
owned+=('ltm virtual /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller_vs')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012native1/__tcl_lsp_probe_nh20261012native1.conf
run applied-0 tmsh list auth partition NH_nh20261012native1
run applied-1 tmsh list sys folder /Common/__tcl_lsp_probe_nh20261012native1_folder
run applied-2 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_provider
run applied-3 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_other
run applied-4 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_folded
run applied-5 tmsh list ltm rule /NH_nh20261012native1/__tcl_lsp_probe_nh20261012native1_foreign
run applied-6 tmsh list ltm rule /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller
run applied-7 tmsh list ltm virtual /Common/__tcl_lsp_probe_nh20261012native1_folder/__tcl_lsp_probe_nh20261012native1_caller_vs
if [ "$(cat "$E/load.exit")" = 0 ] && [ "$(cat "$E/applied-7.exit")" = 0 ]; then touch "$E/traffic-ready"; for n in {1..90}; do [ -e "$E/traffic-done" ] && break; sleep 1; done; fi
sleep 5
run runtime-logs grep -F 'NHO|nh20261012native1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
