use crate::{
    helper::{f64_to_usize_trunc, usize_to_f64},
    parsing_types::ClauseReceiver,
    solver::clauses::{
        ClauseRef, Clauses, Literal, Variable, VariableMetadata, VariableValue, Watch,
    },
};
use rand::{Rng, SeedableRng, seq::SliceRandom};

pub mod clauses;
mod options;
mod restart;

/// Verbosity level enum
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VerbosityLevel {
    OnlyResult = 0,
    All = 1,
}

/// Enum representing the solver status exit codes
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SolverExitCode {
    Unknown = 0,
    Sat = 10,
    Unsat = 20,
}

/// Solver statistics
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SolverStatistics {
    /// Number of variables
    pub num_variables: usize,
    /// Number of clauses
    pub num_clauses: usize,
    /// Number of literals in clauses
    pub num_literals_in_clauses: usize,
    /// Number of learned clauses
    pub num_learned_clauses: usize,
    /// Number of literals in learned clauses
    pub num_literals_in_learned_clauses: usize,
    /// Number of search (re-)starts
    pub num_restarts: usize,
    /// Number of made decisions
    pub num_decisions: usize,
    /// Number of total conflicts
    pub num_total_conflicts: usize,
    /// Number of total propagations
    pub num_propagations: usize,
}

/// Used for analyzing conflicts in `analyzeConflict`
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum VariableStatus {
    /// Variable does not participate in conflict
    Unset = 0,
    /// Variable is a source of conflict
    IsSource = 1,
    /// Variable causes a conflict but could be removed
    Removable = 2,
    /// Removing variable failed
    RemovalFailed = 3,
}

/// Solver class
pub struct Solver {
    // -- Representation of the SAT problem instance
    /// All clauses
    clauses: Clauses,

    // -- Solver data structures
    /// All learned clauses
    learned_clauses: Clauses,
    /// Stack of all decisions currently made
    trail: Vec<Literal>,
    /// Where the decision levels in `trail` start
    trail_separators: Vec<usize>,
    /// Points to the next literal in `trail` to propagate
    trail_propagation_head: usize,
    /// Current variable assignments (variables can be unset, false, or true)
    variable_values: Vec<VariableValue>,
    /// Stores the preferred polarity of a variable (phase saving)
    variable_polarity: Vec<bool>,
    /// Stores metadata for all variables
    variable_metadata: Vec<VariableMetadata>,
    /// Maintains which clauses watch each literal
    literals_watched_by: Vec<Vec<Watch>>,
    /// Unset variables
    unset_variables: Vec<Variable>,

    // -- Solver state
    /// Logging level
    logging_level: VerbosityLevel,
    /// Maximum number of learned clauses allowed
    /// (is dynamically scaled, therefore `double`)
    max_learned_clauses: f64,
    /// By how much to adjust the learned clauses size on conflict
    learned_size_adjust_on_conflict: f64,
    /// Specifies after how many conflicts to adjust the learned clauses size
    learned_size_adjust_count: usize,
    /// Random generator
    random_gen: rand::rngs::SmallRng,
    /// Solver statistics
    stats: SolverStatistics,
}

impl Solver {
    /// Create a new solver
    #[must_use]
    pub fn new(logging_level: VerbosityLevel) -> Self {
        Self {
            clauses: Clauses::default(),
            learned_clauses: Clauses::default(),
            trail: Vec::new(),
            trail_separators: Vec::new(),
            trail_propagation_head: 0,
            variable_values: Vec::new(),
            variable_polarity: Vec::new(),
            variable_metadata: Vec::new(),
            literals_watched_by: Vec::new(),
            unset_variables: Vec::new(),
            logging_level,
            max_learned_clauses: 0.0,
            learned_size_adjust_on_conflict: 100.0,
            learned_size_adjust_count: 100,
            random_gen: rand::rngs::SmallRng::seed_from_u64(42),
            stats: SolverStatistics::default(),
        }
    }

    /// Number of variables
    #[must_use]
    pub const fn num_variables(&self) -> usize {
        self.stats.num_variables
    }

    /// Number of clauses
    #[must_use]
    pub const fn num_clauses(&self) -> usize {
        self.stats.num_clauses
    }

