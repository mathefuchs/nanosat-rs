/// Fraction of learned clauses compared to original clauses
pub const MAX_LEARNED_CLAUSES_FACTOR: f64 = 1.0 / 3.0;
/// Increment of the maximum number of learned clauses
pub const MAX_LEARNED_CLAUSES_INCREMENT: f64 = 1.1;
/// After how many conflicts to adjust the
/// maximum number of learned clauses again
pub const MAX_LEARNED_ADJUST_INCREMENT: f64 = 1.5;
/// The base restart interval
pub const RESTART_FIRST: usize = 100;
