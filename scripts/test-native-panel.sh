#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
cp tests/native/panel-shell-test.js .data/panel-shell/share/gnome-shell/extensions/cstimer-panel-test@local/extension.js
cp native/gnome/cstimer-companion@local/{extension.js,notation.js,stylesheet.css,metadata.json} .data/panel-shell/share/gnome-shell/extensions/cstimer-companion@local/
export CSTIMER_TEST_BIN="$root/.data/panel-shell/bin"
export CSTIMER_TEST_FIXTURE="$root/native/linux/target/release/examples/panel_fixture"
export CSTIMER_TEST_RESTART_PID="$root/.data/panel-shell/restarted-fixture.pid"
export CSTIMER_TEST_RESTART_LOG="$root/.data/panel-shell/restarted-bus.log"
mkdir -p "$CSTIMER_TEST_BIN"
cat > "$CSTIMER_TEST_BIN/systemctl" <<'MOCK'
#!/bin/sh
# Private Shell test only: never touch the real user's systemd service.
[ "$CSTIMER_PANEL_TEST" = 1 ] || exit 2
[ "$*" = "--user start qbing.service" ] || exit 2
CSTIMER_FIXTURE_DISCONNECTED=1 LD_LIBRARY_PATH="$CSTIMER_NATIVE_LIBS" "$CSTIMER_TEST_FIXTURE" >> "$CSTIMER_TEST_RESTART_LOG" 2>&1 &
echo $! > "$CSTIMER_TEST_RESTART_PID"
MOCK
chmod +x "$CSTIMER_TEST_BIN/systemctl"
: > "$root/.data/panel-shell/restarted-bus.log"
rm -f "$CSTIMER_TEST_RESTART_PID"
export CSTIMER_PANEL_TEST=1
export CSTIMER_PANEL_REPORT="$root/.data/panel-shell/report.json"
export XDG_DATA_HOME="$root/.data/panel-shell/share"
export XDG_CONFIG_HOME="$root/.data/panel-shell/config"
export XDG_DATA_DIRS="$XDG_DATA_HOME:${XDG_DATA_DIRS:-/run/current-system/sw/share}"
python3 -c 'import os,pathlib; pathlib.Path(os.environ["CSTIMER_PANEL_REPORT"]).unlink(missing_ok=True)'
export GSETTINGS_BACKEND=memory
export LIBGL_ALWAYS_SOFTWARE=1
export NO_AT_BRIDGE=1
export CSTIMER_NATIVE_LIBS="${LD_LIBRARY_PATH:-}"
exec env -u LD_LIBRARY_PATH dbus-run-session -- bash <<'SESSION'
set -euo pipefail
root="$PWD"
env LD_LIBRARY_PATH="$CSTIMER_NATIVE_LIBS" "$root/native/linux/target/release/examples/panel_fixture" > "$root/.data/panel-shell/bus.log" 2>&1 &
fixture=$!
env -u LD_LIBRARY_PATH PATH="$CSTIMER_TEST_BIN:$PATH" gnome-shell --headless --wayland --no-x11 --wayland-display=cstimer-panel-test --virtual-monitor=1600x900 --mode=cstimer-test > "$root/.data/panel-shell/shell.log" 2>&1 &
shell_pid=$!
cleanup() {
 kill "$shell_pid" "$fixture" 2>/dev/null || true
 if test -s "$CSTIMER_TEST_RESTART_PID"; then kill "$(cat "$CSTIMER_TEST_RESTART_PID")" 2>/dev/null || true; fi
}
trap cleanup EXIT
for attempt in {1..55}; do
 if test -s "$CSTIMER_PANEL_REPORT"; then break; fi
 if ! kill -0 "$shell_pid" 2>/dev/null; then cat "$root/.data/panel-shell/shell.log"; exit 1; fi
 sleep .5
done
test -s "$CSTIMER_PANEL_REPORT"
env LD_LIBRARY_PATH="$CSTIMER_NATIVE_LIBS" "$root/native/linux/target/release/cstimer-native" --companion
sleep 2
rg "Panel action received" "$root/.data/panel-shell/bus.log"
gdbus call --session --dest org.gnome.Shell.Extensions --object-path /org/gnome/Shell/Extensions --method org.gnome.Shell.Extensions.EnableExtension cstimer-companion@local | rg "true"
env LD_LIBRARY_PATH="$CSTIMER_NATIVE_LIBS" "$root/native/linux/target/release/cstimer-native" --floating-companion
sleep .2
rg "Companion action received" "$root/.data/panel-shell/bus.log"
gdbus call --session --dest org.gnome.Shell.Screenshot --object-path /org/gnome/Shell/Screenshot --method org.gnome.Shell.Screenshot.ScreenshotArea 0 0 1600 48 false "$root/.data/panel-shell/panel.png"
cat "$CSTIMER_PANEL_REPORT"
python3 - <<'PY'
import json,os
r=json.load(open(os.environ['CSTIMER_PANEL_REPORT']))
assert r['scrambleVisible'] and len(r['rows'])==2,r
assert r['metricsStable'],r
assert 8 <= r['metricGap'] <= 12,r
assert r['notationRight']<=r['metricsLeft'],r
assert r['trailingPadding']<=5,r
assert r['width']<330,r
assert all(c['sameGeometry'] and c['ready']==c['expected'] for c in r['rowChecks']),r
assert all(row['lines']==1 for row in r['rows']),r
assert len(' '.join(row['text'] for row in r['rows']).split())==20,r
assert r['buttonRight']<=r['clockLeft'],r
assert r['time']=='13.73' and 'tps' in r['stats'] and '26' in r['stats'],r
PY
python3 - <<'PYTEST'
import ast,subprocess,time,json
prefix=['gdbus','call','--session']
def ev(code):
 r=subprocess.run(prefix+['--dest','org.gnome.Shell','--object-path','/org/gnome/Shell','--method','org.gnome.Shell.Eval',code],check=True,capture_output=True,text=True)
 assert r.stdout.startswith('(true,'),r.stdout
 return ast.literal_eval(r.stdout.split(',',1)[1].rsplit(')',1)[0].strip())
