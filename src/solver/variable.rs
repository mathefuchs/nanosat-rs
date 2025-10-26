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
