"""Compare harmless System Events and AX trust queries across sandbox modes."""
import json
import pathlib
import subprocess
import sys

profile = sys.argv[1]
output = pathlib.Path(sys.argv[2])
prefixes = {
    'outside': [],
    'allow_sandbox': ['/usr/bin/sandbox-exec', '-p', '(version 1)(allow default)'],
    'network_deny_sandbox': ['/usr/bin/sandbox-exec', '-f', profile],
}
queries = {
    'system_events_name': ['/usr/bin/osascript', '-e', 'tell application "System Events" to get name'],
    'process_count': ['/usr/bin/osascript', '-e', 'tell application "System Events" to count every application process'],
    'ax_trust': ['/usr/bin/osascript', '-l', 'JavaScript', '-e', 'ObjC.import("ApplicationServices"); $.AXIsProcessTrusted()'],
}
report = {}
for mode, prefix in prefixes.items():
    report[mode] = {}
    for name, query in queries.items():
        try:
            result = subprocess.run(prefix + query, capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=6, check=False)
            report[mode][name] = {'exit_code': result.returncode, 'stdout': result.stdout.strip(), 'stderr': result.stderr.strip(), 'timed_out': False}
        except (OSError, subprocess.TimeoutExpired) as exc:
            report[mode][name] = {'exit_code': None, 'stdout': '', 'stderr': str(exc), 'timed_out': isinstance(exc, subprocess.TimeoutExpired)}
output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
