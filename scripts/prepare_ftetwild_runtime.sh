#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${ECKY_FTETWILD_RUNTIME_DIR:-$PROJECT_ROOT/.dist/runtime/ftetwild}"
BUILD_ROOT="${ECKY_FTETWILD_BUILD_ROOT:-$PROJECT_ROOT/.dist/build/ftetwild}"
SOURCE_OVERRIDE="${ECKY_FTETWILD_SOURCE_ROOT:-}"
REUSE_BUILD_DIR="${ECKY_FTETWILD_REUSE_BUILD_DIR:-}"
SOURCE_DIR="$BUILD_ROOT/source"
BUILD_DIR="$BUILD_ROOT/cmake"

FTETWILD_VERSION="0.1.0-ecky.1"
FTETWILD_COMMIT="d7d99bb4387a07895b9adce058dc7305f6b6e5ab"
FTETWILD_REPOSITORY="https://github.com/wildmeshing/fTetWild.git"
PATCH_FILE="$PROJECT_ROOT/src-tauri/native/patches/ftetwild-ecky-worker.patch"
WORKER_SOURCE="$PROJECT_ROOT/src-tauri/native/ftetwild_worker.cpp"
LEGAL_SOURCE="$PROJECT_ROOT/src-tauri/native/ftetwild-legal"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command missing: $1" >&2
    exit 1
  fi
}

for command_name in cmake git shasum; do
  require_command "$command_name"
done
PYTHON_BIN="${ECKY_FTETWILD_PYTHON:-python3}"
if ! "$PYTHON_BIN" -c 'import sys' >/dev/null 2>&1; then
  if [[ -x /usr/bin/python3 ]] && /usr/bin/python3 -c 'import sys' >/dev/null 2>&1; then
    PYTHON_BIN=/usr/bin/python3
  else
    echo "Working Python 3 interpreter missing; set ECKY_FTETWILD_PYTHON." >&2
    exit 1
  fi
fi

case "$(uname -s)" in
  Darwin)
    RUNTIME_PLATFORM="macos"
    RUNTIME_ARCH="$(uname -m)"
    [[ "$RUNTIME_ARCH" == "arm64" ]] && RUNTIME_ARCH="aarch64"
    ;;
  Linux)
    RUNTIME_PLATFORM="linux"
    RUNTIME_ARCH="$(uname -m)"
    [[ "$RUNTIME_ARCH" == "arm64" ]] && RUNTIME_ARCH="aarch64"
    require_command patchelf
    ;;
  *)
    echo "Unsupported fTetWild runtime platform: $(uname -s)" >&2
    exit 1
    ;;
esac

if [[ -n "$REUSE_BUILD_DIR" ]]; then
  if [[ -z "$SOURCE_OVERRIDE" ]]; then
    echo "ECKY_FTETWILD_REUSE_BUILD_DIR requires ECKY_FTETWILD_SOURCE_ROOT." >&2
    exit 1
  fi
  SOURCE_DIR="$SOURCE_OVERRIDE"
  BUILD_DIR="$REUSE_BUILD_DIR"
else
  rm -rf "$BUILD_ROOT"
  mkdir -p "$BUILD_ROOT"
  if [[ -n "$SOURCE_OVERRIDE" ]]; then
    git clone --no-hardlinks "$SOURCE_OVERRIDE" "$SOURCE_DIR"
  else
    git clone --filter=blob:none "$FTETWILD_REPOSITORY" "$SOURCE_DIR"
  fi
  git -C "$SOURCE_DIR" checkout --detach "$FTETWILD_COMMIT"
  mkdir -p "$SOURCE_DIR/ecky"
  cp "$WORKER_SOURCE" "$SOURCE_DIR/ecky/ftetwild_worker.cpp"
  git -C "$SOURCE_DIR" apply "$PATCH_FILE"

  cmake -S "$SOURCE_DIR" -B "$BUILD_DIR" -G "Unix Makefiles" \
    -DCMAKE_BUILD_TYPE=Release \
    -DFLOAT_TETWILD_ENABLE_TBB=OFF \
    -DFLOAT_TETWILD_WITH_EXACT_ENVELOPE=OFF \
    -DFLOAT_TETWILD_WITH_SANITIZERS=OFF \
    -DLIBIGL_WITH_TETGEN=OFF \
    -DLIBIGL_WITH_TRIANGLE=OFF
  cmake --build "$BUILD_DIR" --target ecky_ftetwild_worker --parallel
fi

observed_revision="$(git -C "$SOURCE_DIR" rev-parse HEAD)"
if [[ "$observed_revision" != "$FTETWILD_COMMIT" ]]; then
  echo "fTetWild source revision mismatch: expected $FTETWILD_COMMIT, got $observed_revision" >&2
  exit 1
fi
if ! git -C "$SOURCE_DIR" apply --reverse --check "$PATCH_FILE"; then
  echo "Pinned fTetWild source does not contain the published Ecky patch." >&2
  exit 1
