#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Builds the solver with Bazel and makes the bundle a Rust (or C) program links:
#   <out>/lib/libvyges_ortools.a   one relocatable object, only the `vyges_mp_*` C API global
#   <out>/include/vyges_mp.h
#
# 🔑 WHY ONE PRELINKED, HIDDEN OBJECT. The hermetic toolchain builds OR-Tools and SCIP against its
# own libc++ — which is part of the pin: the C++ library decides sorts and distributions inside the
# solver. The program linking this crate may already carry another C++ runtime (libstdc++, through
# another `-lib` crate). Two exception runtimes resolving each other's `__cxa_*` would corrupt each
# other's exceptions. So the archive is `ld -r`-linked WITH its libc++/libc++abi/libunwind into one
# object, and every symbol except the C API is made local: the solver's C++ runtime is private to
# it, and no C++ type or exception crosses the C boundary (the shim catches everything).
#
# ⛔ `--whole-archive` for the solver archive, not archive semantics: backends register themselves
# with OR-Tools from static initialisers that nothing references by name. Pulling members by need
# drops them, and `CreateSolver("SCIP")` then returns null.
#
# Linux only for now. Usage: scripts/bundle.sh <out-dir>
set -euo pipefail

out=${1:?usage: scripts/bundle.sh <out-dir>}
here=$(cd "$(dirname "$0")/.." && pwd)
bazel=${VYGES_BAZEL:-$(command -v bazelisk || command -v bazel || true)}
[ -n "$bazel" ] || { echo "bundle.sh: no bazelisk/bazel on PATH (set VYGES_BAZEL)" >&2; exit 1; }
[ "$(uname -s)" = Linux ] || { echo "bundle.sh: Linux only for now" >&2; exit 1; }

cd "$here"
llvm=@@llvm++llvm+llvm-project
runtimes=("$llvm//libcxx:libcxx.static" "$llvm//libcxxabi:libcxxabi.static" "$llvm//libunwind:libunwind.static")
"$bazel" build //:vyges_ortools "${runtimes[@]}"

# The TARGET configuration's archive. Once something is built for the exec (tool) configuration
# too, cquery lists that copy as well — a second line that is not what the program runs.
files() { "$bazel" cquery --output=files "$1" 2>/dev/null | grep '\.a$' | grep -v -- '-exec'; }
solver=$(files //:vyges_ortools)
cxx=$(files "${runtimes[0]}"); cxxabi=$(files "${runtimes[1]}"); unwind=$(files "${runtimes[2]}")
builtins=$(find -L bazel-out/k8-opt/bin/external/llvm+/runtimes/resource_directory_default \
             -name libclang_rt.builtins.a | head -1)
for f in "$solver" "$cxx" "$cxxabi" "$unwind" "$builtins"; do
  [ -s "$f" ] || { echo "bundle.sh: missing or empty archive: $f" >&2; exit 1; }
done

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
nm "$solver" 2>/dev/null | awk '$2 == "T" && $3 ~ /^vyges_mp_/ {print $3}' | sort -u > "$work/keep.txt"
[ -s "$work/keep.txt" ] || { echo "bundle.sh: no vyges_mp_* symbols in $solver" >&2; exit 1; }

ld -r -o "$work/whole.o" --whole-archive "$solver" --no-whole-archive \
   --start-group "$cxx" "$cxxabi" "$unwind" "$builtins" --end-group
objcopy --keep-global-symbols="$work/keep.txt" "$work/whole.o" "$work/local.o"
# ⛔ **Local symbols are not enough: COMDAT GROUPS are merged by their SIGNATURE NAME**, which
# localising a symbol does not change. An inline or template instance the program also has (a
# boost exception vtable, built against the other C++ runtime) shared one group signature with the
# program's copy; the linker kept the first group it saw and DISCARDED the other — leaving the
# program's references to a symbol in a dropped section (lld: "relocation R_X86_64_PC32 cannot be
# used against symbol 'vtable for boost::exception'"), or, linked the other way round, binding the
# program to this runtime's copy. Dropping the group sections makes every member an ordinary
# section of this object: private, like everything else in it.
objcopy --remove-section=.group "$work/local.o" "$work/vyges_ortools.o"

# Nothing but the C API may stay global and defined, and no group may remain to merge.
extra=$(nm -g --defined-only "$work/vyges_ortools.o" | awk '{print $3}' | grep -v '^vyges_mp_' || true)
[ -z "$extra" ] || { echo "bundle.sh: symbols left global: $extra" >&2; exit 1; }
groups=$(readelf -g --wide "$work/vyges_ortools.o" | grep -c 'COMDAT group' || true)
[ "$groups" = 0 ] || { echo "bundle.sh: $groups COMDAT group(s) left" >&2; exit 1; }

mkdir -p "$out/lib" "$out/include"
rm -f "$out/lib/libvyges_ortools.a"
ar rcsD "$out/lib/libvyges_ortools.a" "$work/vyges_ortools.o"
cp shim/vyges_mp.h "$out/include/"
cp ortools-pin.yaml LICENSE NOTICE "$out/"
echo "bundle.sh: $out/lib/libvyges_ortools.a ($(wc -l < "$work/keep.txt") C entry points)"
