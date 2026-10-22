// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Vyges
//
// A C interface to OR-Tools' linear solver (`operations_research::MPSolver`).
//
// 🔑 ONE FUNCTION PER SOLVER CALL. Each function below makes exactly one call on the solver, with
// the arguments it is given, in the order the caller makes them. The shim keeps no state of its own
// and reorders nothing: when two optima tie, the backend's choice can depend on the order variables,
// rows and coefficients were created in, so that order is the CALLER's to decide, not this layer's.
//
// Variables and rows are referred to by the index the solver gave them (creation order, from 0).
// Names are always empty.
//
// A plain C ABI, not a C++ one: the library is compiled by its own toolchain, and nothing C++
// (strings, exceptions, allocators) crosses into the caller. Every entry point catches.

#ifndef VYGES_MP_H
#define VYGES_MP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct vyges_mp_solver vyges_mp_solver;

// `MPSolver::CreateSolver(id)`. NULL when that backend is not linked in (or on any failure).
vyges_mp_solver* vyges_mp_create(const char* solver_id);
void vyges_mp_free(vyges_mp_solver* s);

// `MPSolver::infinity()`.
double vyges_mp_infinity(void);

// `MakeIntVar(lb, ub, "")`; returns the variable's index, or -1 on failure.
int32_t vyges_mp_make_int_var(vyges_mp_solver* s, double lb, double ub);
// `MakeRowConstraint(lb, ub, "")`; returns the row's index, or -1 on failure.
int32_t vyges_mp_make_row_constraint(vyges_mp_solver* s, double lb, double ub);
// `constraint(row)->SetCoefficient(variable(var), coeff)`; 0 on success, -1 on a bad index.
int32_t vyges_mp_set_coefficient(vyges_mp_solver* s, int32_t row, int32_t var, double coeff);
// `MutableObjective()->SetCoefficient(variable(var), coeff)`; 0 on success, -1 on a bad index.
int32_t vyges_mp_objective_set_coefficient(vyges_mp_solver* s, int32_t var, double coeff);
// `MutableObjective()->SetMaximization()` / `SetMinimization()`.
void vyges_mp_objective_set_maximization(vyges_mp_solver* s);
void vyges_mp_objective_set_minimization(vyges_mp_solver* s);

// `Solve()`; the `MPSolver::ResultStatus` value, or -1 if the call threw.
int32_t vyges_mp_solve(vyges_mp_solver* s);
// `variable(var)->solution_value()`; NaN on a bad index.
double vyges_mp_solution_value(const vyges_mp_solver* s, int32_t var);

// `SolverVersion()`, copied NUL-terminated into `buf` (truncated to fit); returns the full length.
int32_t vyges_mp_solver_version(const vyges_mp_solver* s, char* buf, int32_t len);

#ifdef __cplusplus
}
#endif

#endif  // VYGES_MP_H
