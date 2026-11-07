//! The errors of building a registry.

/// Why a [`HealthRegistry`](crate::HealthRegistry) couldn't be built. Running checks never
/// fails: a failing check is a result, not an error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HealthError {
    /// A setting is out of range.
    #[error("setting `{field}` {reason}")]
    InvalidSettings {
        /// The setting's field name, as a config file writes it.
        field: &'static str,
        /// What is wrong, and how to fix it.
        reason: String,
    },

    /// A check's name is empty, uses a character other than ASCII letters, digits, `-`, `_` and
    /// `.`, or is one the registry's own probe lines use.
    #[error(
        "check name `{0}` is not allowed: use ASCII letters, digits, `-`, `_` and `.`, and none of `startup`, `shutdown` or `poller`"
    )]
    InvalidName(String),

    /// Two checks have the same name.
    #[error("two checks are named `{0}`; give each a unique name")]
    DuplicateName(String),

    /// A check's options are out of range.
    #[error("check `{name}` {reason}")]
    InvalidCheck {
        /// The check's name.
        name: String,
        /// What is wrong, and how to fix it.
        reason: String,
    },

    /// An option meant for the last check added, such as `tags`, came before any check.
    #[error("`{0}` sets an option of the last check added, but no check was added before it")]
    NoCheck(&'static str),
}
