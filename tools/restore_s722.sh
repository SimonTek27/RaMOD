#!/usr/bin/env bash
# Restore runtime_core_b + audio_bridge after S7.22 partial push.
# Usage:
#   ARTIFACTS_DIR=/path/to/artifacts ./tools/restore_s722.sh
#   # or from repo root if recovery/*.part* already present:
#   ./tools/restore_s722.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ART="${ARTIFACTS_DIR:-}"

copy_one() {
  local src="$1" dest="$2"
  if [[ -f "$src" ]]; then
    mkdir -p "$(dirname "$dest")"
    cp "$src" "$dest"
    echo "ok  $dest ($(wc -c < "$dest") bytes)"
  else
    echo "miss $src"
    return 1
  fi
}

ok=0
if [[ -n "$ART" ]]; then
  copy_one "$ART/recovery/core_b.rs" "$ROOT/runtime/src/runtime_core_b.rs" && ok=1
  copy_one "$ART/recovery/ab.rs" "$ROOT/editor/src/audio_bridge.rs" && ok=1
  mkdir -p "$ROOT/recovery"
  cp -n "$ART/recovery/core_b.part"* "$ROOT/recovery/" 2>/dev/null || true
  cp -n "$ART/recovery/ab.part"* "$ROOT/recovery/" 2>/dev/null || true
fi

if [[ ! -f "$ROOT/runtime/src/runtime_core_b.rs" ]] || [[ $(wc -c < "$ROOT/runtime/src/runtime_core_b.rs") -lt 12000 ]]; then
  if [[ -f "$ROOT/recovery/core_b.part0" ]]; then
    cat "$ROOT/recovery/core_b.part"* > "$ROOT/runtime/src/runtime_core_b.rs"
    echo "ok  concatenated core_b ($(wc -c < "$ROOT/runtime/src/runtime_core_b.rs") bytes)"
    ok=1
  fi
fi
if [[ ! -f "$ROOT/editor/src/audio_bridge.rs" ]] || [[ $(wc -c < "$ROOT/editor/src/audio_bridge.rs") -lt 15000 ]]; then
  if [[ -f "$ROOT/recovery/ab.part0" ]]; then
    cat "$ROOT/recovery/ab.part"* > "$ROOT/editor/src/audio_bridge.rs"
    echo "ok  concatenated ab ($(wc -c < "$ROOT/editor/src/audio_bridge.rs") bytes)"
    ok=1
  fi
fi

if [[ "$ok" -eq 0 ]]; then
  echo "No restore sources found. Set ARTIFACTS_DIR to the project artifacts folder."
  exit 1
fi
echo "Done. Run: cargo build -p ramod_runtime -p ramod_editor"