    /// Solver statistics
    #[must_use]
    pub const fn statistics(&self) -> &SolverStatistics {
        &self.stats
    }

    /// Contains the model if SAT
    #[must_use]
    pub const fn model(&self) -> &Vec<VariableValue> {
        &self.variable_values
    }

    /// Solves the loaded problem instance
    pub fn solve(&mut self) -> SolverExitCode {
        // Check that clauses are non-empty
        if self.num_variables() == 0 || self.num_clauses() == 0 {
            return SolverExitCode::Unknown;
        }

        // Initial simplification
        if !self.simplify() {
            return SolverExitCode::Unsat;
        }

        // Update maximum learned clauses size
        self.max_learned_clauses =
            usize_to_f64(self.num_clauses()) * options::MAX_LEARNED_CLAUSES_FACTOR;

        // Main loop
        self.stats.num_restarts = 0;
        let mut status = SolverExitCode::Unknown;
        while status == SolverExitCode::Unknown {
            // Restart search after reaching a certain number of conflicts
            // using the Luby restart sequence
            let restart_value = restart::luby(self.stats.num_restarts) * options::RESTART_FIRST;
            status = self.search(restart_value);
            self.stats.num_restarts += 1;
        }

        // Return solver exit status
        status
    }

    /// Search for a model with the given number of allowed conflicts
    fn search(&mut self, allowed_num_of_conflicts: usize) -> SolverExitCode {
        // Number of conflicts
        let mut num_conflicts = 0;
        // Currently learned clause
        let mut learned_clause = Vec::new();

        // Search until finding model or reaching allowed number of conflicts
        loop {
            // Propagate currently selected variables
            let conflict = self.propagate();

            // Check if conflict found
            if conflict.valid() {
                // Found conflict
                self.stats.num_total_conflicts += 1;
                num_conflicts += 1;

                // Conflict reached outer-most layer; UNSAT
                if self.decision_level() == 0 {
                    return SolverExitCode::Unsat;
                }

                // Analyze conflict
                learned_clause.clear();
                let backtrack_level = self.analyze_conflict(conflict, &mut learned_clause);
                self.revert_trail(backtrack_level);

                if learned_clause.len() == 1 {
                    // Found single-literal reason for conflict, propagate
                    self.assign_literal(learned_clause[0], ClauseRef::default());
                } else {
                    // Else, learn clause and propagate first literal
                    let clause_ref = self.attach_clause(learned_clause.clone(), true);
                    self.assign_literal(learned_clause[0], clause_ref);
                }

                // Update maximum number of learned clauses
                self.learned_size_adjust_count -= 1;
                if self.learned_size_adjust_count == 0 {
                    self.learned_size_adjust_on_conflict *= options::MAX_LEARNED_ADJUST_INCREMENT;
                    self.learned_size_adjust_count =
                        f64_to_usize_trunc(self.learned_size_adjust_on_conflict);
                    self.max_learned_clauses *= options::MAX_LEARNED_CLAUSES_INCREMENT;

                    // Log progress
                    if self.logging_level == VerbosityLevel::All {
                        let free_variables = self.stats.num_variables
                            - (if self.trail_separators.is_empty() {
                                self.trail.len()
                            } else {
                                self.trail_separators[0]
                            });
                        let literals_per_learned =
                            usize_to_f64(self.stats.num_literals_in_learned_clauses)
                                / usize_to_f64(self.stats.num_learned_clauses);
                        let progress_estimate_percent = self.progress_estimate() * 100.0;
                        println!(
                            "| {:>9} | {:>7} {:>8} {:>8} | {:>8.0} {:>8} {:>6.0} | {:>6.3} % |",
                            self.stats.num_total_conflicts,
                            free_variables,
                            self.stats.num_clauses,
                            self.stats.num_literals_in_clauses,
                            self.max_learned_clauses,
                            self.stats.num_learned_clauses,
                            literals_per_learned,
                            progress_estimate_percent
                        );
                    }
                }
            } else {
                // No conflict
                if num_conflicts >= allowed_num_of_conflicts {
                    // Reached bound on number of conflicts; revert complete trail
                    self.revert_trail(0);
                    return SolverExitCode::Unknown;
                }

                // Simplify the set of clauses
                if self.decision_level() == 0 && !self.simplify() {
                    return SolverExitCode::Unsat;
                }

                // Reduce the set of learned clauses if too many
                if usize_to_f64(self.learned_clauses.len())
                    >= self.max_learned_clauses + usize_to_f64(self.trail.len())
                {
                    self.prune_learned_clauses();
                }

                // New variable decision
                self.stats.num_decisions += 1;
                if let Some(next_literal) = self.pick_branch_literal() {
                    // Increase decision level
                    self.trail_separators.push(self.trail.len());

                    // Enqueue next branch literal
                    self.assign_literal(next_literal, ClauseRef::default());
                } else {
                    // Model found if all variables assigned without conflict
                    return SolverExitCode::Sat;
                }
            }
        }
    }

