#!/usr/bin/env python3
"""Bounded, offline macOS UI smoke using only a caller-supplied synthetic RIS file.

All evidence is written to a new, caller-supplied directory. The caller owns
access control and encryption; this program never reads an encryption key.
"""

from __future__ import annotations

import argparse
from collections import Counter
import csv
import ctypes
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import signal
import subprocess
import sys
import time

UI_SCRIPT = Path(__file__).with_name("ui.applescript")
PER_CALL_SECONDS = 6
PANEL_CALL_SECONDS = 12
STAGE_SECONDS = 20
DEFAULT_SECONDS = 120
DIAGNOSTIC_RESERVE_SECONDS = 7
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
EXPORT_FILES = {"project.json", "retained.ris", "retained.csv", "csv_loss_report.json",
                "csv_formula_escapes.json", "suppressed.ris", "results.json", "audit.json",
                "review_queue.json", "relations.json", "loss_report.json"}


class Blocked(Exception):
    pass


class Failed(Exception):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def verify_project(project_path: Path, source_bytes: bytes) -> dict:
    """Verify the native save contains the exact synthetic source and records."""
    try:
        project = json.loads(project_path.read_bytes())
        sources = project["sources"]
        summaries = project["summaries"]
        records = project["records"]
        if len(sources) != 1 or len(summaries) != 1:
            raise ValueError("expected exactly one source and summary")
        source = sources[0]
        manifest = source["manifest"]
        stored = bytes(source["bytes"])
        summary = summaries[0]
        if stored != source_bytes or manifest["content_sha256"] != sha256(source_bytes):
            raise ValueError("saved source does not match the synthetic fixture")
        if summary["source_id"] != manifest["logical_id"]:
            raise ValueError("saved source and summary IDs differ")
        if summary["incomplete"] is not False or summary["rejected_records"] != 0:
            raise ValueError("saved import is incomplete or contains rejected records")
        if not isinstance(records, list) or not records or summary["accepted_records"] != len(records):
            raise ValueError("saved record count does not match accepted records")
        if project.get("matching_pending", False) is not False or project["events"] != []:
            raise ValueError("saved project is pending or contains review events")
    except (KeyError, TypeError, ValueError, OSError, UnicodeError) as error:
        raise Failed(f"saved project verification failed: {error}") from error
    return {"source_sha256": sha256(source_bytes), "accepted_records": len(records)}


def ris_titles(data: bytes) -> list[str]:
    """Count complete simple RIS records and extract their unambiguous titles."""
    titles = []
    current = None
    for line in data.decode("utf-8-sig").splitlines():
        if line.startswith("TY  -"):
            if current is not None:
                raise ValueError("nested RIS record")
            current = []
        elif line.startswith("TI  -"):
            if current is None:
                raise ValueError("RIS title outside a record")
            current.append(line[6:].strip())
        elif line.startswith("ER  -"):
            if current is None or len(current) != 1 or not current[0]:
                raise ValueError("incomplete or ambiguous RIS record")
            titles.append(current[0])
            current = None
        elif line.strip() and current is None:
            raise ValueError("RIS content outside a record")
    if current is not None:
        raise ValueError("unterminated RIS record")
    return titles


