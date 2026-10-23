# vyges-ortools-lib

OR-Tools' linear solver with the **SCIP** backend, built with **Bazel** at pinned versions into one
static archive (`libvyges_ortools.a`), behind a small C shim and a safe Rust API.

> Part of Vyges Loom. Used where an engine solves small integer programs and must return the same
> optimum, tie included, on every run and every machine.

## Why pinned, and why the compiler is part of the pin

When an integer program has several optimal solutions, which one comes back is decided by the
solver's internals: its presolve, branching and LP pivots. A different OR-Tools or SCIP release can
return a different, equally optimal answer, and so can a different compiler or floating-point flag.
So this crate pins all of them:

- **OR-Tools 9.15** and **SCIP 9.2.3** from the Bazel Central Registry (`MODULE.bazel`);
- the **hermetic LLVM** toolchain (Bazel module `llvm`), so the compiler is not the host's;
- `-c opt -O3 -ffp-contract=off` (`.bazelrc`): no fused multiply-add, so each floating-point
  operation rounds as IEEE 754 says, however the optimiser inlines.

`ortools-pin.yaml` records the set. Bump on purpose, never to "latest".

## One call per method, in the caller's order

The C shim (`shim/vyges_mp.h`) and the Rust `Solver` make exactly one `MPSolver` call per function,
with the arguments given. Variables and rows are the indices the solver assigned, in creation
order; names are empty. Nothing is sorted, batched or deduplicated: when optima tie, creation order
can decide the answer, so that order belongs to the caller.

```rust
use vyges_ortools_lib::{ResultStatus, Solver};

let mut s = Solver::create("SCIP").unwrap();
let x = s.make_int_var(0.0, 1.0)?;
let y = s.make_int_var(0.0, 1.0)?;
let r = s.make_row_constraint(-Solver::infinity(), 1.0)?;
s.set_coefficient(r, x, 1.0)?;
s.set_coefficient(r, y, 1.0)?;
s.objective_set_coefficient(x, 2.0)?;
s.objective_set_coefficient(y, 1.0)?;
s.objective_set_maximization();
assert_eq!(s.solve(), ResultStatus::Optimal);
```

## Two ways to get the archive

- **Prebuilt:** `scripts/fetch-bundle.sh <dir>` downloads the bundle built from this checkout's
  build inputs (release `bundle-<id>`, id from `scripts/bundle-id.sh`) and checks its SHA-256;
  then set `VYGES_ORTOOLS_PREBUILT_DIR=<dir>`. No Bazel, no compile. The `ci` workflow publishes
  each id once per target (Linux x86_64 and arm64, each built natively), the first time a push to
  `main` changes a build input; the tarball carries the third-party licence files of everything
  compiled into it.
- **From source (default):** `build.rs` runs `bazelisk build //:vyges_ortools` here
  (`VYGES_BAZEL` names another launcher). The first build compiles OR-Tools, SCIP, HiGHS, abseil
  and protobuf from source; Bazel caches them after that. `VYGES_BUNDLE_TARGET=aarch64-unknown-linux-gnu`
  cross-builds the arm64 archive on an x86_64 host with the same compiler and flags.

The archive is ONE relocatable object (`scripts/bundle.sh`): the solver prelinked with the
toolchain's own libc++, libc++abi and libunwind, every symbol made local except the `vyges_mp_*`
C API. The C++ library is part of the pin, and keeping it private means a program that already
links another C++ runtime is unaffected; no C++ type or exception crosses the C boundary. The final
link needs only `pthread`, `m` and `dl`. Linux (x86_64, arm64) for now.

## Licences

This crate (shim, Rust code, build files) is Apache-2.0. OR-Tools and SCIP are Apache-2.0; see
`NOTICE` for the components compiled into the archive.
