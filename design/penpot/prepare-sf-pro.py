"""Download Apple's design font package into ignored scratch storage only."""

import hashlib
import gzip
from pathlib import Path
import subprocess
from urllib.request import Request, urlopen

root = Path(__file__).resolve().parents[2]
destination = root / "target" / "sf-pro-proof" / "SF-Pro.dmg"
url = "https://devimages-cdn.apple.com/design/resources/download/SF-Pro.dmg"
expected_sha256 = "968ab3b8b1f92c2d8af61eb0680f5c2224c4e78d59b986bf004502a7fc012918"
if destination.exists():
    data = destination.read_bytes()
else:
    request = Request(url, headers={"User-Agent": "Lightcal-macOS-design"})
    with urlopen(request, timeout=60) as response:
        data = response.read()
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(data)
print(f"Official Apple package: {len(data)} bytes at {destination}")
print("SHA256 " + hashlib.sha256(data).hexdigest())
if hashlib.sha256(data).hexdigest() != expected_sha256:
    raise ValueError("Apple package changed; review its provenance before updating the recorded hash")

seven_zip = Path("C:/Program Files/7-Zip/7z.exe")
if not seven_zip.exists():
    raise RuntimeError("7-Zip is required to prepare this Windows-hosted design package")
package_directory = destination.parent / "package"
payload_directory = destination.parent / "payload"
subprocess.run([str(seven_zip), "e", str(destination), "SFProFonts.pkg", "-o" + str(package_directory), "-y", "-bso0", "-bsp0"], check=True)
subprocess.run([str(seven_zip), "e", str(package_directory / "SFProFonts.pkg"), "SFProFontsPackage.pkg/Payload", "-o" + str(payload_directory), "-y", "-bso0", "-bsp0"], check=True)

upload_directory = destination.parent / "upload"
upload_directory.mkdir(parents=True, exist_ok=True)
selected = {f"SF-Pro-{family}-{style}.otf" for family in ("Text", "Display") for style in ("Regular", "Medium", "Semibold", "Bold")}
found = set()
with gzip.open(payload_directory / "Payload", "rb") as archive:
    while True:
        header = archive.read(76)
        if not header:
            break
        if header[:6] != b"070707":
            raise ValueError("Unexpected Apple payload CPIO format")
        name_size, size = int(header[59:65], 8), int(header[65:76], 8)
        name = archive.read(name_size).rstrip(b"\0").decode()
        if name == "TRAILER!!!":
            break
        filename = name.rsplit("/", 1)[-1]
        if filename in selected:
            # Flat whitelist destinations; never execute an Apple installer.
            (upload_directory / filename).write_bytes(archive.read(size))
            found.add(filename)
        else:
            archive.seek(size, 1)
if found != selected:
    raise ValueError("Expected SF Pro Text/Display design faces are missing")
print(f"Prepared {len(found)} unmodified design faces in {upload_directory}")