def verify_export(directory: Path, source_bytes: bytes, saved_project: Path) -> dict:
    """Verify every manifest digest and both screening formats against the fixture."""
    try:
        fixture_titles = ris_titles(source_bytes)
        if not fixture_titles:
            raise ValueError("fixture has no complete records")
        manifest = json.loads((directory / "manifest.json").read_bytes())
        entries = manifest["files"]
        if manifest["schema_version"] != 1 or not isinstance(entries, dict) or set(entries) != EXPORT_FILES:
            raise ValueError("export manifest does not list the complete expected artifacts")
        for name, entry in entries.items():
            path = directory / name
            if path.is_symlink() or not path.is_file():
                raise ValueError(f"export artifact is missing or a symlink: {name}")
            data = path.read_bytes()
            if type(entry["bytes"]) is not int or entry["bytes"] != len(data) or entry["sha256"] != sha256(data):
                raise ValueError(f"export artifact size or SHA-256 mismatch: {name}")
        if (directory / "project.json").read_bytes() != saved_project.read_bytes():
            raise ValueError("exported project differs from the native saved project")
        verify_project(directory / "project.json", source_bytes)
        results = json.loads((directory / "results.json").read_bytes())
        audit = json.loads((directory / "audit.json").read_bytes())
        expected = len(fixture_titles)
        expected_counts = {"recognized_records": expected, "accepted_records": expected,
                           "retained_records": expected, "suppressed_records": 0,
                           "rejected_records": 0, "unparsed_fragments": 0}
        for name, value in expected_counts.items():
            if results["counts"][name] != value or audit["counts"][name] != value:
                raise ValueError(f"export count differs from the fixture: {name}")
        if len(audit["sources"]) != 1 or audit["sources"][0]["source"]["content_sha256"] != sha256(source_bytes):
            raise ValueError("audit source hash does not match the fixture")
        exported_ris = ris_titles((directory / "retained.ris").read_bytes())
        if ris_titles((directory / "suppressed.ris").read_bytes()):
            raise ValueError("synthetic fixture unexpectedly has suppressed records")
        with (directory / "retained.csv").open("r", encoding="utf-8-sig", newline="") as stream:
            reader = csv.DictReader(stream)
            if reader.fieldnames is None or not {"record_id", "title"}.issubset(reader.fieldnames):
                raise ValueError("export CSV is missing record_id or title")
            rows = list(reader)
        result_ids = [item["record"]["raw"]["record_id"] for item in results["retained"]]
        saved = json.loads(saved_project.read_bytes())
        saved_ids = [item["raw"]["record_id"] for item in saved["records"]]
        clusters = results["clusters"]
        csv_ids = [row["record_id"] for row in rows]
        if (len(rows) != expected or len(exported_ris) != expected or
                Counter(row["title"] for row in rows) != Counter(fixture_titles) or
                Counter(exported_ris) != Counter(fixture_titles) or
                len(saved_ids) != expected or len(set(saved_ids)) != expected or
                len(result_ids) != expected or len(set(result_ids)) != expected or
                len(set(csv_ids)) != expected or set(csv_ids) != set(result_ids) or
                set(result_ids) != set(saved_ids) or results["suppressed"] != [] or
                len(clusters) != expected or any(len(cluster) != 1 for cluster in clusters) or
                {cluster[0] for cluster in clusters} != set(saved_ids)):
            raise ValueError("CSV or RIS records do not match fixture titles and retained IDs")
    except (KeyError, TypeError, ValueError, OSError, UnicodeError, csv.Error) as error:
        raise Failed(f"synthetic export verification failed: {error}") from error
    return {"fixture_records": expected, "csv_records": len(rows), "ris_records": len(exported_ris),
            "manifest_sha256": sha256((directory / "manifest.json").read_bytes()),
            "manifest_entries_verified": len(entries)}


def inspect_recovery(root: Path, home: Path, pid: int, source_bytes: bytes, baseline_path: Path) -> tuple[str, dict, dict]:
    """Identify the only session created by this PID and its committed review event."""
    try:
        home = home.resolve(strict=True)
        if root.is_symlink() or not root.is_dir() or not root.resolve(strict=True).is_relative_to(home):
            raise ValueError("recovery root is outside the isolated HOME")
        sessions = list(root.iterdir())
        if len(sessions) != 1 or sessions[0].is_symlink() or not sessions[0].is_dir():
            raise ValueError("expected exactly one non-symlink recovery session")
        session = sessions[0]
        if not re.fullmatch(rf"session-\d+-{pid}-\d+", session.name):
            raise ValueError("recovery session ID does not identify the owned app PID")
        if not session.resolve(strict=True).is_relative_to(home):
            raise ValueError("recovery session is outside the isolated HOME")
        files = [session / n for n in ("source-0.bytes", "base.project.json", "journal.jsonl", "commits.jsonl")]
        if any(p.is_symlink() or not p.is_file() or not p.resolve(strict=True).is_relative_to(home) for p in files):
            raise ValueError("recovery files are unavailable or outside isolated HOME")
        source, base, journal, commits = (p.read_bytes() for p in files)
        if source != source_bytes or len(journal) > 1024 * 1024 or len(commits) > 1024 * 1024:
            raise ValueError("recovery source differs or journal exceeds the synthetic bound")
        if not journal.endswith(b"\n") or not commits.endswith(b"\n"):
            raise ValueError("recovery journal has an unfinished entry")
        entries = [json.loads(line) for line in journal.splitlines()]
        receipts = [json.loads(line) for line in commits.splitlines()]
        if not entries or len(entries) != len(receipts):
            raise ValueError("recovery entries and durable receipts differ")
        offset = 0
        previous = "record-deduplicator/desktop-recovery/v1"
        for number, (line, entry, receipt) in enumerate(zip(journal.splitlines(keepends=True), entries, receipts), 1):
            offset += len(line)
            digest_input = {key: entry[key] for key in ("schema_version", "sequence", "previous_sha256", "action")}
            digest = sha256(json.dumps(digest_input, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))
            if (entry["schema_version"] != 1 or receipt["schema_version"] != 1 or
                    entry["sequence"] != number or receipt["sequence"] != number or
                    entry["previous_sha256"] != previous or entry["sha256"] != digest or
                    receipt["journal_bytes"] != offset or receipt["entry_sha256"] != entry["sha256"]):
                raise ValueError("recovery journal and durable receipt do not agree")
            previous = entry["sha256"]
        reviews = [entry["action"]["event"] for entry in entries if entry["action"]["kind"] == "review"]
        if len(reviews) != 1 or entries[-1]["action"]["kind"] != "review":
            raise ValueError("expected one committed review action")
        event = reviews[0]
        baseline = json.loads(baseline_path.read_bytes())
        base_project = json.loads(base)
        ids = [record["raw"]["record_id"] for record in baseline["records"]]
        pair = event["action"]["pair"]
        if (len(ids) != 2 or len(set(ids)) != 2 or event["action"]["kind"] != "keep_both" or
                {pair["left_record_id"], pair["right_record_id"]} != set(ids) or
                event["input_fingerprint"] != baseline["input_fingerprint"] or
                event["policy_sha256"] != baseline["policy_sha256"] or
                len(base_project["sources"]) != 1 or bytes(base_project["sources"][0]["bytes"]) != source_bytes):
            raise ValueError("committed review does not match both imported records")
        evidence = {"session_id": session.name, "source_sha256": sha256(source),
                    "base_sha256": sha256(base), "journal_sha256": sha256(journal),
                    "commits_sha256": sha256(commits), "entries": len(entries), "review_event": event}
        return session.name, event, evidence
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        raise Failed(f"recovery pre-termination verification failed: {error}") from error


