# Native FEM Dependency Audit

Audit date: 2026-08-29. Default path uses separately installed external
meshing runtimes; no mesher binary is linked or bundled by the product. Every
runtime must pass its probe before a mesh can be admitted. An entry here is not
itself packaging or license evidence.

## Locked decisions

| Component | Exact selection | License | Platform / maintenance evidence | Decision |
|---|---|---|---|---|
| Gmsh HXT | Version and executable digest captured by runtime probe | GPL-2.0-or-later | External Gmsh exposes OCC exact-BRep and HXT volume meshing on supported host platforms | Primary Tet4 mesher. Resolve `ECKY_GMSH_EXECUTABLE` or `gmsh` on PATH; never link or bundle by default. |
| Netgen OCC | Python/module version and digests captured by runtime probe | LGPL-2.1-or-later | External Netgen OCC Python module provides exact-BRep fallback on supported host platforms | Optional fallback after HXT failure, within remaining budget. Resolve `ECKY_NETGEN_PYTHON` or the `netgen` launcher; never silently substitute another runtime. |
| Fenris | crate `=0.0.33` | MIT OR Apache-2.0 | Experimental `0.0.x` API, active repository | Audited candidate, not linked in MVP. Current constant-strain Tet4 kernel has independent patch/oracle tests. |
| Faer | crate `=0.24.4`, `default-features = false`, features `std,sparse-linalg` | MIT | Rust 1.84 minimum; active release | Selected production sparse LLT backend. Public contracts expose no Faer types. |
| mshio | crate `=0.4.2` | MIT | Maintained MSH 4.1 parser | Not linked. Adapter parses bounded ASCII MSH2 directly; may be used only by explicit reference-fixture tooling. |
| vtkio | crate `=0.7.0-rc2` | MIT OR Apache-2.0 | Release candidate; legacy/XML VTK support | Not linked for solver. Re-audit before optional VTU interoperability; result sidecars remain Ecky-owned typed arrays. |

Gmsh HXT receives exact STEP input from the Direct OCCT boundary and emits
bounded ASCII MSH2. The adapter records executable path, version, SHA-256,
platform, architecture, protocol, controls, source/STEP digest, and exact OCC
face signatures. Netgen receives the same exact STEP and records interpreter,
module path, version, interpreter/module SHA-256, adapter script digest, and
runtime identity. Both outputs pass Tet4, budget, boundary ownership, and
durable face-group reconciliation before atomic publication.

## Source evidence

- Gmsh documentation and license: <https://gmsh.info/doc/texinfo/gmsh.html>
- Gmsh license: <https://gmsh.info/#License>
- Netgen/NGSolve downloads: <https://ngsolve.org/downloads>
- Fenris crate metadata: <https://crates.io/crates/fenris/0.0.33>
- Faer crate metadata: <https://crates.io/crates/faer/0.24.4>
- mshio crate metadata: <https://crates.io/crates/mshio/0.4.2>
- vtkio crate metadata: <https://crates.io/crates/vtkio/0.7.0-rc2>

## Runtime/license gate

Release and CI evidence must record the exact external executable/interpreter,
version, platform, architecture, SHA-256, adapter protocol, and applicable
license evidence. Missing or mismatched probe data blocks meshing. No TetGen,
FreeCAD, CalculiX, remote, or untagged STL fallback exists. Gmsh's GPL terms
must be reviewed for the deployment environment; the default product does not
redistribute or link Gmsh.
