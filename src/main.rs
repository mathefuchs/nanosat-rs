use std::{
    env,
    process::{ExitCode, exit},
};

use nanosat_rs::{solve_cnf_instance, solver::VerbosityLevel};

/// Main
fn main() -> ExitCode {
    // Check CLI args
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Expects `nanosat file.cnf`, `nanosat file.cnf.gz`, or `nanosat file.cnf.xz`.");
        exit(1);
    }
    let filename = &args[1];

    // Run solver
    let exit_code = solve_cnf_instance(filename, VerbosityLevel::All);
    ExitCode::from(exit_code as u8)
}