    /// Analyze the given conflict; returns the backtrack level
    /// and the learned clause
    fn analyze_conflict(
        &mut self,
        initial_conflict: ClauseRef,
        out_learned_clause: &mut Vec<Literal>,
    ) -> usize {
        // Leave room for the asserting literal
        out_learned_clause.push(Literal::default());
        let mut conflict = initial_conflict;
        let mut index = self.trail.len();
        let mut path_length = 0;
        let mut initial_pass = true;
        let mut asserting_literal = Literal::default();
        let mut variable_seen = vec![VariableStatus::Unset; self.num_variables()];

        // Build learned conflict clause
        while initial_pass || path_length > 0 {
            debug_assert!(conflict.valid());
            initial_pass = false;
            let conflict_clause_len = self.clause_at(conflict).len();
            let start = usize::from(asserting_literal.valid());
            for j in start..conflict_clause_len {
                let conflict_literal = self.clause_at(conflict)[j];

                // Check unseen variables in clause
                if variable_seen[conflict_literal.var()] == VariableStatus::Unset
                    && self.variable_metadata[conflict_literal.var()].decision_level > 0
                {
                    variable_seen[conflict_literal.var()] = VariableStatus::IsSource;

                    if self.variable_metadata[conflict_literal.var()].decision_level
                        >= self.decision_level()
                    {
                        path_length += 1;
                    } else {
                        out_learned_clause.push(conflict_literal);
                    }
                }
            }

            // Select next clause to look at
            while variable_seen[self.trail[index - 1].var()] == VariableStatus::Unset {
                index -= 1;
            }
            index -= 1;

            asserting_literal = self.trail[index];
            conflict = self.variable_metadata[asserting_literal.var()].reason_clause_idx;
            variable_seen[asserting_literal.var()] = VariableStatus::Unset;
            path_length -= 1;
        }
        out_learned_clause[0] = !asserting_literal;

        // Simplify conflict clause
        let mut i = 1;
        let mut j = 1;
        while i < out_learned_clause.len() {
            // Literal needed if it has top-level assignment or is not redundant
            if !self.variable_metadata[out_learned_clause[i].var()]
                .reason_clause_idx
                .valid()
                || !self.is_literal_redundant_in_conflict_clause(
                    &mut variable_seen,
                    out_learned_clause[i],
                )
            {
                out_learned_clause[j] = out_learned_clause[i];
                j += 1;
            }
            i += 1;
        }
        out_learned_clause.resize(j, Literal::default());

        // Find correct backtrack level
        let mut out_btlevel = 0;
        if out_learned_clause.len() != 1 {
            // Find the first literal assigned at the next-highest level
            let mut max_i = 1;
            for i in 2..out_learned_clause.len() {
                if self.variable_metadata[out_learned_clause[i].var()].decision_level
                    > self.variable_metadata[out_learned_clause[max_i].var()].decision_level
                {
                    max_i = i;
                }
            }

            // Swap-in this literal at index 1
            let literal = out_learned_clause[max_i];
            out_learned_clause.swap(max_i, 1);
            out_btlevel = self.variable_metadata[literal.var()].decision_level;
        }

        out_btlevel
    }

