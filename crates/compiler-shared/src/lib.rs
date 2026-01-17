#[derive(Debug, PartialEq, Clone)]
pub enum PatchFlags {
    /// Indicates an element with dynamic textContent (children fast path)
    Text = 1,
    /// Indicates an element with props with dynamic keys. When keys change, a full
    /// diff is always needed to remove the old key. This flag is mutually
    /// exclusive with CLASS, STYLE and PROPS.
    FullProps = 1 << 4,
    /// Indicates a fragment whose children order doesn't change.
    StableFragment = 1 << 6,
    /// Indicates a fragment that was created only because the user has placed
    /// comments at the root level of a template. This is a dev-only flag since
    /// comments are stripped in production.
    DevRootFragment = 1 << 11,
}

impl PatchFlags {
    pub fn to_i16(&self) -> i16 {
        self.clone() as i16
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::FullProps => "FULL_PROPS",
            Self::StableFragment => "STABLE_FRAGMENT",
            Self::DevRootFragment => "DEV_ROOT_FRAGMENT",
        }
    }

    pub fn keys() -> Vec<PatchFlags> {
        vec![
            Self::Text,
            Self::FullProps,
            Self::StableFragment,
            Self::DevRootFragment,
        ]
    }
}
