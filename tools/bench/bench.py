"""Memory and idle-CPU baseline for browsers (P0-07, reused in P5-04).

Opens the URLs in urls.txt in one browser with a fresh temporary profile, waits, then records:
  * total private memory of all the browser's processes
      Windows: sum of Private Bytes (Win32_Process.PrivatePageCount)
      Linux:   sum of PSS from /proc/<pid>/smaps_rollup
  * average CPU over an idle window (sum of user+kernel time of those processes / wall time,
    shown as a percentage of ONE core and of the whole machine)
The browser's processes are its process tree plus anything whose command line names the
temporary profile. The profile is deleted afterwards. Python standard library only.

Usage:
  python tools/bench/bench.py --list
  python tools/bench/bench.py edge-tabs spike [--wait 60] [--idle 60] [--out out/bench.json]
"""

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
IS_WIN = platform.system() == 'Windows'


def urls():
    lines = (HERE / 'urls.txt').read_text(encoding='utf-8').splitlines()
    return [l.strip() for l in lines if l.strip() and not l.strip().startswith('#')]


def first_existing(paths):
    for p in paths:
        if p and Path(p).is_file():
            return str(p)
    return None


def env_path(var, *rest):
    base = os.environ.get(var)
    return str(Path(base, *rest)) if base else None


# name -> (executable candidates, how to build the command line)
def chromium_cmd(exe, profile, us, windows=False):
    cmd = [exe, f'--user-data-dir={profile}', '--no-first-run', '--no-default-browser-check',
           '--disable-sync']
    if windows:
        return [cmd + ['--new-window', us[0]]] + [[exe, f'--user-data-dir={profile}', '--new-window', u] for u in us[1:]]
    return [cmd + us]


def firefox_cmd(exe, profile, us, windows=False):
    # Quiet first-run pages so they don't add tabs.
    Path(profile, 'user.js').write_text('\n'.join([
        'user_pref("browser.aboutwelcome.enabled", false);',
        'user_pref("datareporting.policy.dataSubmissionPolicyBypassNotification", true);',
        'user_pref("toolkit.telemetry.reportingpolicy.firstRun", false);',
        'user_pref("browser.startup.homepage_override.mstone", "ignore");',
        'user_pref("browser.shell.checkDefaultBrowser", false);',
    ]) + '\n', encoding='utf-8')
    base = [exe, '-no-remote', '-profile', str(profile)]
    if windows:
        return [base + ['-new-window', us[0]]] + [[exe, '-profile', str(profile), '-new-window', u] for u in us[1:]]
    return [base + us]


def spike_cmd(exe, profile, us, windows=True):
    lst = Path(profile, 'urls.txt')
    lst.write_text('\n'.join(us) + '\n', encoding='utf-8')
    return [[exe, f'--urls-file={lst}', f'--profile-dir={profile}']]


def spike_exe():
    if IS_WIN:
        return first_existing([ROOT / 'spike/p0-04-helm/target/release/p0-05-extensions.exe'])
    return first_existing([ROOT / 'spike/p0-04-helm/target/release/p0-05-extensions'])


def constellations_exe():
    if IS_WIN:
        return first_existing([
            ROOT / 'target/release/constellations.exe',
            ROOT / 'target/debug/constellations.exe',
        ])
    return first_existing([
        ROOT / 'target/release/constellations',
        ROOT / 'target/debug/constellations',
    ])


def spike_env():
    env = dict(os.environ)
    # The folder that holds the CEF runtime (not the downloaded cef_binary_*.tar.bz2 next to it).
    lib = 'libcef.dll' if IS_WIN else 'libcef.so'
    builds = sorted(p.parent for p in (ROOT / 'spike/p0-04-helm/target/release/build').glob(f'cef-dll-sys*/*/out/cef_*/{lib}'))
    if builds:
        sep = ';' if IS_WIN else ':'
        key = 'PATH' if IS_WIN else 'LD_LIBRARY_PATH'
        env[key] = str(builds[0]) + sep + env.get(key, '')
    return env


def constellations_env():
    env = dict(os.environ)
    lib = 'libcef.dll' if IS_WIN else 'libcef.so'
    builds = sorted(p.parent for p in (ROOT / 'target').glob(f'**/cef_windows*/{lib}'))
    if builds:
        sep = ';' if IS_WIN else ':'
        key = 'PATH' if IS_WIN else 'LD_LIBRARY_PATH'
        env[key] = str(builds[0]) + sep + env.get(key, '')
    return env


BROWSERS = {
    'chrome': (lambda: first_existing([env_path('ProgramFiles', 'Google/Chrome/Application/chrome.exe'),
                                       env_path('ProgramFiles(x86)', 'Google/Chrome/Application/chrome.exe'),
                                       env_path('LOCALAPPDATA', 'Google/Chrome/Application/chrome.exe'),
                                       '/usr/bin/google-chrome', '/usr/bin/google-chrome-stable']), chromium_cmd, False),
    'edge-tabs': (lambda: first_existing([env_path('ProgramFiles(x86)', 'Microsoft/Edge/Application/msedge.exe'),
                                          env_path('ProgramFiles', 'Microsoft/Edge/Application/msedge.exe')]), chromium_cmd, False),
    'edge-windows': (lambda: first_existing([env_path('ProgramFiles(x86)', 'Microsoft/Edge/Application/msedge.exe'),
                                             env_path('ProgramFiles', 'Microsoft/Edge/Application/msedge.exe')]), chromium_cmd, True),
    'brave': (lambda: first_existing([env_path('ProgramFiles', 'BraveSoftware/Brave-Browser/Application/brave.exe'),
                                      env_path('LOCALAPPDATA', 'BraveSoftware/Brave-Browser/Application/brave.exe'),
                                      '/usr/bin/brave-browser']), chromium_cmd, False),
    'firefox': (lambda: first_existing([env_path('ProgramFiles', 'Mozilla Firefox/firefox.exe'),
                                        env_path('ProgramFiles(x86)', 'Mozilla Firefox/firefox.exe'),
                                        '/usr/bin/firefox']), firefox_cmd, False),
    'spike': (spike_exe, spike_cmd, True),
    'constellations': (constellations_exe, spike_cmd, True),
}


