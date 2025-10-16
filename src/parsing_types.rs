use crate::solver::clauses::Literal;

/// Trait describing that a struct can process clauses
pub trait ClauseReceiver {
    /// Creates `num_variables` variables
    fn create_variables(&mut self, num_variables: usize);
    /// Add clauses
    fn add_clause(&mut self, literals: &[Literal]) -> bool;
}
