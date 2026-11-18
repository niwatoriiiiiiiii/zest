const OPENINGS_RAW: &str = include_str!("../../../examples/openings.txt");

/// Returns all non-empty opening lines from `examples/openings.txt`.
pub fn all_openings() -> impl Iterator<Item = &'static str> {
    OPENINGS_RAW
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
}

/// Returns the total number of precomputed balanced openings.
#[must_use]
pub fn opening_count() -> usize {
    all_openings().count()
}

/// Returns the whitespace-joined UCI moves for the opening at `index`.
#[must_use]
pub fn opening_at(index: usize) -> Option<&'static str> {
    all_openings().nth(index)
}

/// Returns `true` if `prefix` matches the beginning of at least one precomputed opening line.
#[must_use]
pub fn is_book_prefix(prefix: &[&str]) -> bool {
    if prefix.is_empty() {
        return false;
    }
    all_openings().any(|line| {
        let mut iter = line.split_whitespace();
        prefix.iter().all(|mv| iter.next() == Some(*mv))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openings_load_and_match_prefixes() {
        assert_eq!(opening_count(), 165);
        assert!(opening_at(0).is_some());
        assert!(is_book_prefix(&["a2a3", "b4a3"]));
        assert!(!is_book_prefix(&["e2e4", "e4e5"]));
    }
}