# ---- process sampling -------------------------------------------------------------------

def snapshot():
    """pid -> dict(ppid, cmd, private_bytes, cpu_seconds)."""
    if IS_WIN:
        ps = ('Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,CommandLine,'
              'PrivatePageCount,UserModeTime,KernelModeTime | ConvertTo-Json -Compress')
        raw = subprocess.run(['powershell', '-NoProfile', '-Command', ps], capture_output=True, text=True,
                             encoding='utf-8', errors='replace', check=True).stdout
        out = {}
        for p in json.loads(raw):
            out[p['ProcessId']] = dict(ppid=p['ParentProcessId'], cmd=p.get('CommandLine') or '',
                                       mem=int(p.get('PrivatePageCount') or 0),
                                       cpu=(int(p.get('UserModeTime') or 0) + int(p.get('KernelModeTime') or 0)) / 1e7)
        return out
    out = {}
    tick = os.sysconf('SC_CLK_TCK')
    for d in Path('/proc').iterdir():
        if not d.name.isdigit():
            continue
        try:
            stat = (d / 'stat').read_text()
            rest = stat[stat.rindex(')') + 2:].split()
            ppid, utime, stime = int(rest[1]), int(rest[11]), int(rest[12])
            cmd = (d / 'cmdline').read_bytes().replace(b'\0', b' ').decode(errors='replace')
            pss = 0
            for line in (d / 'smaps_rollup').read_text().splitlines():
                if line.startswith('Pss:'):
                    pss = int(line.split()[1]) * 1024
                    break
            out[int(d.name)] = dict(ppid=ppid, cmd=cmd, mem=pss, cpu=(utime + stime) / tick)
        except (OSError, ValueError, IndexError):
            continue
    return out


def browser_pids(snap, roots, profile):
    pids = {p for p, v in snap.items() if str(profile).lower() in v['cmd'].lower()} | {r for r in roots if r in snap}
    changed = True
    while changed:
        changed = False
        for p, v in snap.items():
            if p not in pids and v['ppid'] in pids:
                pids.add(p)
                changed = True
    return pids


def kill(pids):
    for p in pids:
        try:
            if IS_WIN:
                subprocess.run(['taskkill', '/F', '/PID', str(p)], capture_output=True)
            else:
                os.kill(p, 9)
        except OSError:
            pass


def run_one(name, wait, idle):
    find, build, windows = BROWSERS[name]
    exe = find()
    if not exe:
        return dict(browser=name, error='not installed')
    profile = Path(tempfile.mkdtemp(prefix=f'constellations-bench-{name}-'))
    us = urls()
    env = constellations_env() if name == 'constellations' else (spike_env() if name == 'spike' else None)
    roots = []
    try:
        for i, cmd in enumerate(build(exe, profile, us, windows)):
            roots.append(subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).pid)
            if i == 0:
                time.sleep(4)  # let the first instance start before sending more windows
        time.sleep(wait)
        s1 = snapshot()
        t1 = time.time()
        pids1 = browser_pids(s1, roots, profile)
        mem = sum(s1[p]['mem'] for p in pids1)
        time.sleep(idle)
        s2 = snapshot()
        t2 = time.time()
        pids2 = browser_pids(s2, roots, profile)
        common = pids1 & pids2
        cpu = sum(s2[p]['cpu'] - s1[p]['cpu'] for p in common)
        cores = os.cpu_count() or 1
        return dict(browser=name, exe=exe, urls=len(us), processes=len(pids1),
                    private_mb=round(mem / 2**20, 1),
                    cpu_one_core_pct=round(100 * cpu / (t2 - t1), 1),
                    cpu_machine_pct=round(100 * cpu / (t2 - t1) / cores, 2),
                    wait_s=wait, idle_s=idle)
    finally:
        kill(browser_pids(snapshot(), roots, profile))
        time.sleep(3)
        shutil.rmtree(profile, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('browsers', nargs='*')
    ap.add_argument('--wait', type=int, default=60, help='seconds after opening before measuring memory')
    ap.add_argument('--idle', type=int, default=60, help='seconds of idle CPU measurement')
    ap.add_argument('--out', default=None, help='append results as JSON lines to this file')
    ap.add_argument('--list', action='store_true')
    a = ap.parse_args()
    if a.list or not a.browsers:
        for n, (find, _, _) in BROWSERS.items():
            print(f'{n:13} {find() or "not installed"}')
        return
    for n in a.browsers:
        if n not in BROWSERS:
            sys.exit(f'unknown browser {n!r}; see --list')
        r = run_one(n, a.wait, a.idle)
        r['date'] = time.strftime('%Y-%m-%d %H:%M')
        print(json.dumps(r))
        if a.out:
            Path(a.out).parent.mkdir(parents=True, exist_ok=True)
            with open(a.out, 'a', encoding='utf-8') as f:
                f.write(json.dumps(r) + '\n')


if __name__ == '__main__':
    main()
