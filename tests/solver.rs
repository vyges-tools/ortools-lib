// SPDX-License-Identifier: Apache-2.0
use vyges_ortools_lib::{ResultStatus, Row, Solver, Var};

fn scip() -> Solver {
    Solver::create("SCIP").expect("the SCIP backend is linked in")
}

/// The pinned backend answers, and says which build it is.
#[test]
fn the_scip_backend_is_the_pinned_release() {
    let s = scip();
    let v = s.solver_version();
    assert!(v.contains("SCIP 9.2.3"), "{v}");
}

/// A backend that is not linked in is `None`, not a crash.
#[test]
fn an_unknown_backend_is_none() {
    assert!(Solver::create("NO_SUCH_BACKEND").is_none());
}

/// Max 5x + 4y + 3z, 2x + 3y + z <= 5, x/y/z binary: the unique optimum is x = y = 1, z = 0 (9).
#[test]
fn a_small_knapsack_solves_to_its_unique_optimum() {
    let mut s = scip();
    let x = s.make_int_var(0.0, 1.0).unwrap();
    let y = s.make_int_var(0.0, 1.0).unwrap();
    let z = s.make_int_var(0.0, 1.0).unwrap();
    // Indices are creation order, from 0.
    assert_eq!((x, y, z), (Var(0), Var(1), Var(2)));
    let cap = s.make_row_constraint(-Solver::infinity(), 5.0).unwrap();
    assert_eq!(cap, Row(0));
    for (v, w) in [(x, 2.0), (y, 3.0), (z, 1.0)] {
        s.set_coefficient(cap, v, w).unwrap();
    }
    for (v, c) in [(x, 5.0), (y, 4.0), (z, 3.0)] {
        s.objective_set_coefficient(v, c).unwrap();
    }
    s.objective_set_maximization();
    assert_eq!(s.solve(), ResultStatus::Optimal);
    assert_eq!([s.solution_value(x), s.solution_value(y), s.solution_value(z)], [1.0, 1.0, 0.0]);
}

/// x binary with x >= 2 has no solution.
#[test]
fn an_infeasible_model_says_so() {
    let mut s = scip();
    let x = s.make_int_var(0.0, 1.0).unwrap();
    let r = s.make_row_constraint(2.0, Solver::infinity()).unwrap();
    s.set_coefficient(r, x, 1.0).unwrap();
    assert_eq!(s.solve(), ResultStatus::Infeasible);
}

/// An index the solver never handed out is refused at the shim, not passed to the solver.
#[test]
fn an_index_the_solver_never_gave_is_refused() {
    let mut s = scip();
    let x = s.make_int_var(0.0, 1.0).unwrap();
    assert!(s.set_coefficient(Row(0), x, 1.0).is_err());
    assert!(s.objective_set_coefficient(Var(7), 1.0).is_err());
    assert!(s.solution_value(Var(-1)).is_nan());
}
