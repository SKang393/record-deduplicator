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
import subprocess
import sys
import time

UI_SCRIPT = Path(__file__).with_name("ui.applescript")
PER_CALL_SECONDS = 6
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


class Probe:
    def __init__(self, app: Path, fixture: Path, output: Path, sandbox_profile: Path, seconds: int):
        self.app = app.resolve(strict=True)
        self.fixture = fixture.resolve(strict=True)
        self.output = output.resolve()
        self.sandbox_profile = sandbox_profile.resolve(strict=True)
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
            result = self.command(["/usr/bin/osascript", str(UI_SCRIPT), str(self.process.pid),
                                   self.bundle_id, operation, target, extra], "accessibility.log")
            reply = result.stdout.decode("utf-8", "replace").strip()
            lines = reply.splitlines()
            identity = {"stage": self.stage, "operation": operation,
                        "state": lines[0] if lines else "no_identity_response"}
            for line in lines[1:]:
                key, separator, value = line.partition("=")
                if separator and key in {"requested_pid", "observed_pid", "observed_name", "observed_bundle"}:
                    identity[key] = value[:160]
            with (self.output / "system-events-identity.jsonl").open("a", encoding="utf-8") as log:
                log.write(json.dumps(identity, ensure_ascii=True) + "\n")
            self.status["checks"]["system_events_identity"] = identity
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
        raise Blocked(f"{self.stage}: expected window, accessibility control, or native panel unavailable ({last_state})")

    def screenshot(self, name: str) -> None:
        path = self.output / f"{name}.png"
        try:
            result = self.command(["/usr/sbin/screencapture", "-x", str(path)], "screencapture.log", timeout_limit=2)
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
            self.command(["/usr/bin/osascript", str(UI_SCRIPT), str(self.process.pid),
                          self.bundle_id, "dump", "", ""], path.name, diagnostic=True)
        except (Blocked, OSError):
            if not path.exists():
                path.write_text("AX diagnostic unavailable or timed out.\n", encoding="utf-8")

    def check_launched_pid(self, executable: Path) -> None:
        """Require sandbox-exec to become the expected app at the owned PID."""
        try:
            libproc = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
            pidpath = libproc.proc_pidpath
            pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
            pidpath.restype = ctypes.c_int
        except (OSError, AttributeError) as error:
            raise Blocked("cannot inspect the launched process executable") from error
        expected = executable.resolve(strict=True)
        stage_deadline = min(self.deadline - DIAGNOSTIC_RESERVE_SECONDS, time.monotonic() + STAGE_SECONDS)
        while time.monotonic() < stage_deadline:
            if self.process.poll() is not None:
                raise Failed("sandboxed app exited before its executable could be verified")
            buffer = ctypes.create_string_buffer(4096)
            if pidpath(self.process.pid, buffer, len(buffer)) > 0:
                try:
                    if Path(os.fsdecode(buffer.value)).resolve(strict=True) == expected:
                        if not self.status["checks"].get("ps_identity_recorded"):
                            ps = self.command(["/bin/ps", "-p", str(self.process.pid), "-o", "pid=", "-o", "comm="],
                                              "process-identity.log")
                            fields = ps.stdout.decode("utf-8", "replace").strip().split(maxsplit=1)
                            if ps.returncode != 0 or len(fields) != 2 or fields[0] != str(self.process.pid):
                                raise Blocked("ps did not confirm the owned app PID")
                            self.status["checks"]["ps_identity_recorded"] = True
                        self.status["checks"]["owned_pid_executable_matches"] = True
                        return
                except OSError:
                    pass
            time.sleep(min(0.2, self.remaining()))
        raise Blocked("sandbox-exec PID did not become the expected app executable")

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
            source_bytes = self.fixture.read_bytes()
            if not source_bytes or len(source_bytes) > 1024 * 1024:
                raise ValueError("synthetic fixture must be nonempty and at most 1 MiB")
            profile_bytes = self.sandbox_profile.read_bytes()
            if not re.search(rb"^\s*\(deny network\*\)\s*$", profile_bytes, re.MULTILINE):
                raise ValueError("sandbox profile lacks the expected deny-network rule")
        except (OSError, KeyError, TypeError, ValueError, plistlib.InvalidFileException) as error:
            raise Failed(f"invalid app bundle or fixture: {error}") from error
        self.status["inputs"] = {"bundle_id": self.bundle_id, "executable_sha256": sha256(executable.read_bytes()),
                                 "fixture_sha256": sha256(source_bytes), "sandbox_profile_sha256": sha256(profile_bytes)}
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
        self.status.update(status="passed", stage="complete", reason="synthetic native open, import, save, and export verified")

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
    parser.add_argument("--timeout-seconds", type=int, default=DEFAULT_SECONDS)
    args = parser.parse_args(argv)
    if not 15 <= args.timeout_seconds <= 180:
        print("native-ui: invalid timeout", file=sys.stderr)
        return 3
    try:
        args.output.mkdir(mode=0o700, parents=False, exist_ok=False)
    except OSError:
        print("native-ui: cannot create a new evidence directory", file=sys.stderr)
        return 3
    try:
        probe = Probe(args.app, args.fixture, args.output, args.sandbox_profile, args.timeout_seconds)
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