    /// Checks whether literal is redundant in the conflict
    fn is_literal_redundant_in_conflict_clause(
        &self,
        variable_seen: &mut [VariableStatus],
        initial_literal: Literal,
    ) -> bool {
        let mut literal = initial_literal;
        debug_assert!(
            variable_seen[literal.var()] == VariableStatus::Unset
                || variable_seen[literal.var()] == VariableStatus::IsSource
        );
        debug_assert!(
            self.variable_metadata[literal.var()]
                .reason_clause_idx
                .valid()
        );
        let mut clause = self.clause_at(self.variable_metadata[literal.var()].reason_clause_idx);
        let mut stack = Vec::new();
        let mut i = 0;
        loop {
            i += 1;
            if i < clause.len() {
                // Checking `literal`'s parent `l` (in reason graph)
                let parent = clause[i];

                // Variable at level 0 or previously removable
                if self.variable_metadata[parent.var()].decision_level == 0
                    || variable_seen[parent.var()] == VariableStatus::IsSource
                    || variable_seen[parent.var()] == VariableStatus::Removable
                {
                    continue;
                }

                // Check variable can not be removed for some local reason
                if !self.variable_metadata[parent.var()]
                    .reason_clause_idx
                    .valid()
                    || variable_seen[parent.var()] == VariableStatus::RemovalFailed
                {
                    stack.push((0, literal));
                    for i in 0..stack.len() {
                        if variable_seen[stack[i].1.var()] == VariableStatus::Unset {
                            variable_seen[stack[i].1.var()] = VariableStatus::RemovalFailed;
                        }
                    }

                    return false;
                }

                // Recursively check `parent`
                stack.push((i, literal));
                i = 0;
                literal = parent;
                clause = self.clause_at(self.variable_metadata[literal.var()].reason_clause_idx);
            } else {
                // Finished with current element `literal` and reason `clause`
                if variable_seen[literal.var()] == VariableStatus::Unset {
                    variable_seen[literal.var()] = VariableStatus::Removable;
                }

                if let Some((top_i, top_literal)) = stack.pop() {
                    // Continue with top element on stack
                    i = top_i;
                    literal = top_literal;
                    clause =
                        self.clause_at(self.variable_metadata[literal.var()].reason_clause_idx);
                } else {
                    // Terminate with success if stack is empty
                    return true;
                }
            }
        }
    }

    /// Prune learned clauses if too many
    fn prune_learned_clauses(&mut self) {
        for i in 0..self.learned_clauses.len() {
            let clause_ref = ClauseRef::from_idx(i, true);
            let clause = &self.learned_clauses[clause_ref];

            // Skip deleted clauses
            if clause.is_empty() {
                continue;
            }

            // Randomly delete learned clauses;
            // do not delete binary or referenced clauses
            if clause.len() > 2
                && self.random_gen.random_bool(0.5)
                && !self.is_locked_clause(clause_ref)
            {
                self.detach_clause(clause_ref);
            }
        }
    }

    /// Progress estimate
    fn progress_estimate(&self) -> f64 {
        let set_vars = *self.trail_separators.first().unwrap_or(&self.trail.len());
        usize_to_f64(set_vars) / usize_to_f64(self.num_variables())
    }

    /// Pick next literal to branch on
    fn pick_branch_literal(&mut self) -> Option<Literal> {
        // Random decision
        while !self.unset_variables.is_empty() {
            // Select random unset variable
            let idx = self.random_gen.random_range(0..self.unset_variables.len());
            let var = self.unset_variables.swap_remove(idx);

            // Check whether variable is unset
            if self.variable_values[var].is_unset() {
                // Choose polarity based on preferred polarity
                return Some(Literal::from_var_with_polarity(
                    var,
                    self.variable_polarity[var],
                ));
            }
        }

        None
    }

    /// Accesses an original or learned clause
    fn clause_at(&self, clause_ref: ClauseRef) -> &Vec<Literal> {
        if clause_ref.is_learned() {
            &self.learned_clauses[clause_ref]
        } else {
            &self.clauses[clause_ref]
        }
    }

    /// Accesses an original or learned clause
    fn clause_at_mut(&mut self, clause_ref: ClauseRef) -> &mut Vec<Literal> {
        if clause_ref.is_learned() {
            &mut self.learned_clauses[clause_ref]
        } else {
            &mut self.clauses[clause_ref]
        }
    }

