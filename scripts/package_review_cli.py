"""Build a standalone review archive for the host or a specified Rust target."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile


def output(*cmd):
    return subprocess.check_output(cmd, text=True, encoding="utf-8").strip()


def package(target, extra_notices):
    compiler = output("rustc", "-vV").splitlines()
    if target is None:
        target = next(line.removeprefix("host: ") for line in compiler if line.startswith("host: "))
    revision = output("git", "rev-parse", "HEAD")
    metadata = json.loads(output("cargo", "metadata", "--quiet", "--locked", "--format-version", "1", "--filter-platform", target))
    target_dir = Path(metadata["target_directory"])
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).resolve()
    roots = [(Path.home(), "home"), (Path.cwd(), "source"), (cargo_home, "cargo"), (target_dir, "build")]
    encoded = os.environ.get("CARGO_ENCODED_RUSTFLAGS")
    flags = encoded.split("\x1f") if encoded else os.environ.get("RUSTFLAGS", "").split()
    markers = set()
    for root, label in roots:
        if root == root.parent:
            continue
        for prefix in sorted({str(root), root.as_posix()}):
            flags.append(f"--remap-path-prefix={prefix}={label}")
            markers.update(prefix.encode(encoding).lower() for encoding in ("utf-8", "utf-16-le"))
    subprocess.run(["cargo", "build", "--quiet", "--release", "--locked", "-p", "dedup-cli", "--target", target],
                   env=dict(os.environ, CARGO_PROFILE_RELEASE_STRIP="symbols",
                            CARGO_ENCODED_RUSTFLAGS="\x1f".join(flags)), check=True)
    exe = "dedup.exe" if "-windows-" in target else "dedup"
    name = f"recorddedu-{target}-{revision[:12]}"
    destination = target_dir / "review-package" / (name + ".tar.gz")
    destination.parent.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp) / name
        folder.mkdir()
        binary = folder / exe
        shutil.copy2(target_dir / target / "release" / exe, binary)
        shutil.copyfile("README.md", folder / "README.md")
        shutil.copyfile("LICENSE", folder / "LICENSE")
        (folder / "BUILD.txt").write_text(f"{revision}\n{target}\n{compiler[0]}\n", encoding="utf-8")
        digest = hashlib.sha256(binary.read_bytes()).hexdigest()
        (folder / "SHA256SUMS").write_bytes(f"{digest}  {exe}\n".encode())

        notices = folder / "licenses"
        for crate in metadata["packages"]:
            if crate["source"] is None:
                continue
            source = Path(crate["manifest_path"]).parent
            licenses = [p for p in source.iterdir() if p.is_file() and
                        p.name.lower().startswith(("license", "notice", "copyright"))]
            if not licenses:
                raise ValueError(f"Missing license text for {crate['name']}")
            crate_dir = notices / f"{crate['name']}-{crate['version']}"
            crate_dir.mkdir(parents=True)
            for path in licenses:
                shutil.copyfile(path, crate_dir / path.name)
        rust_docs = Path(output("rustc", "--print", "sysroot")) / "share/doc/rust"
        shutil.copytree(rust_docs / "licenses", notices / "rust/licenses")
        shutil.copyfile(rust_docs / "COPYRIGHT-library.html", notices / "rust/COPYRIGHT-library.html")
        for path in extra_notices:
            shutil.copyfile(path, notices / path.name)

        for path in folder.rglob("*"):
            if path.is_file():
                contents = path.read_bytes().lower()
                if any(marker in contents for marker in markers):
                    raise ValueError("Package contains an absolute builder path")

        def archive_entry(entry):
            entry.uid = entry.gid = 0
            entry.uname = entry.gname = ""
            entry.mode = 0o755 if entry.isdir() or entry.name == f"{name}/{exe}" else 0o644
            return entry

        with tarfile.open(destination, "w:gz") as archive:
            archive.add(folder, arcname=name, filter=archive_entry)
    print(f"{hashlib.sha256(destination.read_bytes()).hexdigest()}  {destination.name}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", help="Rust target triple (defaults to the compiler host)")
    parser.add_argument("--notice", type=Path, action="append", default=[], help="Additional toolchain license file")
    args = parser.parse_args()
    package(args.target, args.notice)
