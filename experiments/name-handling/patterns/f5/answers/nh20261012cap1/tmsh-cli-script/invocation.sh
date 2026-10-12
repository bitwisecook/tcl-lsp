set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script
E=/var/tmp/tcl_lsp_nh20261012cap1/evidence/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script
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
run before-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script.conf
if ! grep -q "^41108aa55b742228569b7d3c3b5b9a08f0845372a03b39a8a9ec643044eb9cea " "$E/input-hash.stdout"; then exit 4; fi
owned+=('cli script /Common/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap1/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script.conf
run applied-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script
run invoke tmsh run cli script /Common/__tcl_lsp_probe_nh20261012cap1_tmsh_cli_script
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap1|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
