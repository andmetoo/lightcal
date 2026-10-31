"""Prepare pinned native design fonts; never execute downloaded package code."""

import base64
import hashlib
import io
import json
from pathlib import Path
import tarfile
from urllib.request import Request, urlopen
import zipfile

ROOT = Path(__file__).resolve().parents[2]
DESTINATION = ROOT / "design" / "fonts" / "onest"
PACKAGE_URL = "https://registry.npmjs.org/@fontsource/onest/-/onest-5.3.1.tgz"
PACKAGE_INTEGRITY = "sha512-2uA9HUzQHBB8H7T2R6inmrnqPofB6+svATHM/huIG640duJr4rm+uanmAIheTTlbZvRC0ujMtf2T4U4RX51oEQ=="
RELEASE_URL = "https://github.com/simpals/onest/releases/download/2.001/onest-2.001.zip"
RELEASE_SHA256 = "17b59e19c349e603b7d113a596b6d8e08427e97a7b5235668b69e9d8a06a4267"


def download(url):
    request = Request(url, headers={"User-Agent": "Lightcal-design-font-vendor"})
    with urlopen(request, timeout=60) as response:
        return response.read()


def main():
    package = download(PACKAGE_URL)
    integrity = "sha512-" + base64.b64encode(hashlib.sha512(package).digest()).decode()
    if integrity != PACKAGE_INTEGRITY:
        raise ValueError("Fontsource package integrity mismatch")
    with tarfile.open(fileobj=io.BytesIO(package), mode="r:gz") as archive:
        metadata = json.load(archive.extractfile("package/metadata.json"))
        if metadata["family"] != "Onest":
            raise ValueError("Unexpected font family")
        # Keep subset files only in ignored scratch storage for glyph validation.
        scratch = ROOT / "target" / "onest-proof"
        scratch.mkdir(parents=True, exist_ok=True)
        for weight in (400, 500, 600, 700):
            for subset in ("latin", "cyrillic"):
                name = f"onest-{subset}-{weight}-normal.woff"
                (scratch / name).write_bytes(archive.extractfile("package/files/" + name).read())
    release = download(RELEASE_URL)
    if hashlib.sha256(release).hexdigest() != RELEASE_SHA256:
        raise ValueError("Upstream release integrity mismatch")
    DESTINATION.mkdir(parents=True, exist_ok=True)
    hashes = {}
    with zipfile.ZipFile(io.BytesIO(release)) as archive:
        selected = {"OFL.txt": "OFL.txt"}
        for style in ("Regular", "Medium", "SemiBold", "Bold"):
            selected[f"fonts/ttf/Onest-{style}.ttf"] = f"Onest-{style}.ttf"
        # Explicit member reads avoid extracting untrusted archive paths.
        for source, filename in selected.items():
            data = archive.read("onest-2.001/" + source)
            (DESTINATION / filename).write_bytes(data)
            hashes[filename] = hashlib.sha256(data).hexdigest()
    manifest = {
        "family": "Onest",
        "selection": "Fontsource 5.3.x, pinned to 5.3.1",
        "fontsource": {"package": "@fontsource/onest", "version": "5.3.1", "url": PACKAGE_URL, "integrity": PACKAGE_INTEGRITY, "metadata": metadata},
        "nativeSource": {"version": "2.001", "url": RELEASE_URL, "sha256": RELEASE_SHA256, "sourceCommit": "8739b1910618a15335e4cc48842052d0ee739ade"},
        "filesSha256": hashes,
        "penpot": {"family": "Onest", "fontId": "gfont-onest", "binaryRevision": "not exposed by plugin API"},
    }
    (DESTINATION / "source.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Prepared {len(hashes)} verified font/license files and source.json in {DESTINATION}")


if __name__ == "__main__":
    main()
