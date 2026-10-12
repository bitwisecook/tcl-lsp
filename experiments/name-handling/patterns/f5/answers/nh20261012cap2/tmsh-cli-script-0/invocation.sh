set -u
mkdir -p /var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0
E=/var/tmp/tcl_lsp_nh20261012cap2/evidence/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0
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
run before-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0
if ! grep -q "was not found" "$E/before-0.stderr" "$E/before-0.stdout"; then echo "Collision or unverified absence" >&2; exit 3; fi
run input-hash sha256sum /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0.conf
if ! grep -q "^1fea69017d4a1c95247c0d9175be87f84ee6585896afcc7974ed242f3521ea28 " "$E/input-hash.stdout"; then exit 4; fi
owned+=('cli script /Common/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0')
run load tmsh load sys config merge file /var/tmp/tcl_lsp_nh20261012cap2/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0.conf
run applied-0 tmsh list cli script /Common/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0
run invoke tmsh run cli script /Common/__tcl_lsp_probe_nh20261012cap2_tmsh_cli_script_0
sleep 2
run runtime-logs grep -F 'NHO|nh20261012cap2|' /var/log/ltm
echo "configuration_saved=false" > "$E/save-status.txt"
