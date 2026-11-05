"""Bound detach of a task-owned mounted image to ten seconds."""
import subprocess
import sys

try:
    result = subprocess.run(['/usr/bin/hdiutil', 'detach', sys.argv[1]], capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=10, check=False)
    print(f'detach exit={result.returncode}\n{result.stdout}{result.stderr}')
    raise SystemExit(result.returncode)
except (OSError, subprocess.TimeoutExpired) as exc:
    print(f'detach unavailable or timed out: {type(exc).__name__}')
    raise SystemExit(1)