def copy_recovery_evidence(session: Path, home: Path, output: Path, evidence: dict) -> dict:
    """Copy only already-verified recovery bytes, without replacing any evidence."""
    names = {"source-0.bytes": "source_sha256", "base.project.json": "base_sha256",
             "journal.jsonl": "journal_sha256", "commits.jsonl": "commits_sha256"}
    try:
        home = home.resolve(strict=True)
        if session.is_symlink() or not session.is_dir() or not session.resolve(strict=True).is_relative_to(home):
            raise ValueError("recovery session is outside the isolated HOME")
        destination = output / "recovery-prekill-files"
        destination.mkdir(mode=0o700, exist_ok=False)
        for name, hash_key in names.items():
            source = session / name
            if source.is_symlink() or not source.is_file() or not source.resolve(strict=True).is_relative_to(home):
                raise ValueError(f"recovery file is unavailable or outside isolated HOME: {name}")
            data = source.read_bytes()
            if sha256(data) != evidence[hash_key]:
                raise ValueError(f"recovery file changed after verification: {name}")
            with (destination / name).open("xb") as copy:
                copy.write(data)
                copy.flush()
                os.fsync(copy.fileno())
        for name, hash_key in names.items():
            source = session / name
            copied = destination / name
            if (source.is_symlink() or copied.is_symlink() or
                    not source.resolve(strict=True).is_relative_to(home) or
                    sha256(source.read_bytes()) != evidence[hash_key] or
                    sha256(copied.read_bytes()) != evidence[hash_key]):
                raise ValueError(f"recovery source or copy changed during evidence capture: {name}")
        return {"directory": destination.name, "verified_files": len(names)}
    except (OSError, KeyError, TypeError, ValueError) as error:
        raise Failed(f"recovery evidence capture failed: {error}") from error


def verify_recovered_project(path: Path, baseline_path: Path, source_bytes: bytes, event: dict) -> dict:
    """Confirm native save after restoration contains the exact committed event and source."""
    try:
        recovered = json.loads(path.read_bytes())
        baseline = json.loads(baseline_path.read_bytes())
        if recovered["events"] != [event]:
            raise ValueError("restored project does not contain exactly the committed review event")
        for key in ("sources", "records", "summaries", "policy", "policy_sha256", "input_fingerprint"):
            if recovered[key] != baseline[key]:
                raise ValueError(f"restored project changed {key}")
        if len(recovered["sources"]) != 1 or bytes(recovered["sources"][0]["bytes"]) != source_bytes:
            raise ValueError("restored project source bytes differ from the fixture")
        if recovered.get("matching_pending", False) is not False:
            raise ValueError("restored matching remains pending")
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        raise Failed(f"recovered native save verification failed: {error}") from error
    return {"source_sha256": sha256(source_bytes), "project_sha256": sha256(path.read_bytes()),
            "review_event_sha256": event["event_sha256"], "review_kind": event["action"]["kind"]}


