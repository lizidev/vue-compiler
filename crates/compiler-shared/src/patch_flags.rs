use bitflags::{bitflags, bitflags_match};
use std::fmt::Display;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PatchFlags: i16 {
        /// Indicates an element with dynamic textContent (children fast path)
        const Text = 1;
        /// Indicates an element with dynamic class binding.
        const Class = 2;
        /// Indicates an element with props with dynamic keys. When keys change, a full
        /// diff is always needed to remove the old key. This flag is mutually
        /// exclusive with CLASS, STYLE and PROPS.
        const FullProps = 1 << 4;
        /// Indicates a fragment whose children order doesn't change.
        const StableFragment = 1 << 6;
        /// Indicates a fragment with keyed or partially keyed children
        const KeyedFragment = 1 << 7;
        /// Indicates a fragment with unkeyed children.
        const UnkeyedFragment = 1 << 8;
        /// Indicates a fragment that was created only because the user has placed
        /// comments at the root level of a template. This is a dev-only flag since
        /// comments are stripped in production.
        const DevRootFragment = 1 << 11;
    }
}

impl PatchFlags {
    pub fn as_str(&self) -> &'static str {
        bitflags_match!(self, {
            &Self::Text => "TEXT",
            &Self::Class => "CLASS",
            &Self::FullProps => "FULL_PROPS",
            &Self::StableFragment => "STABLE_FRAGMENT",
            &Self::DevRootFragment => "DEV_ROOT_FRAGMENT",
            _ => unreachable!()
        })
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

impl Display for PatchFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.bits())
    }
}

impl PartialEq<i16> for PatchFlags {
    fn eq(&self, other: &i16) -> bool {
        &self.bits() == other
    }
}

impl PartialOrd<i16> for PatchFlags {
    fn partial_cmp(&self, other: &i16) -> Option<std::cmp::Ordering> {
        self.bits().partial_cmp(other)
    }
}