fi
if ! cmp -s "$WORKER_SOURCE" "$SOURCE_DIR/ecky/ftetwild_worker.cpp"; then
  echo "Built fTetWild worker source differs from repository adapter source." >&2
  exit 1
fi

WORKER_BINARY="$BUILD_DIR/ecky_ftetwild_worker"
if [[ ! -x "$WORKER_BINARY" ]]; then
  echo "fTetWild worker binary missing after build: $WORKER_BINARY" >&2
  exit 1
fi
if ! grep -q '^FLOAT_TETWILD_ENABLE_TBB:BOOL=OFF$' "$BUILD_DIR/CMakeCache.txt" \
  || ! grep -q '^LIBIGL_WITH_TETGEN:BOOL=OFF$' "$BUILD_DIR/CMakeCache.txt"; then
  echo "fTetWild build enabled forbidden nondeterministic or TetGen capability." >&2
  exit 1
fi

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/bin" "$OUTPUT_DIR/lib" "$OUTPUT_DIR/legal" "$OUTPUT_DIR/source"
cp "$WORKER_BINARY" "$OUTPUT_DIR/bin/ftetwild-worker"
chmod 755 "$OUTPUT_DIR/bin/ftetwild-worker"

if [[ "$RUNTIME_PLATFORM" == "macos" ]]; then
  require_command install_name_tool
  require_command otool
  gmp_dependency="$(otool -L "$WORKER_BINARY" | tail -n +2 | awk '{print $1}' | grep '/libgmp\..*\.dylib$' | head -n 1)"
  if [[ -z "$gmp_dependency" || ! -f "$gmp_dependency" ]]; then
    echo "Pinned worker has no resolvable dynamic GMP dependency." >&2
    exit 1
  fi
  gmp_library_name="$(basename "$gmp_dependency")"
  cp -L "$gmp_dependency" "$OUTPUT_DIR/lib/$gmp_library_name"
  chmod u+w "$OUTPUT_DIR/bin/ftetwild-worker" "$OUTPUT_DIR/lib/$gmp_library_name"
  install_name_tool -id "@rpath/$gmp_library_name" "$OUTPUT_DIR/lib/$gmp_library_name"
  install_name_tool -change "$gmp_dependency" "@rpath/$gmp_library_name" "$OUTPUT_DIR/bin/ftetwild-worker"
  install_name_tool -add_rpath "@executable_path/../lib" "$OUTPUT_DIR/bin/ftetwild-worker"
  codesign --force --sign - "$OUTPUT_DIR/lib/$gmp_library_name" >/dev/null
  codesign --force --sign - "$OUTPUT_DIR/bin/ftetwild-worker" >/dev/null
  linked_report="$(otool -L "$OUTPUT_DIR/bin/ftetwild-worker")"
else
  require_command ldd
  gmp_dependency="$(ldd "$WORKER_BINARY" | awk '/libgmp\.so/{print $3; exit}')"
  if [[ -z "$gmp_dependency" || ! -f "$gmp_dependency" ]]; then
    echo "Pinned worker has no resolvable dynamic GMP dependency." >&2
    exit 1
  fi
  gmp_library_name="$(basename "$gmp_dependency")"
  cp -L "$gmp_dependency" "$OUTPUT_DIR/lib/$gmp_library_name"
  patchelf --set-rpath '$ORIGIN/../lib' "$OUTPUT_DIR/bin/ftetwild-worker"
  linked_report="$(ldd "$OUTPUT_DIR/bin/ftetwild-worker")"
fi

if grep -Eiq 'tetgen|gmsh|freecad|python|calculix|tbb' <<<"$linked_report"; then
  echo "fTetWild worker links a forbidden fallback/runtime dependency:" >&2
  echo "$linked_report" >&2
  exit 1
fi

cp "$SOURCE_DIR/LICENSE.MPL2" "$OUTPUT_DIR/legal/LICENSE.MPL-2.0"
cp "$LEGAL_SOURCE/NOTICE.txt" "$OUTPUT_DIR/legal/NOTICE.txt"
cp "$LEGAL_SOURCE/transitive-licenses.json" "$OUTPUT_DIR/legal/transitive-licenses.json"

DEPS_DIR="$BUILD_DIR/_deps"
for required_source in libigl eigen geogram fmt spdlog json predicates; do
  if [[ ! -d "$DEPS_DIR/${required_source}-src" ]]; then
    echo "Corresponding source missing for linked dependency: $required_source" >&2
    exit 1
  fi
done