class Probe:
    def __init__(self, app: Path, fixture: Path, output: Path, sandbox_profile: Path, seconds: int,
                 allow_unavailable_bundle_id: bool = False):
        self.app = app.resolve(strict=True)
        self.fixture = fixture.resolve(strict=True)
        self.output = output.resolve()
        self.sandbox_profile = sandbox_profile.resolve(strict=True)
        self.allow_unavailable_bundle_id = allow_unavailable_bundle_id
        self.deadline = time.monotonic() + seconds
        self.process = None
        self.app_log = None
        self.stage = "preflight"
        self.status = {"schema_version": 1, "status": "blocked", "stage": self.stage,
                       "screenshots": [], "screenshots_unavailable": [], "checks": {}, "reason": "not completed"}

    def remaining(self, *, diagnostic: bool = False) -> float:
        remaining = self.deadline - time.monotonic() - (0 if diagnostic else DIAGNOSTIC_RESERVE_SECONDS)
        if remaining <= 0:
            raise Blocked("overall timeout")
        return remaining

    def command(self, argv: list[str], log_name: str, *, diagnostic: bool = False,
                timeout_limit: int = PER_CALL_SECONDS) -> subprocess.CompletedProcess:
        try:
            result = subprocess.run(argv, capture_output=True, timeout=min(timeout_limit, self.remaining(diagnostic=diagnostic)), check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            with (self.output / log_name).open("ab") as log:
                log.write((f"command unavailable or timed out: {type(error).__name__}\n").encode())
            raise Blocked(f"{self.stage}: command unavailable or timed out") from error
        with (self.output / log_name).open("ab") as log:
            log.write(b"stdout:\n" + result.stdout + b"\nstderr:\n" + result.stderr + b"\n")
        return result

    def ax(self, operation: str, target: str = "", extra: str = "", *, retry: bool = True) -> None:
        stage_deadline = min(self.deadline - DIAGNOSTIC_RESERVE_SECONDS, time.monotonic() + STAGE_SECONDS)
        last_state = "app_process"
        while time.monotonic() < stage_deadline:
            if self.process.poll() is not None:
                raise Failed(f"{self.stage}: app exited unexpectedly")
            self.check_launched_pid(self.executable)
            fallback_mode = "verified-owned-pid" if self.allow_unavailable_bundle_id else "strict"
            result = self.command(["/usr/bin/osascript", str(UI_SCRIPT), str(self.process.pid),
                                   self.bundle_id, operation, target, extra, self.executable.name,
                                   fallback_mode], "accessibility.log",
                                  timeout_limit=PANEL_CALL_SECONDS if operation.startswith("panel_") else PER_CALL_SECONDS)
            reply = result.stdout.decode("utf-8", "replace").strip()
            lines = reply.splitlines()
            identity = {"stage": self.stage, "operation": operation,
                        "state": lines[0] if lines else "no_identity_response"}
            for line in lines[1:]:
                key, separator, value = line.partition("=")
                if separator and key in {"requested_pid", "observed_pid", "observed_name", "observed_bundle",
                                         "bundle_state", "bundle_error_code", "identity_mode"}:
                    identity[key] = value[:160]
            with (self.output / "system-events-identity.jsonl").open("a", encoding="utf-8") as log:
                log.write(json.dumps(identity, ensure_ascii=True) + "\n")
            self.status["checks"]["system_events_identity"] = identity
            if identity.get("identity_mode") == "owned_pid_unavailable_bundle":
                self.status["checks"]["unavailable_bundle_fallback_used"] = True
            if result.returncode != 0:
                raise Blocked(f"{self.stage}: System Events, accessibility, or automation unavailable")
            if reply.startswith("OK"):
                return
            if lines and lines[0].startswith("WAIT:"):
                last_state = lines[0][5:]
            if not reply.startswith("WAIT:") or not retry:
                raise Blocked(f"{self.stage}: System Events identity or control unavailable ({last_state})")
            time.sleep(min(0.5, self.remaining()))
        if last_state == "bundle_unavailable":
            raise Blocked(f"{self.stage}: System Events bundle identifier remained unavailable")
        if last_state == "bundle_mismatch":
            raise Blocked(f"{self.stage}: System Events reported a persistent bundle identifier mismatch")
        if last_state == "process_name_mismatch":
            raise Blocked(f"{self.stage}: System Events process name does not match the owned executable")
        raise Blocked(f"{self.stage}: expected window, accessibility control, or native panel unavailable ({last_state})")

    def screenshot(self, name: str, *, diagnostic: bool = False) -> None:
        path = self.output / f"{name}.png"
        try:
            result = self.command(["/usr/sbin/screencapture", "-x", str(path)], "screencapture.log",
                                  diagnostic=diagnostic, timeout_limit=2)
            valid = result.returncode == 0 and path.is_file() and path.stat().st_size > len(PNG_SIGNATURE) and path.read_bytes()[:8] == PNG_SIGNATURE
        except (Blocked, OSError):
            valid = False
        if not valid:
            self.status["screenshots_unavailable"].append({"stage": self.stage, "file": path.name})
            return
        self.status["screenshots"].append({"file": path.name, "sha256": sha256(path.read_bytes())})

    def diagnostic(self) -> None:
        path = self.output / "ax-blocked.log"
        if self.process is None or self.process.poll() is not None or not hasattr(self, "bundle_id"):
            path.write_text("AX diagnostic unavailable: the launched app is not running.\n", encoding="utf-8")
            return
        try:
            self.check_launched_pid(self.executable, diagnostic=True)
            fallback_mode = "verified-owned-pid" if self.allow_unavailable_bundle_id else "strict"
            self.command(["/usr/bin/osascript", str(UI_SCRIPT), str(self.process.pid),
                          self.bundle_id, "dump", "", "", self.executable.name, fallback_mode],
                         path.name, diagnostic=True)
        except Failed as error:
            self.status.update(status="failed", stage="identity", reason=str(error))
            if not path.exists():
                path.write_text("AX diagnostic unavailable: owned process identity changed.\n", encoding="utf-8")
        except (Blocked, OSError):
            if not path.exists():
                path.write_text("AX diagnostic unavailable or timed out.\n", encoding="utf-8")

    def check_launched_pid(self, executable: Path, *, diagnostic: bool = False) -> None:
        """Require sandbox-exec to become the expected app at the owned PID."""
        try:
            libproc = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
            pidpath = libproc.proc_pidpath
            pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
            pidpath.restype = ctypes.c_int
        except (OSError, AttributeError) as error:
            raise Blocked("cannot inspect the launched process executable") from error
        expected = executable.resolve(strict=True)
        reserve = 0 if diagnostic else DIAGNOSTIC_RESERVE_SECONDS
        stage_deadline = min(self.deadline - reserve, time.monotonic() + STAGE_SECONDS)
        while time.monotonic() < stage_deadline:
            if self.process.poll() is not None:
                raise Failed("sandboxed app exited before its executable could be verified")
            buffer = ctypes.create_string_buffer(4096)
            if pidpath(self.process.pid, buffer, len(buffer)) > 0:
                try:
                    if Path(os.fsdecode(buffer.value)).resolve(strict=True) == expected:
                        if sha256(expected.read_bytes()) != self.status["inputs"]["executable_sha256"]:
                            raise Failed("owned app executable bytes changed")
                        if not self.status["checks"].get("ps_identity_recorded"):
                            ps = self.command(["/bin/ps", "-p", str(self.process.pid), "-o", "pid=", "-o", "comm="],
                                              "process-identity.log", diagnostic=diagnostic)
                            fields = ps.stdout.decode("utf-8", "replace").strip().split(maxsplit=1)
                            if ps.returncode != 0 or len(fields) != 2 or fields[0] != str(self.process.pid):
                                raise Blocked("ps did not confirm the owned app PID")
                            self.status["checks"]["ps_identity_recorded"] = True
                        self.status["checks"]["owned_pid_executable_matches"] = True
                        return
                except OSError:
                    pass
            time.sleep(min(0.2, self.remaining(diagnostic=diagnostic)))
        raise Blocked("sandbox-exec PID did not become the expected app executable")

    def kill_owned_child(self) -> int:
        """Force-stop only the live Popen child after a fresh executable check."""
        if self.process is None or self.process.poll() is not None:
            raise Failed("owned child is no longer running before forced termination")
        self.check_launched_pid(self.executable)
        pid = self.process.pid
        self.process.kill()
        try:
            result = self.process.wait(timeout=min(5, self.remaining()))
        except subprocess.TimeoutExpired as error:
            raise Blocked("owned child did not exit after forced termination") from error
        if result != -signal.SIGKILL:
            raise Failed("owned child did not exit from the requested SIGKILL")
        self.status["checks"]["forced_termination"] = {"pid": pid, "signal": "SIGKILL", "returncode": result}
        return pid

    def run(self) -> None:
        if sys.platform != "darwin":
            raise Blocked("requires a macOS GUI session")
        if self.app.suffix != ".app" or not self.fixture.is_file() or self.fixture.suffix.lower() != ".ris":
            raise Failed("expected an application bundle and a synthetic .ris fixture")
        if not UI_SCRIPT.is_file():
            raise Blocked("companion AppleScript is missing")
        try:
            with (self.app / "Contents" / "Info.plist").open("rb") as stream:
                info = plistlib.load(stream)
            self.bundle_id = info["CFBundleIdentifier"]
            executable_name = info["CFBundleExecutable"]
            if not isinstance(executable_name, str) or Path(executable_name).name != executable_name:
                raise ValueError("invalid app executable name")
            if self.bundle_id != "com.skang393.recorddeduplicator":
                raise ValueError("unexpected app bundle identifier")
            executable = self.app / "Contents" / "MacOS" / executable_name
            if not executable.is_file():
                raise ValueError("app executable is missing")
            self.executable = executable.resolve(strict=True)
            home_value = os.environ.get("HOME", "")
            if not home_value or not Path(home_value).is_absolute():
                raise ValueError("a fresh absolute HOME is required")
            self.isolated_home = Path(home_value).resolve(strict=True)
            if not self.isolated_home.is_dir():
                raise ValueError("isolated HOME must be a directory")
            self.recovery_root = self.isolated_home / "Library" / "Application Support" / self.bundle_id / "recovery"
            if self.recovery_root.is_symlink() or (self.recovery_root.exists() and any(self.recovery_root.iterdir())):
                raise ValueError("isolated HOME contains an existing recovery session")
            if not self.recovery_root.resolve().is_relative_to(self.isolated_home):
                raise ValueError("recovery root resolves outside isolated HOME")
            source_bytes = self.fixture.read_bytes()
            if not source_bytes or len(source_bytes) > 1024 * 1024:
                raise ValueError("synthetic fixture must be nonempty and at most 1 MiB")
            profile_bytes = self.sandbox_profile.read_bytes()
            if not re.search(rb"^\s*\(deny network\*\)\s*$", profile_bytes, re.MULTILINE):
                raise ValueError("sandbox profile lacks the expected deny-network rule")
        except (OSError, KeyError, TypeError, ValueError, plistlib.InvalidFileException) as error:
            raise Failed(f"invalid app bundle or fixture: {error}") from error
        self.status["inputs"] = {"bundle_id": self.bundle_id, "executable_sha256": sha256(executable.read_bytes()),
                                 "fixture_sha256": sha256(source_bytes), "sandbox_profile_sha256": sha256(profile_bytes),
                                 "allow_unavailable_bundle_id": self.allow_unavailable_bundle_id}
        self.stage = "launch"
        self.app_log = (self.output / "app.log").open("wb")
        try:
            self.process = subprocess.Popen(["/usr/bin/sandbox-exec", "-f", str(self.sandbox_profile), str(executable)],
                                            stdout=self.app_log, stderr=subprocess.STDOUT,
                                            stdin=subprocess.DEVNULL, start_new_session=True)
        except OSError as error:
            raise Blocked(f"cannot launch bundled app: {type(error).__name__}") from error
        self.status["app_pid"] = self.process.pid
        if self.sandbox_profile.read_bytes() != profile_bytes:
            raise Failed("sandbox profile changed during app launch")
        self.stage = "sandboxed_app_pid"
        self.check_launched_pid(executable)
        self.stage = "window"
        self.ax("window")
        self.screenshot("01-app-window")
        self.stage = "open_dialog"
        self.ax("click", "Choose files")
        self.screenshot("02-after-choose-files")
        self.ax("panel_check", "Open")
        self.screenshot("02-open-panel")
        self.ax("panel_goto", "Open", str(self.fixture), retry=False)
        self.ax("panel_press", "Open")
        self.stage = "import"
        self.ax("text", "Ready to import")
        self.ax("click", "Continue to review")
        self.ax("text", "Review possible duplicates")
        self.status["checks"]["native_open_and_import"] = True
        self.screenshot("03-review")
        self.stage = "save_dialog"
        self.ax("click", "Save project")
        self.ax("panel_check", "Save")
        self.screenshot("04-save-panel")
        self.ax("panel_save_name", "Save", "native-ui-project.json")
        self.ax("panel_goto", "Save", str(self.output), retry=False)
        self.ax("panel_press", "Save")
        self.stage = "saved_project"
        project_path = self.output / "native-ui-project.json"
        stage_deadline = min(self.deadline - DIAGNOSTIC_RESERVE_SECONDS, time.monotonic() + STAGE_SECONDS)
        while not project_path.is_file() and time.monotonic() < stage_deadline:
            if self.process.poll() is not None:
                raise Failed("app exited before saving the project")
            time.sleep(min(0.5, self.remaining()))
        if not project_path.is_file():
            raise Blocked("native save did not produce the expected project path")
        self.status["checks"]["saved_project"] = verify_project(project_path, source_bytes)
        if self.fixture.read_bytes() != source_bytes:
            raise Failed("synthetic input bytes changed during the probe")
        self.screenshot("05-project-saved")
        self.stage = "export_dialog"
        self.ax("click", "Export")
        self.ax("text", "Export records")
        self.ax("click", "Export CSV, RIS and audit")
        self.ax("panel_check", "Save")
        self.screenshot("06-export-panel")
        self.ax("panel_export_name", "Save", "native-ui-export")
        self.ax("panel_goto", "Save", str(self.output), retry=False)
        self.ax("panel_press", "Save")
        self.stage = "exported_records"
        export_directory = self.output / "native-ui-export"
        stage_deadline = min(self.deadline - DIAGNOSTIC_RESERVE_SECONDS, time.monotonic() + STAGE_SECONDS)
        while not (export_directory / "manifest.json").is_file() and time.monotonic() < stage_deadline:
            if self.process.poll() is not None:
                raise Failed("app exited before completing export")
            time.sleep(min(0.5, self.remaining()))
        if not (export_directory / "manifest.json").is_file():
            raise Blocked("native export did not produce the expected completion manifest")
        self.status["checks"]["export"] = verify_export(export_directory, source_bytes, project_path)
        if self.fixture.read_bytes() != source_bytes:
            raise Failed("synthetic input bytes changed during export")
        self.screenshot("07-export-complete")
        self.stage = "review_decision"
        self.ax("click", "Review")
        self.ax("text", "Review possible duplicates")
        self.ax("click_unique", "Keep both", "Keep both K")
        self.ax("text", "No pairs need review")
        self.screenshot("08-review-decision")
        self.stage = "verify_recovery_before_kill"
        session_id, review_event, recovery_evidence = inspect_recovery(
            self.recovery_root, self.isolated_home, self.process.pid, source_bytes, project_path)
        self.status["checks"]["recovery_file_copies"] = copy_recovery_evidence(
            self.recovery_root / session_id, self.isolated_home, self.output, recovery_evidence)
        with (self.output / "recovery-before-kill.json").open("x", encoding="utf-8") as stream:
            stream.write(json.dumps(recovery_evidence, indent=2, sort_keys=True) + "\n")
        self.status["checks"]["recovery_before_kill"] = {"session_id": session_id, "inside_isolated_home": True,
                                                          "review_event_sha256": review_event["event_sha256"]}
        if self.sandbox_profile.read_bytes() != profile_bytes or Path(os.environ["HOME"]).resolve(strict=True) != self.isolated_home:
            raise Failed("sandbox profile or isolated HOME changed before forced termination")
        self.stage = "forced_termination"
        old_pid = self.kill_owned_child()
        self.app_log.close()
        self.app_log = None
        self.stage = "restart"
        if self.sandbox_profile.read_bytes() != profile_bytes or sha256(self.executable.read_bytes()) != self.status["inputs"]["executable_sha256"]:
            raise Failed("sandbox profile or executable changed before restart")
        self.app_log = (self.output / "app-restart.log").open("wb")
        try:
            self.process = subprocess.Popen(["/usr/bin/sandbox-exec", "-f", str(self.sandbox_profile), str(self.executable)],
                                            stdout=self.app_log, stderr=subprocess.STDOUT,
                                            stdin=subprocess.DEVNULL, start_new_session=True,
                                            env={**os.environ, "HOME": str(self.isolated_home)})
        except OSError as error:
            raise Blocked(f"cannot restart bundled app: {type(error).__name__}") from error
        if self.process.pid == old_pid:
            raise Blocked("restarted app reused the terminated PID")
        self.status["restart_pid"] = self.process.pid
        self.status["checks"].pop("ps_identity_recorded", None)
        self.check_launched_pid(self.executable)
        self.ax("window")
        self.stage = "restore_recovery"
        self.ax("text", "Ready to recover")
        self.screenshot("09-recovery-available")
        self.ax("click_unique", f"Recover {session_id}")
        self.ax("text", "Recovery copy opened.")
        self.ax("text", "No pairs need review")
        self.screenshot("10-recovery-restored")
        self.stage = "save_restored_project"
        recovered_path = self.output / "recovered-ui-project.json"
        if recovered_path.exists() or recovered_path.is_symlink():
            raise Failed("restored project output already exists")
        self.ax("click", "Save project")
        self.ax("panel_check", "Save")
        self.screenshot("11-restored-save-panel")
        self.ax("panel_save_name", "Save", "recovered-ui-project.json")
        self.ax("panel_goto", "Save", str(self.output), retry=False)
        self.ax("panel_press", "Save")
        stage_deadline = min(self.deadline - DIAGNOSTIC_RESERVE_SECONDS, time.monotonic() + STAGE_SECONDS)
        while not recovered_path.is_file() and time.monotonic() < stage_deadline:
            if self.process.poll() is not None:
                raise Failed("restarted app exited before saving the restored project")
            time.sleep(min(0.5, self.remaining()))
        if recovered_path.is_symlink() or not recovered_path.is_file():
            raise Blocked("native save did not produce the restored project")
        self.status["checks"]["recovered_project"] = verify_recovered_project(recovered_path, project_path, source_bytes, review_event)
        if self.fixture.read_bytes() != source_bytes:
            raise Failed("synthetic input bytes changed during recovery")
        self.screenshot("12-recovery-saved")
        self.status.update(status="passed", stage="complete", reason="synthetic native open, import, save, export, and forced-termination recovery verified")

    def finish(self) -> None:
        if self.process is not None and self.process.poll() is None:
            try:
                self.process.terminate()
                try:
                    self.process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=3)
            except ProcessLookupError:
                pass
        if self.app_log is not None:
            self.app_log.close()
        if "inputs" in self.status:
            try:
                final_hash = sha256(self.sandbox_profile.read_bytes())
                self.status["inputs"]["sandbox_profile_sha256_after"] = final_hash
                if final_hash != self.status["inputs"]["sandbox_profile_sha256"]:
                    self.status.update(status="failed", stage="sandbox_profile", reason="sandbox profile changed during probe")
            except OSError:
                self.status.update(status="failed", stage="sandbox_profile", reason="sandbox profile unavailable after probe")
        (self.output / "status.json").write_text(json.dumps(self.status, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", required=True, type=Path)
    parser.add_argument("--fixture", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--sandbox-profile", required=True, type=Path,
                        help="externally verified deny-network sandbox profile for the app")
    parser.add_argument("--allow-unavailable-bundle-id", action="store_true",
                        help="synthetic hosted probe only: permit unavailable bundle ID with verified owned PID/executable/name")
    parser.add_argument("--timeout-seconds", type=int, default=DEFAULT_SECONDS)
    args = parser.parse_args(argv)
    if not 15 <= args.timeout_seconds <= 300:
        print("native-ui: invalid timeout", file=sys.stderr)
        return 3
    try:
        args.output.mkdir(mode=0o700, parents=False, exist_ok=False)
    except OSError:
        print("native-ui: cannot create a new evidence directory", file=sys.stderr)
        return 3
    try:
        probe = Probe(args.app, args.fixture, args.output, args.sandbox_profile, args.timeout_seconds,
                      allow_unavailable_bundle_id=args.allow_unavailable_bundle_id)
    except OSError:
        (args.output / "status.json").write_text('{"schema_version":1,"status":"failed","stage":"preflight","reason":"input path unavailable"}\n', encoding="utf-8")
        print("native-ui: failed")
        return 3
    exit_code = 2
    try:
        probe.run()
        exit_code = 0
    except Blocked as error:
        probe.status.update(status="blocked", stage=probe.stage, reason=str(error))
        probe.screenshot(f"blocked-{probe.stage}", diagnostic=True)
        probe.diagnostic()
    except (Failed, OSError, ValueError, TypeError) as error:
        probe.status.update(status="failed", stage=probe.stage, reason=str(error))
        exit_code = 3
    except Exception as error:
        probe.status.update(status="failed", stage=probe.stage, reason=f"unexpected error: {type(error).__name__}")
        exit_code = 3
    finally:
        probe.finish()
    print(f"native-ui: {probe.status['status']}")
    return 3 if probe.status["status"] == "failed" else exit_code


if __name__ == "__main__":
    raise SystemExit(main())
