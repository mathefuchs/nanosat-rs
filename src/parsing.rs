use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
    process::{Command, Stdio, exit},
};

use crate::{
    parsing_types::ClauseReceiver,
    solver::clauses::{self, Literal},
};

/// Parsing state tag
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ParseStateTag {
    /// Read a new line character; next char is in the next line
    NewLine,
    ExpectNewLine,
    /// Comment line
    Comment,
    /// `p cnf nv nc` header
    HeaderP, // Expect b' b' next
    HeaderPC,      // Expect b'c' next
    HeaderPCn,     // Expect b'n' next
    HeaderPCnf,    // Expect b'f' next
    HeaderPCnf_,   // Expect b' b' next
    HeaderPCnfN,   // Expect 0-9 next
    HeaderPCnfN_,  // Expect 0-9 or space next
    HeaderPCnfNN,  // Expect 0-9 next
    HeaderPCnfNN_, // Expect 0-9 or new line next
    /// Reading clause
    ClauseDigit, // Expect digit (1-9) next
    ClauseDigitSpace, // Expect digit (0-9), space, or new line next
    ClauseDigitMinus, // Expect digit (0-9) or minus (-) next
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
    /// Currently parsed clause
    pub clause: Vec<Literal>,
    /// Currently parsed literal variable
    pub variable: usize,
    /// Currently parsed literal polarity
    pub polarity: bool,
    /// Current state tag
    pub curr_state_tag: ParseStateTag,
}

impl ParseState {
    fn new() -> Self {
        Self {
            num_variables_header: 0,
            curr_num_variables: 0,
            num_clauses_header: 0,
            curr_num_clauses: 0,
            processed_header: false,
            clause: Vec::new(),
            variable: 0,
            polarity: true,
            curr_state_tag: ParseStateTag::NewLine,
        }
    }
}

/// Received unexpected token
fn unexpected_token() -> ! {
    eprintln!("Failed to parse cnf file.");
    exit(1);
}

/// Open plain text file
fn open_plain_file(filename: &str) -> Box<dyn Read> {
    if let Ok(file) = File::open(filename) {
        Box::new(file)
    } else {
        eprintln!("Failed to open file \"{filename}\" using plain text mode.");
        exit(1);
    }
}