ui='Main.extensionManager.lookup("cstimer-companion@local").stateObj'
def wait(code):
 until=time.monotonic()+5
 while time.monotonic()<until:
  if ev(code)=='true':return
  time.sleep(.15)
 raise AssertionError(code)
wait(ui+'._connect.label.text === "Disconnect cube"')
# Screenshots of the actual Shell widget, using synthetic cube data only.
from pathlib import Path
r=json.loads(Path('.data/panel-shell/report.json').read_text())
def screenshot(name):
 args=['--dest','org.gnome.Shell.Screenshot','--object-path','/org/gnome/Shell/Screenshot','--method','org.gnome.Shell.Screenshot.ScreenshotArea',str(round(r['buttonLeft'])),'0',str(round(r['width'])),'32','false',str(Path('.data/panel-shell',name).resolve())]
 subprocess.run(prefix+args,check=True,capture_output=True,text=True)
screenshot('companion-scrambling.png')
ev('(()=>{const u='+ui+';u._previewPoll=u._poll;u._poll=()=>{};return true})()')
time.sleep(.2)
ev('(()=>{const u='+ui+';u._previewState=u._state;u._state={...u._state,running:false,tokens:u._state.tokens.map(t=>({...t,kind:1}))};u._render();return true})()')
wait(ui+'._readyIcon.visible === true')
time.sleep(.2)
screenshot('companion-ready.png')
ev('(()=>{const u='+ui+';u._state=u._previewState;u._poll=u._previewPoll;u._render();return true})()')
ev(ui+'._connect.activate(null); true')
wait(ui+'._state.connected === false && '+ui+'._connect.label.text === "Connect cube"')
ev(ui+'._scan.activate(null); true')
time.sleep(.4)
ev(ui+'._connect.activate(null); true')
wait(ui+'._state.connected === true && '+ui+'._connect.label.text === "Disconnect cube"')
print('Panel connection actions: disconnect, scan and reconnect without opening main window passed.')
PYTEST
rg 'Disconnect action received' "$root/.data/panel-shell/bus.log"
rg 'Scan action received' "$root/.data/panel-shell/bus.log"
rg 'Connect action received' "$root/.data/panel-shell/bus.log"
kill "$fixture";wait "$fixture" 2>/dev/null || true;sleep .5
gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell --method org.gnome.Shell.Eval 'Main.extensionManager.lookup("cstimer-companion@local").stateObj._state === null && Main.extensionManager.lookup("cstimer-companion@local").stateObj._time.text === "—"' | rg "true.*true"

python3 - <<'PYTEST'
import ast,subprocess,time
ui='Main.extensionManager.lookup("cstimer-companion@local").stateObj'
def ev(code):
 r=subprocess.run(['gdbus','call','--session','--dest','org.gnome.Shell','--object-path','/org/gnome/Shell','--method','org.gnome.Shell.Eval',code],check=True,capture_output=True,text=True)
 assert r.stdout.startswith('(true,'),r.stdout
 return ast.literal_eval(r.stdout.split(',',1)[1].rsplit(')',1)[0].strip())
ev(ui+'._connect.activate(null); true')
until=time.monotonic()+8
while time.monotonic()<until:
 if ev(ui+'._state?.connected === true')=='true':break
 time.sleep(.2)
else:raise AssertionError('Companion could not start the private service and reconnect: '+ev('JSON.stringify({starting:'+ui+'._starting,auto:'+ui+'._connectAfterStart,error:'+ui+'._cubeError,state:'+ui+'._state,owner:'+ui+'._proxy.g_name_owner})'))
print('Offline Connect starts the private service and reconnects without opening main.')
PYTEST
rg 'Connect action received' "$root/.data/panel-shell/restarted-bus.log"
if rg 'Main window requested' "$root/.data/panel-shell/restarted-bus.log"; then exit 1; fi

SESSION
