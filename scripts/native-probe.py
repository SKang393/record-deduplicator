"""Capture bounded accessibility and screen evidence from the test runner."""
import json
import pathlib
import subprocess
import sys

pid = int(sys.argv[1])
evidence = pathlib.Path(sys.argv[2])
applescript = pathlib.Path(__file__).with_name('native-ui.applescript')


def run(argv, output, timeout):
    try:
        result = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=timeout, check=False)
        output.write_text(result.stdout + result.stderr, encoding='utf-8')
        return {'exit_code': result.returncode, 'timed_out': False}
    except (OSError, subprocess.TimeoutExpired) as exc:
        output.write_text(f'{type(exc).__name__}: {exc}\n', encoding='utf-8')
        return {'exit_code': None, 'timed_out': isinstance(exc, subprocess.TimeoutExpired)}


ui = run(['/usr/bin/osascript', str(applescript), str(pid)], evidence / 'native-ui-tree.txt', 15)
ui_text = (evidence / 'native-ui-tree.txt').read_text(encoding='utf-8')
if ui['exit_code'] != 0 or 'accessibility-error=' in ui_text or 'process-not-found' in ui_text:
    ui['state'] = 'blocked'
elif 'tree-error=' in ui_text:
    ui['state'] = 'partial'
elif 'window-count=0' in ui_text:
    ui['state'] = 'no-window'
elif '\nwindow-count=' not in ui_text or '\nwindow=' not in ui_text:
    ui['state'] = 'inconclusive'
else:
    ui['state'] = 'captured'
screenshot_path = evidence / 'native-screen.png'
screen = run(['/usr/sbin/screencapture', '-x', '-t', 'png', str(screenshot_path)], evidence / 'native-screen-capture.txt', 15)
screen['nonempty_file'] = screenshot_path.is_file() and screenshot_path.stat().st_size > 0
(evidence / 'native-probe-status.json').write_text(json.dumps({'pid': pid, 'accessibility': ui, 'screenshot': screen}, indent=2) + '\n', encoding='utf-8')