/// Open xz-compressed file
fn open_xz_file(filename: &str) -> Box<dyn Read> {
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
fn open_gzip_file(filename: &str) -> Box<dyn Read> {
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

/// Single state transition; returns whether still satisfiable
#[allow(clippy::too_many_lines)]
fn state_transition(curr_state: &mut ParseState, c: u8, solver: &mut impl ClauseReceiver) -> bool {
    match curr_state.curr_state_tag {
        // Read a new line character; next char is in the next line
        ParseStateTag::NewLine => {
            if c == b'\n' || c == b'\r' {
                return true;
            }
            if !curr_state.processed_header && c == b'p' {
                curr_state.curr_state_tag = ParseStateTag::HeaderP;
                curr_state.processed_header = true;
            } else if c == b'c' {
                curr_state.curr_state_tag = ParseStateTag::Comment;
            } else if curr_state.processed_header && c == b'-' {
                curr_state.polarity = false;
                curr_state.curr_state_tag = ParseStateTag::ClauseDigit;
                // Prepare for new clause
                curr_state.clause.clear();
                curr_state.curr_num_clauses += 1;
            } else if curr_state.processed_header && (b'1'..=b'9').contains(&c) {
                curr_state.variable = usize::from(c - b'0');
                curr_state.polarity = true;
                curr_state.curr_state_tag = ParseStateTag::ClauseDigitSpace;
                // Prepare for new clause
                curr_state.clause.clear();
                curr_state.curr_num_clauses += 1;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::ExpectNewLine => {
            if c == b'\n' || c == b'\r' {
                curr_state.curr_state_tag = ParseStateTag::NewLine;
            } else {
                unexpected_token();
            }
        }

        // Comment line
        ParseStateTag::Comment => {
            if c == b'\n' || c == b'\r' {
                curr_state.curr_state_tag = ParseStateTag::NewLine;
            }
        }

        // `p cnf nv nc` header
        ParseStateTag::HeaderP => {
            if c == b' ' {
                curr_state.curr_state_tag = ParseStateTag::HeaderPC;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPC => {
            if c == b'c' {
                curr_state.curr_state_tag = ParseStateTag::HeaderPCn;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPCn => {
            if c == b'n' {
                curr_state.curr_state_tag = ParseStateTag::HeaderPCnf;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPCnf => {
            if c == b'f' {
                curr_state.curr_state_tag = ParseStateTag::HeaderPCnf_;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPCnf_ => {
            if c == b' ' {
                curr_state.curr_state_tag = ParseStateTag::HeaderPCnfN;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPCnfN => {
            if (b'1'..=b'9').contains(&c) {
                curr_state.num_variables_header = usize::from(c - b'0');
                curr_state.curr_state_tag = ParseStateTag::HeaderPCnfN_;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPCnfN_ => {
            if c == b' ' {
                curr_state.curr_state_tag = ParseStateTag::HeaderPCnfNN;
            } else if c.is_ascii_digit() {
                curr_state.num_variables_header =
                    10 * curr_state.num_variables_header + usize::from(c - b'0');
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPCnfNN => {
            if (b'1'..=b'9').contains(&c) {
                curr_state.num_clauses_header = usize::from(c - b'0');
                curr_state.curr_state_tag = ParseStateTag::HeaderPCnfNN_;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::HeaderPCnfNN_ => {
            if c == b'\n' || c == b'\r' {
                // Completed parsing header
                solver.create_variables(curr_state.num_variables_header);
                curr_state.curr_state_tag = ParseStateTag::NewLine;
            } else if c.is_ascii_digit() {
                curr_state.num_clauses_header =
                    10 * curr_state.num_clauses_header + usize::from(c - b'0');
            } else {
                unexpected_token();
            }
        }

        // Reading clause
        ParseStateTag::ClauseDigit => {
            if (b'1'..=b'9').contains(&c) {
                curr_state.variable = usize::from(c - b'0');
                curr_state.curr_state_tag = ParseStateTag::ClauseDigitSpace;
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::ClauseDigitSpace => {
            if c == b' ' {
                curr_state.curr_state_tag = ParseStateTag::ClauseDigitMinus;
                curr_state
                    .clause
                    .push(clauses::Literal::from_var_with_polarity(
                        curr_state.variable - 1,
                        curr_state.polarity,
                    ));
                if curr_state.variable > curr_state.curr_num_variables {
                    curr_state.curr_num_variables = curr_state.variable;
                }
                curr_state.polarity = true;
            } else if c.is_ascii_digit() {
                curr_state.variable = 10 * curr_state.variable + usize::from(c - b'0');
            } else {
                unexpected_token();
            }
        }
        ParseStateTag::ClauseDigitMinus => {
            if c == b'-' {
                curr_state.curr_state_tag = ParseStateTag::ClauseDigit;
                curr_state.polarity = false;
            } else if c == b'0' {
                curr_state.curr_state_tag = ParseStateTag::ExpectNewLine;
                // Finished clause;
                // if already unsat, no need to add remaining clauses
                let still_satisfiable = solver.add_clause(&curr_state.clause);
                if !still_satisfiable {
                    return false;
                }
            } else if (b'1'..=b'9').contains(&c) {
                curr_state.variable = usize::from(c - b'0');
                curr_state.curr_state_tag = ParseStateTag::ClauseDigitSpace;
            } else {
                unexpected_token();
            }
        }
    }

    true
}

/// Parse `.cnf`, `.cnf.xz`, or `.cnf.gz`
pub fn parse_cnf(filename: &str, solver: &mut impl ClauseReceiver) {
    // Open file
    let path = Path::new(filename);
    let mut file = match path.extension() {
        Some(x) if x.eq_ignore_ascii_case("xz") => open_xz_file(filename),
        Some(x) if x.eq_ignore_ascii_case("gz") => open_gzip_file(filename),
        _ => open_plain_file(filename),
    };
    let mut buffer = [0u8; 4096];

    // Parse file
    let mut curr_state = ParseState::new();
    loop {
        // Read next chunk
        if let Ok(n_bytes_read) = file.read(&mut buffer) {
            if n_bytes_read == 0 {
                break;
            }

            // Run state machine across all read bytes
            for &c in buffer.iter().take(n_bytes_read) {
                if !state_transition(&mut curr_state, c, solver) {
                    return;
                }
            }
        } else {
            unexpected_token();
        }
    }

    // Invalid if file ended in an intermediate state
    if curr_state.curr_state_tag != ParseStateTag::NewLine {
        unexpected_token();
    }

    // Check number of variables and clauses
    if curr_state.curr_num_variables != curr_state.num_variables_header {
        eprintln!("Number of variables in cnf incorrect.");
        exit(1);
    }
    if curr_state.curr_num_clauses != curr_state.num_clauses_header {
        eprintln!("Number of clauses in cnf incorrect.");
        exit(1);
    }
}
