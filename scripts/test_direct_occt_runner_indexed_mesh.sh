#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNNER="$ROOT/.dist/runtime/occt/bin/direct-occt-runner"
WORK="$(mktemp -d)"

python3 - "$WORK/plan.json" <<'PY'
import json, sys

def arg(kind, value): return {"kind": kind, "value": value}
ref = lambda n: arg("ref", n)
vertices = [[0,0,0], [2,0,0], [2,2,0], [0,2,0], [0,0,2], [2,0,2], [2,2,2], [0,2,2]]
triangles = [[0,2,1],[0,3,2],[4,5,6],[4,6,7],[0,1,5],[0,5,4],[1,2,6],[1,6,5],[2,3,7],[2,7,6],[3,0,4],[3,4,7]]
mesh = [arg("list", [arg("point3", p) for p in vertices]), arg("list", [arg("list", [arg("number", i) for i in t]) for t in triangles]), arg("text", "sha256:fixture")]
commands = [
  {"output": 1, "op": "import-indexed-mesh", "args": mesh, "keywords": []},
  {"output": 2, "op": "solidify", "args": [ref(1)], "keywords": []},
  {"output": 3, "op": "cylinder", "args": [arg("number", .5), arg("number", 4)], "keywords": []},
  {"output": 4, "op": "difference", "args": [ref(2), ref(3)], "keywords": []},
]
json.dump({"schemaVersion":1,"planId":"indexed-mesh-fixture","parts":[{"key":"body","label":"body","root":4,"commands":commands}]}, open(sys.argv[1], "w"))
PY

"$RUNNER" --plan "$WORK/plan.json" --out "$WORK/out"
test -s "$WORK/out/preview.stl"
test ! -e "$WORK/out/model.step"
python3 - "$WORK/out/stage-report.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
assert [s["name"] for s in report["stages"]] == ["import","validate","solidify","boolean","cleanup","mesh","verify","export"]
by_name = {s["name"]: s for s in report["stages"]}
for name in ("validate", "solidify", "boolean", "mesh", "export"):
    assert by_name[name]["status"] == "executed", name
PY

echo "Indexed mesh native runner contract passed"