    /// Check that clause is not the reason of some propagation
    fn is_locked_clause(&self, clause_ref: ClauseRef) -> bool {
        let clause = self.clause_at(clause_ref);
        clause[0].is_true(&self.variable_values)
            && self.variable_metadata[clause[0].var()]
                .reason_clause_idx
                .valid()
            && self.variable_metadata[clause[0].var()].reason_clause_idx == clause_ref
    }

    /// Propagate all facts in `trail` starting from `trail_propagation_head`;
    /// returns conflicting clause index or `UNDEF_CLAUSE` if none
    fn propagate(&mut self) -> ClauseRef {
        // Current conflict
        let mut conflict = ClauseRef::default();

        // Propagates all enqueued facts
        while self.trail_propagation_head < self.trail.len() {
            // Get literal and watches to propagate
            let literal_to_propagate = self.trail[self.trail_propagation_head];
            self.trail_propagation_head += 1;
            self.stats.num_propagations += 1;

            // Check all watches
            let num_watches = self.literals_watched_by[literal_to_propagate.repr()].len();
            let mut i = 0;
            let mut j = 0;
            while i < num_watches {
                // Clause can be skipped if `Watch::blocker` literal is true
                let (clause_ref, blocker) = {
                    let watches = &mut self.literals_watched_by[literal_to_propagate.repr()];
                    let clause_ref = watches[i].clause_ref;
                    let blocker = watches[i].blocker;
                    if blocker.is_true(&self.variable_values) {
                        watches[j] = watches[i];
                        i += 1;
                        j += 1;
                        continue;
                    }
                    (clause_ref, blocker)
                };

                // Make sure the false literal is at position 2
                let not_literal = !literal_to_propagate;
                let first_literal = {
                    let clause = self.clause_at_mut(clause_ref);
                    if clause[0] == not_literal {
                        clause[0] = clause[1];
                        clause[1] = not_literal;
                    }
                    debug_assert!(clause[1] == not_literal);
                    i += 1;
                    clause[0]
                };

                // If first watch is true, then clause is already satisfied
                let new_watch = Watch::from_ref_and_blocker(clause_ref, first_literal);
                if first_literal != blocker && first_literal.is_true(&self.variable_values) {
                    self.literals_watched_by[literal_to_propagate.repr()][j] = new_watch;
                    j += 1;
                    continue;
                }

                // Look for new watch that is not already false
                let found_new_watch = {
                    let clause = if clause_ref.is_learned() {
                        &mut self.learned_clauses[clause_ref]
                    } else {
                        &mut self.clauses[clause_ref]
                    };
                    let (vals, ws) = (&self.variable_values, &mut self.literals_watched_by);
                    let mut found_new_watch = false;
                    for k in 2..clause.len() {
                        if !clause[k].is_false(vals) {
                            clause[1] = clause[k];
                            clause[k] = not_literal;
                            ws[(!clause[1]).repr()].push(new_watch);
                            found_new_watch = true;
                            break;
                        }
                    }
                    found_new_watch
                };
                if found_new_watch {
                    continue;
                }

                // Did not find new watch; clause must be unit
                let literal_watches = &mut self.literals_watched_by[literal_to_propagate.repr()];
                literal_watches[j] = new_watch;
                j += 1;
                if first_literal.is_false(&self.variable_values) {
                    // Found conflict
                    conflict = clause_ref;
                    self.trail_propagation_head = self.trail.len();
                    while i < literal_watches.len() {
                        literal_watches[j] = literal_watches[i];
                        i += 1;
                        j += 1;
                    }
                } else {
                    // Found fact
                    self.assign_literal(first_literal, clause_ref);
                }
            }

            // Resize `watches`
            let watches = &mut self.literals_watched_by[literal_to_propagate.repr()];
            watches.resize(j, Watch::default());
        }

        // Return current conflict
        conflict
    }

    /// Returns the current decision level
    const fn decision_level(&self) -> usize {
        self.trail_separators.len()
    }

