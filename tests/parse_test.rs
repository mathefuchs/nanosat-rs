use std::process::Command;

use nanosat_rs::{parsing::parse_cnf, solver::literal::Literal};

use crate::common::SolverMock;

mod common;

fn check_medium_cnf(file_ending: &str) {
    let mut solver = SolverMock::default();
    parse_cnf(
        format!("res/success/medium_sat.{file_ending}").as_str(),
        &mut solver,
    );
    assert_eq!(solver.num_variables, 403);
    assert_eq!(solver.num_clauses, 2029);
    assert_eq!(solver.clauses.len(), 2029);
    assert_eq!(
        solver.clauses.last(),
        Some(&vec![
            Literal::from_var_with_polarity(402, false),
            Literal::from_var_with_polarity(22, true)
        ])
    );
}

#[test]
fn test_parse_cnf() {
    check_medium_cnf("cnf");
}

#[test]
fn test_parse_cnf_xz() {
    check_medium_cnf("cnf.xz");
}

#[test]
fn test_parse_cnf_gz() {
    check_medium_cnf("cnf.gz");
}

#[test]
fn test_parse_cnf_file_does_not_exist() {
    let exe = env!("CARGO_BIN_EXE_nanosat-rs");
    let output = Command::new(exe)
        .arg("file_not_existing.cnf")
        .output()
        .expect("failed to run main binary");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr,
        "Failed to open file \"file_not_existing.cnf\" using plain text mode.\n"
    );
}

fn check_parsing_fails(file_name: &str, expected_exit_code: i32, expected_error: &str) {
    let exe = env!("CARGO_BIN_EXE_nanosat-rs");
    let output = Command::new(exe)
        .arg(file_name)
        .output()
        .expect("failed to run main binary");
    assert_eq!(output.status.code(), Some(expected_exit_code));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with(expected_error),
        "stderr did not contain \"{expected_error}\"; it container \"{stderr}\""
    );
}

#[test]
fn test_parse_cnf_missing_clause() {
    check_parsing_fails(
        "res/fail/missing_clause.cnf",
        1,
        "Number of clauses in cnf incorrect (res/fail/missing_clause.cnf:0)",
    );
}

#[test]
fn test_parse_cnf_too_many_vars() {
    check_parsing_fails(
        "res/fail/too_many_vars.cnf",
        101,
        "\nthread 'main' panicked",
    );
}

#[test]
fn test_parse_cnf_double_minus() {
    check_parsing_fails(
        "res/fail/double_minus.cnf",
        1,
        "Could not parse literal (res/fail/double_minus.cnf:11)",
    );
}

#[test]
fn test_parse_cnf_empty_clause() {
    check_parsing_fails(
        "res/fail/empty_clause.cnf",
        1,
        "Number of clauses in cnf incorrect (res/fail/empty_clause.cnf:0)",
    );
}

#[test]
fn test_parse_cnf_unknown_line() {
    check_parsing_fails(
        "res/fail/unknown_line.cnf",
        1,
        "Could not parse literal (res/fail/unknown_line.cnf:14)",
    );
}
