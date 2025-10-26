use crate::solver::{
    literal::Literal,
    variable::{Variable, VariableValue},
};

/// Clause reference type
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClauseRef {
    /// Even indices are original clauses; odd indices are learned clauses
    repr: usize,
}

impl ClauseRef {
    /// Invalid reference
    const INVALID: usize = usize::MAX;

    /// New clause reference
    #[must_use]
    pub const fn from_idx(idx: usize, is_learned: bool) -> Self {
        ClauseRef {
            repr: 2 * idx + is_learned as Variable,
        }
    }

    /// Index
    #[must_use]
    pub const fn idx(&self) -> Variable {
        self.repr >> 1
    }
    /// Whether clause is learned
    #[must_use]
    pub const fn is_learned(&self) -> bool {
        (self.repr & 1) != 0
    }
    /// Whether is valid
    #[must_use]
    pub const fn valid(&self) -> bool {
        self.repr != Self::INVALID
    }
}

impl Default for ClauseRef {
    fn default() -> Self {
        Self {
            repr: Self::INVALID,
        }
    }
}

/// Literal is watched by `clause_idx`.
/// If `Watch::blocker` is satisfied, clause is not required to be inspected
#[derive(Clone, Copy, Eq, Debug, Default)]
pub struct Watch {
    pub clause_ref: ClauseRef,
    pub blocker: Literal,
}

impl Watch {
    /// New watch
    #[must_use]
    pub const fn from_ref_and_blocker(clause_ref: ClauseRef, blocker: Literal) -> Self {
        Watch {
            clause_ref,
            blocker,
        }
    }
}

/// Watches are equal if at least the clause reference matches
impl PartialEq for Watch {
    fn eq(&self, other: &Self) -> bool {
        self.clause_ref == other.clause_ref
    }
}

/// Store metadata for a variable
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct VariableMetadata {
    /// Link to a clause
    pub reason_clause_idx: ClauseRef,
    /// The associated decision level for a variable assignment
    pub decision_level: usize,
}

/// Class managing the creation, deletion, and access of clauses
#[derive(Clone, Default)]
pub struct Clauses<const IS_LEARNED: bool> {
    /// Stores all clauses
    container: Vec<Vec<Literal>>,
    /// Stores indices of empty vectors
    free_indices: Vec<usize>,
}

impl<const IS_LEARNED: bool> Clauses<IS_LEARNED> {
    /// Size
    #[must_use]
    pub const fn len(&self) -> usize {
        self.container.len()
    }

    /// Is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Move clause into container
    pub fn add_clause(&mut self, literals: Vec<Literal>, is_learned: bool) -> ClauseRef {
        if let Some(idx) = self.free_indices.pop() {
            // Use free slot if any
            self.container[idx] = literals;
            ClauseRef::from_idx(idx, is_learned)
        } else {
            // Append at the end
            let idx = self.container.len();
            self.container.push(literals);
            ClauseRef::from_idx(idx, is_learned)
        }
    }

    /// Remove clause
    pub fn remove_clause(&mut self, clause_ref: ClauseRef) {
        debug_assert_eq!(clause_ref.is_learned(), IS_LEARNED);
        let idx = clause_ref.idx();
        if idx == self.container.len() - 1 {
            self.container.pop();
        } else {
            self.container[idx].clear();
            self.free_indices.push(clause_ref.idx());
        }
    }

    /// Whether clause is satisfied
    #[must_use]
    pub fn is_clause_satisfied(
        &self,
        clause_ref: ClauseRef,
        variable_values: &[VariableValue],
    ) -> bool {
        debug_assert_eq!(clause_ref.is_learned(), IS_LEARNED);
        for &literal in &self[clause_ref] {
            if literal.is_true(variable_values) {
                return true;
            }
        }
        false
    }
}

/// Clause at given index
impl<const IS_LEARNED: bool> std::ops::Index<ClauseRef> for Clauses<IS_LEARNED> {
    type Output = Vec<Literal>;

    fn index(&self, index: ClauseRef) -> &Self::Output {
        debug_assert!(index.valid());
        &self.container[index.idx()]
    }
}
impl<const IS_LEARNED: bool> std::ops::IndexMut<ClauseRef> for Clauses<IS_LEARNED> {
    fn index_mut(&mut self, index: ClauseRef) -> &mut Self::Output {
        debug_assert!(index.valid());
        &mut self.container[index.idx()]
    }
}