    /// Reverts the assignment trail until the given decision level
    fn revert_trail(&mut self, level: usize) {
        // Reverting to `level` only necessary if current level higher
        if self.decision_level() > level {
            let mut c = self.trail.len();
            while c > self.trail_separators[level] {
                // Literal to revert
                let literal_to_revert = self.trail[c - 1];
                let variable = literal_to_revert.var();
                let polarity = literal_to_revert.polarity();

                // Unset assignment and save preferred polarity
                self.variable_values[variable] = VariableValue::Unset;
                self.variable_polarity[variable] = polarity;
                self.unset_variables.push(variable);
                c -= 1;
            }

            // Shrink `trail` and `trail_separators` to specified `level`
            self.trail_propagation_head = self.trail_separators[level];
            self.trail
                .resize(self.trail_propagation_head, Literal::default());
            self.trail_separators.resize(level, 0);
        }
    }

    /// Assigns the given literal (must be unset previously)
    fn assign_literal(&mut self, literal: Literal, reason_clause_idx: ClauseRef) {
        // Assigned literal must be unset previously
        let var = literal.var();
        debug_assert!(self.variable_values[var].is_unset());

        // Assign literal
        self.variable_values[var] = VariableValue::from_bool(literal.polarity());
        self.variable_metadata[var].decision_level = self.decision_level();
        self.variable_metadata[var].reason_clause_idx = reason_clause_idx;
        self.trail.push(literal);
    }

    /// Removes a watch from `literals_watched_by`
    fn remove_watch(&mut self, literal: Literal, watch_to_remove: Watch) {
        // Find watch
        let watches = &mut self.literals_watched_by[literal.repr()];
        let mut i = 0;
        while i < watches.len() && watches[i] != watch_to_remove {
            i += 1;
        }

        // Assert that watch found
        debug_assert!(i < watches.len());

        // Move remaining watches and pop back
        watches.remove(i);
    }

    /// Attaches a clause by creating watches
    fn attach_clause(&mut self, literals: Vec<Literal>, is_learned: bool) -> ClauseRef {
        // Add clause
        let first_literal = literals[0];
        let second_literal = literals[1];
        let clause_ref = if is_learned {
            self.stats.num_learned_clauses += 1;
            self.stats.num_literals_in_learned_clauses += literals.len();
            self.learned_clauses.add_clause(literals, true)
        } else {
            self.stats.num_clauses += 1;
            self.stats.num_literals_in_clauses += literals.len();
            self.clauses.add_clause(literals, false)
        };

        // Keep two watches per clause
        self.literals_watched_by[(!first_literal).repr()]
            .push(Watch::from_ref_and_blocker(clause_ref, second_literal));
        self.literals_watched_by[(!second_literal).repr()]
            .push(Watch::from_ref_and_blocker(clause_ref, first_literal));
        clause_ref
    }

    /// Removes a clause by removing watches and clearing literals
    fn detach_clause(&mut self, clause_ref: ClauseRef) {
        let (first_lit, second_lit, len) = {
            let clause = self.clause_at(clause_ref);
            (clause[0], clause[1], clause.len())
        };
        self.remove_watch(
            !first_lit,
            Watch::from_ref_and_blocker(clause_ref, second_lit),
        );
        self.remove_watch(
            !second_lit,
            Watch::from_ref_and_blocker(clause_ref, first_lit),
        );
        if self.is_locked_clause(clause_ref) {
            self.variable_metadata[first_lit.var()].reason_clause_idx = ClauseRef::default();
        }

        if clause_ref.is_learned() {
            self.stats.num_learned_clauses -= 1;
            self.stats.num_literals_in_learned_clauses -= len;
            self.learned_clauses.remove_clause(clause_ref);
        } else {
            self.stats.num_clauses -= 1;
            self.stats.num_literals_in_clauses -= len;
            self.clauses.remove_clause(clause_ref);
        }
    }

