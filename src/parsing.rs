use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio, exit},
};

use crate::solver::literal::Literal;

/// Trait describing that a struct can process clauses
pub trait ClauseReceiver {
    /// Creates `num_variables` variables
    fn create_variables(&mut self, num_variables: usize);
    /// Add clauses
    fn add_clause(&mut self, literals: &[Literal]) -> bool;
}

/// Parsing state
struct ParseState {
    /// Number of variabels in header
    pub num_variables_header: usize,
    /// Number of parsed variables
    pub curr_num_variables: usize,
    /// Number of clauses in header
    pub num_clauses_header: usize,
    /// Number of parsed clauses
    pub curr_num_clauses: usize,
    /// Whether already processed the header `p cnf ...`
    pub processed_header: bool,
}

impl ParseState {
    fn new() -> Self {
        Self {
            num_variables_header: 0,
            curr_num_variables: 0,
            num_clauses_header: 0,
            curr_num_clauses: 0,
            processed_header: false,
        }
    }
}

/// Open plain text file
fn open_plain_file(filename: &str) -> Box<dyn BufRead> {
    if let Ok(file) = File::open(filename) {
        Box::new(BufReader::new(file))
    } else {
        eprintln!("Failed to open file \"{filename}\" using plain text mode.");
        exit(1);
    }
}

/// Open xz-compressed file
fn open_xz_file(filename: &str) -> Box<dyn BufRead> {
    if let Ok(child) = Command::new("xz")
        .args(["-dc", filename])
        .stdout(Stdio::piped())
        .spawn()
    {
        let reader = BufReader::new(child.stdout.unwrap());
        Box::new(reader)
    } else {
        eprintln!("Failed to open file \"{filename}\" using \"xz\".");
        exit(1);
    }
}

/// Open gzip-compressed file
fn open_gzip_file(filename: &str) -> Box<dyn BufRead> {
    if let Ok(child) = Command::new("gzip")
        .args(["-dc", filename])
        .stdout(Stdio::piped())
        .spawn()
    {
        let reader = BufReader::new(child.stdout.unwrap());
        Box::new(reader)
    } else {
        eprintln!("Failed to open file \"{filename}\" using \"gzip\".");
        exit(1);
    }
}

/// Unexpected token
fn unexpected_token(err_msg: &str, filename: &str, line_no: usize) -> ! {
    eprintln!("{err_msg} ({filename}:{line_no}).");
    exit(1);
}

/// Parse `.cnf`, `.cnf.xz`, or `.cnf.gz`
pub fn parse_cnf(filename: &str, solver: &mut impl ClauseReceiver) {
    // Open file
    let path = Path::new(filename);
    let file = match path.extension() {
        Some(x) if x.eq_ignore_ascii_case("xz") => open_xz_file(filename),
        Some(x) if x.eq_ignore_ascii_case("gz") => open_gzip_file(filename),
        _ => open_plain_file(filename),
    };

    // Parse file
    let mut curr_state = ParseState::new();
    for (line_idx, line_res) in file.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = line_res
            .unwrap_or_else(|_| unexpected_token("Could not parse line", filename, line_no));
        match line {
            // Comment
            l if l.starts_with('c') => {}
            // Header
            l if l.starts_with("p cnf ") && !curr_state.processed_header => {
                curr_state.processed_header = true;
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() != 4 {
                    unexpected_token("CNF header invalid", filename, line_no);
                }
                curr_state.num_variables_header = parts[2].parse().unwrap_or_else(|_| {
                    unexpected_token(
                        "Could not parse number of variables in header",
                        filename,
                        line_no,
                    )
                });
                curr_state.num_clauses_header = parts[3].parse().unwrap_or_else(|_| {
                    unexpected_token(
                        "Could not parse number of clauses in header",
                        filename,
                        line_no,
                    )
                });
                solver.create_variables(curr_state.num_variables_header);
            }
            // Header missing
            _ if !curr_state.processed_header => {
                unexpected_token("CNF header missing", filename, line_no)
            }
            // Parse clause
            l => {
                let literals: Vec<Literal> = l
                    .split_whitespace()
                    .map(|s| {
                        s.parse::<i32>().unwrap_or_else(|_| {
                            unexpected_token("Could not parse literal", filename, line_no)
                        })
                    })
                    .filter(|&num| num != 0)
                    .map(|num| {
                        let var = usize::try_from(num.unsigned_abs() - 1).unwrap_or_else(|_| {
                            unexpected_token("Could not parse literal", filename, line_no)
                        });
                        if var + 1 > curr_state.curr_num_variables {
                            curr_state.curr_num_variables = var + 1;
                        }
                        Literal::from_var_with_polarity(var, num > 0)
                    })
                    .collect();
                if !literals.is_empty() {
                    curr_state.curr_num_clauses += 1;
                    let still_satisfiable = solver.add_clause(&literals);
                    if !still_satisfiable {
                        break;
                    }
                }
            }
        }
    }

    // Check number of variables and clauses
    if curr_state.curr_num_variables != curr_state.num_variables_header {
        unexpected_token("Number of variables in cnf incorrect", filename, 0);
    }
    if curr_state.curr_num_clauses != curr_state.num_clauses_header {
        unexpected_token("Number of clauses in cnf incorrect", filename, 0);
    }
}
