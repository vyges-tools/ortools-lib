// SPDX-License-Identifier: Apache-2.0
//! OR-Tools' linear solver (`MPSolver`) with the SCIP backend, pinned and statically linked.
//!
//! 🔑 **One method per solver call, in the caller's order.** Each method makes exactly one call on
//! the solver. When optima tie, the answer returned can depend on the order variables, rows and
//! coefficients were created in — so a caller reproducing another program's model must make these
//! calls in that program's order. Nothing here sorts, batches or deduplicates.
//!
//! Variables and rows are the indices the solver assigned (creation order, from 0); names are
//! always empty.

use std::ffi::{c_char, CString};
use std::ptr::NonNull;

mod ffi {
    use std::ffi::c_char;

    #[repr(C)]
    pub struct vyges_mp_solver {
        _private: [u8; 0],
    }

    extern "C" {
        pub fn vyges_mp_create(solver_id: *const c_char) -> *mut vyges_mp_solver;
        pub fn vyges_mp_free(s: *mut vyges_mp_solver);
        pub fn vyges_mp_infinity() -> f64;
        pub fn vyges_mp_make_int_var(s: *mut vyges_mp_solver, lb: f64, ub: f64) -> i32;
        pub fn vyges_mp_make_row_constraint(s: *mut vyges_mp_solver, lb: f64, ub: f64) -> i32;
        pub fn vyges_mp_set_coefficient(s: *mut vyges_mp_solver, row: i32, var: i32, coeff: f64) -> i32;
        pub fn vyges_mp_objective_set_coefficient(s: *mut vyges_mp_solver, var: i32, coeff: f64) -> i32;
        pub fn vyges_mp_objective_set_maximization(s: *mut vyges_mp_solver);
        pub fn vyges_mp_objective_set_minimization(s: *mut vyges_mp_solver);
        pub fn vyges_mp_solve(s: *mut vyges_mp_solver) -> i32;
        pub fn vyges_mp_solution_value(s: *const vyges_mp_solver, var: i32) -> f64;
        pub fn vyges_mp_solver_version(s: *const vyges_mp_solver, buf: *mut c_char, len: i32) -> i32;
    }
}

/// A variable, by the index the solver gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Var(pub i32);

/// A row (linear constraint), by the index the solver gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Row(pub i32);

/// `MPSolver::ResultStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultStatus {
    Optimal,
    Feasible,
    Infeasible,
    Unbounded,
    Abnormal,
    ModelInvalid,
    NotSolved,
    /// A value this crate does not know, or the call failed (-1).
    Other(i32),
}

impl ResultStatus {
    fn from_raw(v: i32) -> Self {
        match v {
            0 => Self::Optimal,
            1 => Self::Feasible,
            2 => Self::Infeasible,
            3 => Self::Unbounded,
            4 => Self::Abnormal,
            5 => Self::ModelInvalid,
            6 => Self::NotSolved,
            v => Self::Other(v),
        }
    }
}

/// A misuse the shim refused: an index the solver never handed out, or a failed call.
#[derive(Debug, PartialEq, Eq)]
pub struct Refused(pub &'static str);

/// One `MPSolver`, owned.
pub struct Solver(NonNull<ffi::vyges_mp_solver>);

impl Solver {
    /// `MPSolver::CreateSolver(id)` — `None` when that backend is not linked in.
    pub fn create(id: &str) -> Option<Self> {
        let id = CString::new(id).ok()?;
        NonNull::new(unsafe { ffi::vyges_mp_create(id.as_ptr()) }).map(Self)
    }

    /// `MPSolver::infinity()`.
    pub fn infinity() -> f64 {
        unsafe { ffi::vyges_mp_infinity() }
    }

    /// `MakeIntVar(lb, ub, "")`.
    pub fn make_int_var(&mut self, lb: f64, ub: f64) -> Result<Var, Refused> {
        match unsafe { ffi::vyges_mp_make_int_var(self.0.as_ptr(), lb, ub) } {
            -1 => Err(Refused("MakeIntVar")),
            i => Ok(Var(i)),
        }
    }

    /// `MakeRowConstraint(lb, ub, "")`.
    pub fn make_row_constraint(&mut self, lb: f64, ub: f64) -> Result<Row, Refused> {
        match unsafe { ffi::vyges_mp_make_row_constraint(self.0.as_ptr(), lb, ub) } {
            -1 => Err(Refused("MakeRowConstraint")),
            i => Ok(Row(i)),
        }
    }

    /// `constraint(row)->SetCoefficient(variable(var), coeff)`.
    pub fn set_coefficient(&mut self, row: Row, var: Var, coeff: f64) -> Result<(), Refused> {
        match unsafe { ffi::vyges_mp_set_coefficient(self.0.as_ptr(), row.0, var.0, coeff) } {
            0 => Ok(()),
            _ => Err(Refused("SetCoefficient")),
        }
    }

    /// `MutableObjective()->SetCoefficient(variable(var), coeff)`.
    pub fn objective_set_coefficient(&mut self, var: Var, coeff: f64) -> Result<(), Refused> {
        match unsafe { ffi::vyges_mp_objective_set_coefficient(self.0.as_ptr(), var.0, coeff) } {
            0 => Ok(()),
            _ => Err(Refused("objective SetCoefficient")),
        }
    }

    /// `MutableObjective()->SetMaximization()`.
    pub fn objective_set_maximization(&mut self) {
        unsafe { ffi::vyges_mp_objective_set_maximization(self.0.as_ptr()) }
    }

    /// `MutableObjective()->SetMinimization()`.
    pub fn objective_set_minimization(&mut self) {
        unsafe { ffi::vyges_mp_objective_set_minimization(self.0.as_ptr()) }
    }

    /// `Solve()`.
    pub fn solve(&mut self) -> ResultStatus {
        ResultStatus::from_raw(unsafe { ffi::vyges_mp_solve(self.0.as_ptr()) })
    }

    /// `variable(var)->solution_value()`; NaN for an index the solver never handed out.
    pub fn solution_value(&self, var: Var) -> f64 {
        unsafe { ffi::vyges_mp_solution_value(self.0.as_ptr(), var.0) }
    }

    /// `SolverVersion()` — for provenance: which backend build answered.
    pub fn solver_version(&self) -> String {
        let n = unsafe { ffi::vyges_mp_solver_version(self.0.as_ptr(), std::ptr::null_mut(), 0) };
        if n < 0 {
            return String::new();
        }
        let mut buf = vec![0 as c_char; n as usize + 1];
        unsafe { ffi::vyges_mp_solver_version(self.0.as_ptr(), buf.as_mut_ptr(), buf.len() as i32) };
        let bytes: Vec<u8> = buf[..n as usize].iter().map(|&c| c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Drop for Solver {
    fn drop(&mut self) {
        unsafe { ffi::vyges_mp_free(self.0.as_ptr()) }
    }
}
