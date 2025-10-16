use nanosat_rs::{parsing_types::ClauseReceiver, solver::clauses::Literal};

/// Mock for solver type
#[derive(Clone, Debug, Default)]
pub struct SolverMock {
    pub num_variables: usize,
    pub num_clauses: usize,
    pub clauses: Vec<Vec<Literal>>,
}

impl ClauseReceiver for SolverMock {
    fn create_variables(&mut self, num_variables: usize) {
        self.num_variables = num_variables;
    }

    fn add_clause(&mut self, literals: &[Literal]) -> bool {
        self.num_clauses += 1;
        self.clauses.push(Vec::from(literals));
        return true;
    }
}
