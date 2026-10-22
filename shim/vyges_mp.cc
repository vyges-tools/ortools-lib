// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Vyges

#include "shim/vyges_mp.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <memory>
#include <string>

#include "ortools/linear_solver/linear_solver.h"

using operations_research::MPSolver;

struct vyges_mp_solver {
  std::unique_ptr<MPSolver> solver;
};

namespace {

bool var_ok(const vyges_mp_solver* s, int32_t var) {
  return s != nullptr && var >= 0 && var < s->solver->NumVariables();
}

bool row_ok(const vyges_mp_solver* s, int32_t row) {
  return s != nullptr && row >= 0 && row < s->solver->NumConstraints();
}

}  // namespace

extern "C" {

vyges_mp_solver* vyges_mp_create(const char* solver_id) {
  try {
    std::unique_ptr<MPSolver> solver(MPSolver::CreateSolver(solver_id));
    if (!solver) {
      return nullptr;
    }
    return new vyges_mp_solver{std::move(solver)};
  } catch (...) {
    return nullptr;
  }
}

void vyges_mp_free(vyges_mp_solver* s) {
  delete s;
}

double vyges_mp_infinity(void) {
  return MPSolver::infinity();
}

int32_t vyges_mp_make_int_var(vyges_mp_solver* s, double lb, double ub) {
  if (s == nullptr) {
    return -1;
  }
  try {
    return s->solver->MakeIntVar(lb, ub, "")->index();
  } catch (...) {
    return -1;
  }
}

int32_t vyges_mp_make_row_constraint(vyges_mp_solver* s, double lb, double ub) {
  if (s == nullptr) {
    return -1;
  }
  try {
    return s->solver->MakeRowConstraint(lb, ub, "")->index();
  } catch (...) {
    return -1;
  }
}

int32_t vyges_mp_set_coefficient(vyges_mp_solver* s, int32_t row, int32_t var, double coeff) {
  if (!row_ok(s, row) || !var_ok(s, var)) {
    return -1;
  }
  try {
    s->solver->constraint(row)->SetCoefficient(s->solver->variable(var), coeff);
    return 0;
  } catch (...) {
    return -1;
  }
}

int32_t vyges_mp_objective_set_coefficient(vyges_mp_solver* s, int32_t var, double coeff) {
  if (!var_ok(s, var)) {
    return -1;
  }
  try {
    s->solver->MutableObjective()->SetCoefficient(s->solver->variable(var), coeff);
    return 0;
  } catch (...) {
    return -1;
  }
}

void vyges_mp_objective_set_maximization(vyges_mp_solver* s) {
  if (s != nullptr) {
    s->solver->MutableObjective()->SetMaximization();
  }
}

void vyges_mp_objective_set_minimization(vyges_mp_solver* s) {
  if (s != nullptr) {
    s->solver->MutableObjective()->SetMinimization();
  }
}

int32_t vyges_mp_solve(vyges_mp_solver* s) {
  if (s == nullptr) {
    return -1;
  }
  try {
    return static_cast<int32_t>(s->solver->Solve());
  } catch (...) {
    return -1;
  }
}

double vyges_mp_solution_value(const vyges_mp_solver* s, int32_t var) {
  if (!var_ok(s, var)) {
    return std::nan("");
  }
  return s->solver->variable(var)->solution_value();
}

int32_t vyges_mp_solver_version(const vyges_mp_solver* s, char* buf, int32_t len) {
  if (s == nullptr) {
    return -1;
  }
  try {
    const std::string v = s->solver->SolverVersion();
    if (buf != nullptr && len > 0) {
      const size_t n = std::min(v.size(), static_cast<size_t>(len - 1));
      std::memcpy(buf, v.data(), n);
      buf[n] = '\0';
    }
    return static_cast<int32_t>(v.size());
  } catch (...) {
    return -1;
  }
}

}  // extern "C"