cp "$DEPS_DIR/libigl-src/LICENSE.MPL2" "$OUTPUT_DIR/legal/LICENSE.libigl.MPL-2.0"
cp "$DEPS_DIR/eigen-src/COPYING.MPL2" "$OUTPUT_DIR/legal/LICENSE.Eigen.MPL-2.0"
cp "$DEPS_DIR/geogram-src/LICENSE" "$OUTPUT_DIR/legal/LICENSE.Geogram.BSD-3-Clause"
cp "$DEPS_DIR/fmt-src/LICENSE" "$OUTPUT_DIR/legal/LICENSE.fmt.MIT"
cp "$DEPS_DIR/spdlog-src/LICENSE" "$OUTPUT_DIR/legal/LICENSE.spdlog.MIT"
cp "$DEPS_DIR/json-src/LICENSE.MIT" "$OUTPUT_DIR/legal/LICENSE.nlohmann-json.MIT"
cp "$DEPS_DIR/predicates-src/README.md" "$OUTPUT_DIR/legal/LICENSE.libigl-predicates.Public-Domain"

if [[ "$RUNTIME_PLATFORM" == "macos" ]]; then
  gmp_prefix="$(cd "$(dirname "$gmp_dependency")/.." && pwd)"
  cp "$gmp_prefix/COPYING.LESSERv3" "$OUTPUT_DIR/legal/LICENSE.GMP.LGPL-3.0" 2>/dev/null \
    || cp "$(brew --prefix gmp)/COPYING.LESSERv3" "$OUTPUT_DIR/legal/LICENSE.GMP.LGPL-3.0"
else
  if [[ -n "${ECKY_GMP_LICENSE_FILE:-}" && -f "$ECKY_GMP_LICENSE_FILE" ]]; then
    cp "$ECKY_GMP_LICENSE_FILE" "$OUTPUT_DIR/legal/LICENSE.GMP.LGPL-3.0"
  else
    echo "Linux packaging requires ECKY_GMP_LICENSE_FILE for the bundled GMP library." >&2
    exit 1
  fi
fi

"$PYTHON_BIN" - "$OUTPUT_DIR/source/ftetwild-corresponding-source.tar.gz" "$SOURCE_DIR" "$DEPS_DIR" "$PATCH_FILE" "$WORKER_SOURCE" <<'PY'
import gzip
import os
import sys
import tarfile
from pathlib import Path

archive_path = Path(sys.argv[1])
source_root = Path(sys.argv[2])
deps_root = Path(sys.argv[3])
patch_path = Path(sys.argv[4])
worker_path = Path(sys.argv[5])
sources = [
    (source_root, "fTetWild"),
    *[(deps_root / f"{name}-src", f"dependencies/{name}")
      for name in ("libigl", "eigen", "geogram", "fmt", "spdlog", "json", "predicates")],
]

def normalized(info: tarfile.TarInfo) -> tarfile.TarInfo:
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    info.mtime = 0
    return info

def excluded(path: Path) -> bool:
    return any(part == ".git" or part.startswith("build") for part in path.parts)

with archive_path.open("wb") as raw:
    with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as zipped:
        with tarfile.open(fileobj=zipped, mode="w") as archive:
            for root, prefix in sources:
                for path in sorted(root.rglob("*")):
                    relative = path.relative_to(root)
                    if excluded(relative):
                        continue
                    archive.add(path, arcname=str(Path(prefix) / relative), recursive=False, filter=normalized)
            archive.add(patch_path, arcname="ecky/ftetwild-ecky-worker.patch", filter=normalized)
            archive.add(worker_path, arcname="ecky/ftetwild_worker.cpp", filter=normalized)
PY

"$PYTHON_BIN" - "$OUTPUT_DIR" "$FTETWILD_VERSION" "$FTETWILD_COMMIT" "$RUNTIME_PLATFORM" "$RUNTIME_ARCH" "$gmp_library_name" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])

def entry(relative: str) -> dict[str, str]:
    data = (root / relative).read_bytes()
    return {"path": relative, "sha256": "sha256:" + hashlib.sha256(data).hexdigest()}

manifest = {
    "schemaVersion": 1,
    "runtimeName": "fTetWild",
    "runtimeVersion": sys.argv[2],
    "sourceRevision": sys.argv[3],
    "platform": sys.argv[4],
    "arch": sys.argv[5],
    "workerProtocol": "ecky-ftetwild-worker-v1",
    "executable": entry("bin/ftetwild-worker"),
    "runtimeLibraries": [entry("lib/" + sys.argv[6])],
    "sourceArchive": entry("source/ftetwild-corresponding-source.tar.gz"),
    "license": entry("legal/LICENSE.MPL-2.0"),
    "notice": entry("legal/NOTICE.txt"),
    "transitiveLicenseInventory": entry("legal/transitive-licenses.json"),
    "capabilities": {
        "structuredArrays": True,
        "tet4": True,
        "wideSurfaceTags": True,
        "isolatedWorker": True,
    },
}
(root / "runtime-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
PY

chmod -R u+rwX,go+rX "$OUTPUT_DIR"
echo "Prepared fTetWild runtime: $OUTPUT_DIR"
