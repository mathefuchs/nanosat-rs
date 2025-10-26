use std::ops::Not;

use crate::solver::variable::{Variable, VariableValue};

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
