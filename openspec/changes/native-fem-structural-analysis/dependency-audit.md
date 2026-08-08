# Native FEM Dependency Audit

Audit date: 2026-08-09. Default distributed path remains native-only and
offline. Every shipped artifact must pass `fem_mesher::probe_ftetwild_runtime`;
an entry in this document is not packaging evidence.

## Locked decisions

| Component | Exact selection | License | Platform / maintenance evidence | Decision |
|---|---|---|---|---|
| fTetWild | Git commit `d7d99bb4387a07895b9adce058dc7305f6b6e5ab` | MPL-2.0 | Upstream documents macOS, Linux, Windows. Selected commit dated 2025-11-13. | Bundle patched isolated worker plus corresponding source archive and notices. No ambient executable lookup. |
| libigl | `v2.6.0`, fixed by selected fTetWild CMake | MPL-2.0 for selected core/predicates path | Active upstream; cross-platform CMake. Copyleft optional modules stay disabled. | Bundle only core and predicates requested by fTetWild. TetGen and other copyleft optional modules disabled. |
| geogram | `v1.9.6`, fixed by selected fTetWild CMake | BSD-3-Clause | Selected fTetWild config names Darwin clang, Linux GCC, Windows MSVC paths. | Static library allowed with notice. Graphics, Lua, exploragram, legacy numerics, and Triangle disabled. |
| GMP | `6.3.0` | LGPL-3.0-or-later or GPL-2.0-or-later | Mature upstream. fTetWild CMake leaves version unconstrained; Ecky packaging must not. | Dynamically package exact 6.3.0-compatible library and LGPL source/notice/relink evidence. Static GMP is rejected until relink obligations have an approved packaging implementation. Windows MPIR substitution needs a separate exact audit and manifest. |
| oneTBB | `v2022.2.0`, fixed by selected fTetWild CMake | Apache-2.0 | Active Intel oneAPI project; macOS/Linux/Windows. | Build static TBB only if deterministic single-thread worker tests prove no hidden parallelism; otherwise build fTetWild with TBB disabled. MVP worker protocol requires `threadCount = 1`. |
| Fenris | crate `=0.0.33` | MIT OR Apache-2.0 | Experimental `0.0.x` API, active repository. | Audited candidate, not linked in MVP. Current constant-strain Tet4 kernel is smaller and has independent patch/oracle tests. Adding Fenris requires adapter differential tests first. |
| Faer | crate `=0.24.4`, `default-features = false`, features `std,sparse-linalg` | MIT | Rust 1.84 minimum; active release. | Selected production sparse LLT backend. Rayon/default extras disabled. Public contracts expose no Faer types. |
| mshio | crate `=0.4.2` | MIT | Maintained MSH 4.1 parser. | Not linked. Custom worker protocol returns typed structured arrays; no Gmsh/MSH handoff in production path. May be used only by explicit reference-fixture tooling. |
| vtkio | crate `=0.7.0-rc2` | MIT OR Apache-2.0 | Release candidate; legacy/XML VTK support. | Not linked for solver. Re-audit before optional VTU export; result sidecars remain Ecky-owned typed arrays. |

The selected fTetWild CMake also pins CLI11 `v2.5.0`, fmt `11.2.0`, spdlog
`v1.15.3`, and `jdumas/json` commit
`0901d33bf6e7dfe6f70fd9d142c8f5c6695c6c5b`. These must appear in the shipped
transitive inventory with exact source and license digests. Fast Envelope is
disabled for MVP. Sanitizer build dependencies are not distributed.

## Source evidence

- fTetWild repository and supported platforms: <https://github.com/wildmeshing/fTetWild>
- selected fTetWild dependency pins: <https://github.com/wildmeshing/fTetWild/blob/d7d99bb4387a07895b9adce058dc7305f6b6e5ab/cmake/FloatTetwildDependencies.cmake>
- fTetWild MPL text: <https://github.com/wildmeshing/fTetWild/blob/d7d99bb4387a07895b9adce058dc7305f6b6e5ab/LICENSE.MPL2>
- libigl license: <https://github.com/libigl/libigl/blob/v2.6.0/LICENSE.MPL2>
- geogram license: <https://github.com/BrunoLevy/geogram/blob/v1.9.6/LICENSE>
- oneTBB license: <https://github.com/oneapi-src/oneTBB/blob/v2022.2.0/LICENSE.txt>
- GMP license guidance: <https://www.gnu.org/licenses/gpl-faq.html#LGPLStaticVsDynamic>
- Fenris crate metadata: <https://crates.io/crates/fenris/0.0.33>
- Faer crate metadata: <https://crates.io/crates/faer/0.24.4>
- mshio crate metadata: <https://crates.io/crates/mshio/0.4.2>
- vtkio crate metadata: <https://crates.io/crates/vtkio/0.7.0-rc2>

## Packaging hard stop

Release packaging must provide exact files named by `runtime-manifest.json`:
worker executable, selected source archive, MPL license, notices, and non-empty
transitive inventory. SHA-256, OS, architecture, protocol, capability, source
revision, and runtime version must all match. Missing or mismatched evidence
blocks meshing. No TetGen, Gmsh, FreeCAD, Python, remote, or STL fallback exists.
