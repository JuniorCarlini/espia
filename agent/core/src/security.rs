//! Small, security-relevant helpers shared across the agent's binaries —
//! deliberately tiny and dependency-free rather than pulling in a crate
//! like `subtle` for one function.

/// A plain `==` on a secret would short-circuit at the first mismatched
/// byte, which in theory leaks how many leading bytes of a guess were
/// right through response timing. This always compares every byte.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn matches_equal_slices() {
        assert!(constant_time_eq(b"a-real-token", b"a-real-token"));
    }

    #[test]
    fn rejects_different_content() {
        assert!(!constant_time_eq(b"a-real-token", b"a-fake-token"));
    }

    #[test]
    fn rejects_different_length() {
        assert!(!constant_time_eq(b"short", b"a-much-longer-value"));
    }
}
