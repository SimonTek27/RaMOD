#!/usr/bin/env python3
"""Restore src/fsb.rs from compressed recovery chunks in tools/fsb_recovery/."""
import base64
import zlib
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parts = sorted((root / "tools" / "fsb_recovery").glob("chunk_*.b64"))
if not parts:
    raise SystemExit("no recovery chunks found")
data = "".join(p.read_text().strip() for p in parts)
raw = zlib.decompress(base64.b64decode(data))
out = root / "src" / "fsb.rs"
out.write_bytes(raw)
print(f"wrote {out} ({len(raw)} bytes)")