    /// Remove the satisfied clauses in the given container
    fn remove_satisfied_clauses(&mut self, is_learned: bool) {
        let clause_len = if is_learned {
            self.learned_clauses.len()
        } else {
            self.clauses.len()
        };

        for clause_idx in 0..clause_len {
            let clause_ref = ClauseRef::from_idx(clause_idx, is_learned);
            let is_clause_deleted = if is_learned {
                self.learned_clauses[clause_ref].is_empty()
            } else {
                self.clauses[clause_ref].is_empty()
            };

            // Skip deleted clauses
            if is_clause_deleted {
                continue;
            }

            // Check if clause already satisfied
            let is_clause_already_satisfied = if is_learned {
                self.is_clause_satisfied(&self.learned_clauses[clause_ref])
            } else {
                self.is_clause_satisfied(&self.clauses[clause_ref])
            };
            if is_clause_already_satisfied {
                // Remove clause
                self.detach_clause(clause_ref);
            } else {
                // Trim clause; first two literals cannot be true since otherwise
                // `isClauseSatisfied()` and cannot be false by invariant
                let clause = if is_learned {
                    &mut self.learned_clauses[clause_ref]
                } else {
                    &mut self.clauses[clause_ref]
                };
                debug_assert!(clause.len() > 1);
                debug_assert!(self.variable_values[clause[0].var()].is_unset());
                debug_assert!(self.variable_values[clause[1].var()].is_unset());
                let mut i = 2;
                while i < clause.len() {
                    if clause[i].is_false(&self.variable_values) {
                        clause.swap_remove(i);
                        i -= 1;
                    }
                    i += 1;
                }
            }
        }
    }

    /// Simplify by removing satisfied clauses
    fn simplify(&mut self) -> bool {
        // Only top-level simplifications
        debug_assert!(self.decision_level() == 0);

        // Check that top-level propagation does not produce a conflict
        if self.propagate().valid() {
            return false;
        }

        // Remove satisfied clauses
        self.remove_satisfied_clauses(true);
        self.remove_satisfied_clauses(false);

        // Update unset variables
        self.unset_variables.clear();
        for var in 0..self.variable_values.len() {
            if self.variable_values[var].is_unset() {
                self.unset_variables.push(var);
            }
        }
        self.unset_variables.shuffle(&mut self.random_gen);

        // Problem instance still satisfiable
        true
    }

    /// Checks whether the given clause is satisfied
    fn is_clause_satisfied(&self, clause: &Vec<Literal>) -> bool {
        for literal in clause {
            if literal.is_true(&self.variable_values) {
                return true;
            }
        }
        false
    }
}

/// Adding clauses to a solver
impl ClauseReceiver for Solver {
    fn create_variables(&mut self, num_variables: usize) {
        self.stats.num_variables = num_variables;
        self.variable_values
            .resize(num_variables, VariableValue::Unset);
        self.variable_polarity.resize(num_variables, false);
        self.variable_metadata
            .resize(num_variables, VariableMetadata::default());
        self.trail.reserve(num_variables + 1);
        self.unset_variables.reserve(num_variables);
        self.literals_watched_by
            .resize(num_variables * 2, Vec::new());
    }

    fn add_clause(&mut self, literals: &[Literal]) -> bool {
        debug_assert!(self.decision_level() == 0);
        debug_assert!(!literals.is_empty());

        // Copy literals and sort (positive and negative literals
        // of the same variable are consecutive)
        let mut copied_literals = Vec::from(literals);
        copied_literals.sort();

        // Check for satisfied clauses and duplicate literals
        let mut last_literal = Literal::default();
        let mut num_final_elems = 0;
        let mut i = 0;
        while i < copied_literals.len() {
            let curr_literal = copied_literals[i];
            let var = curr_literal.var();
            let polarity = curr_literal.polarity();
            debug_assert!(var < self.num_variables());
            i += 1;

            // Clause already satisfied
            if self.variable_values[var] == polarity {
                return true;
            }
            // `not A or A` is always true
            if curr_literal == !last_literal {
                return true;
            }
            // Literal false; no need to add
            if self.variable_values[var] == !polarity {
                continue;
            }
            // Duplicate literal, continue
            if curr_literal == last_literal {
                continue;
            }

            // Add literal to final output
            last_literal = curr_literal;
            copied_literals[num_final_elems] = curr_literal;
            num_final_elems += 1;
        }

        // Update clause size
        copied_literals.resize(num_final_elems, Literal::default());

        // If literals are empty, instance is UNSAT
        if copied_literals.is_empty() {
            return false;
        }

        // Add fact for next propagation if singleton
        if copied_literals.len() == 1 {
            self.assign_literal(copied_literals[0], ClauseRef::default());
            return !self.propagate().valid(); // Check conflicts
        }

        // Add clause
        self.attach_clause(copied_literals, false);
        true
    }
}
