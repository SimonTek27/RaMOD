#!/usr/bin/env python3
"""Restore src/fsb.rs from tools/fsb_recovery/chunk_*.b64 (zlib + base64)."""
from __future__ import annotations

import base64
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHUNK_DIR = ROOT / "tools" / "fsb_recovery"
OUT = ROOT / "src" / "fsb.rs"


def main() -> int:
    parts = sorted(CHUNK_DIR.glob("chunk_*.b64"))
    if not parts:
        print("no chunks in", CHUNK_DIR, file=sys.stderr)
        return 1
    data = "".join(p.read_text().strip() for p in parts)
    try:
        raw = zlib.decompress(base64.b64decode(data))
    except Exception as e:
        print("decompress failed:", e, file=sys.stderr)
        print("Chunks may be incomplete — use:", file=sys.stderr)
        print("  git checkout d0e38a10e0ad438bbea79095292b3bbd0218c953 -- src/fsb.rs", file=sys.stderr)
        return 1
    text = raw.decode("utf-8")
    if "pub struct Fsb5" not in text:
        print("restored content looks invalid", file=sys.stderr)
        return 1
    OUT.write_bytes(raw)
    print(f"wrote {OUT} ({len(raw)} bytes) from {len(parts)} chunks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
