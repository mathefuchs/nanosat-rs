use nanosat_rs::{
    parsing::parse_cnf,
    solver::{Solver, SolverExitCode, VerbosityLevel},
};

use crate::common::SolverMock;

mod common;

/// Check SAT model
fn check_model(solver: &Solver, mock_solver: &SolverMock) {
    for clause in mock_solver.clauses.iter() {
        let mut contains_true_literal = false;
        for &lit in clause.iter() {
            if lit.is_true(&solver.model()) {
                contains_true_literal = true;
                break;
            }
        }
        assert!(contains_true_literal);
    }
}

#[test]
fn test_solve_small_sat_instance() {
    let mut solver = Solver::new(VerbosityLevel::OnlyResult);
    parse_cnf("res/success/small_sat.cnf", &mut solver);
    let mut mock_solver = SolverMock::default();
    parse_cnf("res/success/small_sat.cnf", &mut mock_solver);

    // Solve
    let res = solver.solve();
    assert_eq!(res, SolverExitCode::Sat);
    check_model(&solver, &mock_solver);
}

#[test]
fn test_solve_medium_sat_instance() {
    let mut solver = Solver::new(VerbosityLevel::OnlyResult);
    parse_cnf("res/success/medium_sat.cnf", &mut solver);
    let mut mock_solver = SolverMock::default();
    parse_cnf("res/success/medium_sat.cnf", &mut mock_solver);

    // Solve
    let res = solver.solve();
    assert_eq!(res, SolverExitCode::Sat);
    check_model(&solver, &mock_solver);
}

#[test]
fn test_solve_big_sat_instance() {
    let mut solver = Solver::new(VerbosityLevel::OnlyResult);
    parse_cnf("res/success/big_sat_instance.cnf.xz", &mut solver);
    let mut mock_solver = SolverMock::default();
    parse_cnf("res/success/big_sat_instance.cnf.xz", &mut mock_solver);

    // Solve
    let res = solver.solve();
    assert_eq!(res, SolverExitCode::Sat);
    check_model(&solver, &mock_solver);
}
