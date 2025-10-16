use std::ops::Not;

/// Variable type
pub type Variable = usize;

/// Variable value type
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VariableValue {
    False = 0,
    True = 1,
    Unset = 2,
}

impl VariableValue {
    /// Variable from bool
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }

    /// Whether is false
    #[must_use]
    pub fn is_false(self) -> bool {
        self == VariableValue::False
    }
    /// Whether is true
    #[must_use]
    pub fn is_true(self) -> bool {
        self == VariableValue::True
    }
    /// Whether is unset
    #[must_use]
    pub fn is_unset(self) -> bool {
        self == VariableValue::Unset
    }
}

impl Default for VariableValue {
    fn default() -> Self {
        Self::Unset
    }
}

impl PartialEq<bool> for VariableValue {
    fn eq(&self, other: &bool) -> bool {
        *self == VariableValue::from_bool(*other)
    }
}

/// Literal type
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Literal {
    /// Literal representation; positive and negative literals are consecutive;
    /// `[    0, 1,     2, 3,     4, 5, ...]`
    /// `[not 0, 0, not 1, 1, not 2, 2, ...]`
    repr: Variable,
}

impl Literal {
    /// Invalid literal
    const INVALID: Variable = Variable::MAX;

    /// Constructs a literal from a variable (0..n)
    /// with polarity (+ : true, - : false)
    #[must_use]
    pub const fn from_var_with_polarity(variable: Variable, polarity: bool) -> Self {
        Literal {
            repr: 2 * variable + polarity as Variable,
        }
    }

    /// Access representation for indexing
    #[must_use]
    pub const fn repr(&self) -> usize {
        self.repr
    }
    /// The variable used in the literal for indexing purposes
    #[must_use]
    pub const fn var(&self) -> usize {
        self.repr >> 1
    }
    /// The polarity of the literal (+ : true, - : false)
    #[must_use]
    pub const fn polarity(&self) -> bool {
        (self.repr & 1) != 0
    }
    /// Whether is valid
    #[must_use]
    pub const fn valid(&self) -> bool {
        self.repr != Self::INVALID
    }
    /// Whether literal evaluates to true
    #[must_use]
    pub fn is_true(&self, model: &[VariableValue]) -> bool {
        model[self.var()] == self.polarity()
    }
    /// Whether literal evaluates to false
    #[must_use]
    pub fn is_false(&self, model: &[VariableValue]) -> bool {
        model[self.var()] == !self.polarity()
    }
    /// Whether literal evaluates to undetermined value
    #[must_use]
    pub fn is_unset(&self, model: &[VariableValue]) -> bool {
        model[self.var()] == VariableValue::Unset
    }
}

impl Default for Literal {
    fn default() -> Self {
        Self {
            repr: Self::INVALID,
        }
    }
}

/// Implement negation for literals
impl Not for Literal {
    type Output = Self;

    fn not(self) -> Self::Output {
        Literal {
            repr: self.repr ^ 1,
        }
    }
}

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
pub struct Clauses {
    /// Stores all clauses
    container: Vec<Vec<Literal>>,
    /// Stores indices of empty vectors
    free_indices: Vec<usize>,
}

impl Clauses {
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
        let idx = clause_ref.idx();
        if idx == self.container.len() - 1 {
            self.container.pop();
        } else {
            self.container[idx].clear();
            self.free_indices.push(clause_ref.idx());
        }
    }
}

/// Clause at given index
impl std::ops::Index<ClauseRef> for Clauses {
    type Output = Vec<Literal>;

    fn index(&self, index: ClauseRef) -> &Self::Output {
        debug_assert!(index.valid());
        &self.container[index.idx()]
    }
}
impl std::ops::IndexMut<ClauseRef> for Clauses {
    fn index_mut(&mut self, index: ClauseRef) -> &mut Self::Output {
        debug_assert!(index.valid());
        &mut self.container[index.idx()]
    }
}
