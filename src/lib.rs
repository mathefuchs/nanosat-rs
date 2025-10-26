use std::time::Instant;

use crate::{
    parsing::parse_cnf,
    solver::{Solver, SolverExitCode, VerbosityLevel},
};

mod helper;
pub mod parsing;
pub mod solver;

/// Duration between `start` and `end` in seconds
#[allow(clippy::cast_precision_loss)]
fn duration_in_seconds(start: Instant, end: Instant) -> f64 {
    (end - start).as_micros() as f64 / 1e6
}

/// Print stats on the loaded CNF instance
fn print_stats(solver: &Solver, start_time: Instant, parse_end_time: Instant) {
    print!(
        "
============================[ Problem Statistics ]=============================
|                                                                             |
|  Number of variables:  {:>12}                                         |
|  Number of clauses:    {:>12}                                         |
|  Parse time:           {:>12.6}                                         |
|                                                                             |",
        solver.num_variables(),
        solver.num_clauses(),
        duration_in_seconds(start_time, parse_end_time)
    );
}

/// Print search statistics header
fn print_search_stats_banner() {
    print!(
        "
============================[ Search Statistics ]==============================
| Conflicts |          ORIGINAL         |          LEARNED         | Progress |
|           |    Vars  Clauses Literals |    Limit  Clauses Lit/Cl |          |
===============================================================================
"
    );
}

/// Print stats after finished with solving
#[allow(clippy::cast_precision_loss)]
fn print_post_solve_stats(solver: &Solver, start_time: Instant, end_time: Instant) {
    let total_time = duration_in_seconds(start_time, end_time);
    let conflicts_per_s = solver.statistics().num_total_conflicts as f64 / total_time;
    let propagations_per_s = solver.statistics().num_propagations as f64 / total_time;
    print!(
        "============================[      Summary      ]==============================
|                                                                             |
|  #Restarts:            {:>12}                                         |
|  #Conflicts:           {:>12} ({:>12.3}/sec)                      |
|  #Decisions:           {:>12}                                         |
|  #Propagations:        {:>12} ({:>12.3}/sec)                      |
|  Total time:           {:>12.6}                                         |
|                                                                             |
===============================================================================
",
        solver.statistics().num_restarts,
        solver.statistics().num_total_conflicts,
        conflicts_per_s,
        solver.statistics().num_decisions,
        solver.statistics().num_propagations,
        propagations_per_s,
        total_time
    );
}

/// Print result
fn print_result(solver: &Solver, exit_code: SolverExitCode) {
    println!();
    match exit_code {
        // Unknown
        SolverExitCode::Unknown => {
            println!("UNKNOWN");
        }
        // SAT
        SolverExitCode::Sat => {
            print!("SAT");
            for var in 0..solver.model().len() {
                let val = solver.model()[var];
                debug_assert!(val.is_true() || val.is_false());
                if val.is_true() {
                    print!(" {}", var + 1);
                } else {
                    print!(" -{}", var + 1);
                }
            }
            println!();
        }
        // UNSAT
        SolverExitCode::Unsat => {
            println!("UNSAT");
        }
    }
}

/// Solves a CNF instance in a `.cnf`, `.cnf.xz`, or `.cnf.gz` file
#[must_use]
pub fn solve_cnf_instance(filename: &str, logging_level: VerbosityLevel) -> SolverExitCode {
    // Create solver and parse clauses
    let start_time = Instant::now();
    let mut solver = Solver::new(logging_level);
    parse_cnf(filename, &mut solver);
    if logging_level == VerbosityLevel::All {
        let parse_end_time = Instant::now();
        print_stats(&solver, start_time, parse_end_time);
        print_search_stats_banner();
    }

    // Solve
    let exit_code = solver.solve();

    // End time recording; print elapsed time
    if logging_level == VerbosityLevel::All {
        let end_time = Instant::now();
        print_post_solve_stats(&solver, start_time, end_time);
    }

    // Print model
    print_result(&solver, exit_code);

    // Return unknown (0), sat (10), or unsat (20)
    exit_code
}
