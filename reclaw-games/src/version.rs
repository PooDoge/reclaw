//! Telling release tags apart: `v1.4.2` and `1.4.2` are the same release, `1.4.2-beta` is not `1.4.2`. A port of Quiver's
//! `ReleaseVersionIdentity`, so an update is offered under the same rules.

/// The numbers of a tag, as `System.Version` would hold them: three to four whole numbers. A tag that cannot be one
/// (a number too large for 32 bits) has no version, and nothing is newer or equal to it.
fn parse(version: &str) -> Option<[u32; 4]> {
    let normalized = normalize(version);
    let mut parts = [0u32; 4];
    for (slot, piece) in parts.iter_mut().zip(normalized.split('.')) {
        *slot = piece.parse::<i32>().ok()?.try_into().ok()?;
    }
    Some(parts)
}

fn strip_build_metadata(version: &str) -> &str {
    version.split_once('+').map_or(version, |(before, _)| before)
}

fn identity(version: &str) -> &str {
    strip_build_metadata(version.trim().trim_start_matches(['v', 'V']))
}

fn has_label(version: &str) -> bool {
    identity(version).contains(['-', ' ', '\t'])
}

/// `v1.4-rc1` becomes `1.4.0`: the leading numbers, padded to three, at most four.
pub fn normalize(version: &str) -> String {
    if version.trim().is_empty() {
        return "0.0.0".to_string();
    }
    let text = identity(version);
    let text = text.split(['-', ' ', '\t']).next().unwrap_or_default();
    let mut segments: Vec<String> = Vec::new();
    for part in text.split('.').filter(|p| !p.is_empty()) {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            break;
        }
        segments.push(digits);
    }
    while segments.len() < 3 {
        segments.push("0".to_string());
    }
    segments.truncate(4);
    segments.join(".")
}

/// Whether `candidate` is a higher version than `baseline` (labels such as `-beta` are ignored).
pub fn is_newer(candidate: &str, baseline: &str) -> bool {
    match (parse(candidate), parse(baseline)) {
        (Some(c), Some(b)) => c > b,
        _ => false,
    }
}

/// Whether two tags name the same release. A tag with a label (`-beta`) only equals a tag spelled the same.
pub fn equivalent(first: &str, second: &str) -> bool {
    if first.trim().is_empty() || second.trim().is_empty() {
        return false;
    }
    if identity(first).eq_ignore_ascii_case(identity(second)) {
        return true;
    }
    if has_label(first) || has_label(second) {
        return false;
    }
    matches!((parse(first), parse(second)), (Some(a), Some(b)) if a == b)
}

/// A tag with a label (`1.2.0-rc1`) is a pre-release by its name.
pub fn looks_like_prerelease(version: &str) -> bool {
    !version.trim().is_empty() && has_label(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_reduced_to_their_numbers() {
        assert_eq!(normalize("v1.4.2"), "1.4.2");
        assert_eq!(normalize("V2"), "2.0.0");
        assert_eq!(normalize("1.2.3.4.5"), "1.2.3.4");
        assert_eq!(normalize("1.2-rc1"), "1.2.0");
        assert_eq!(normalize("2024.05.01+build7"), "2024.05.01");
        assert_eq!(normalize("v1.x"), "1.0.0", "a piece with no number ends the version");
        assert_eq!(normalize(""), "0.0.0");
        assert_eq!(normalize("nightly"), "0.0.0");
    }

    #[test]
    fn newer_means_a_higher_number_and_nothing_else() {
        assert!(is_newer("v1.4.3", "1.4.2"));
        assert!(is_newer("1.10.0", "1.9.9"), "numbers, not text");
        assert!(!is_newer("1.4.2", "v1.4.2"));
        assert!(!is_newer("1.4.1", "1.4.2"));
        assert!(!is_newer("1.4.2-beta", "1.4.2"), "a label does not make it newer");
        assert!(!is_newer("99999999999.0.0", "1.0.0"), "a number that is not a version number is never newer");
    }

    #[test]
    fn the_same_release_may_be_spelled_two_ways() {
        assert!(equivalent("v1.4.2", "1.4.2"));
        assert!(equivalent("V1.4.2", "v1.4.2+abc"));
        assert!(equivalent("1.4", "1.4.0"));
        assert!(equivalent("Nightly", "nightly"), "the same word is the same release");
    }

    #[test]
    fn a_labelled_tag_is_only_itself() {
        assert!(!equivalent("1.4.2-beta", "1.4.2"));
        assert!(!equivalent("1.4.2", "1.4.2-beta"));
        assert!(equivalent("1.4.2-beta", "v1.4.2-beta"));
        assert!(!equivalent("", "1.0.0"));
        assert!(!equivalent("1.0.0", "  "));
    }

    #[test]
    fn prerelease_by_name() {
        assert!(looks_like_prerelease("1.2.0-rc1"));
        assert!(looks_like_prerelease("v2.0 beta"));
        assert!(!looks_like_prerelease("v1.2.0"));
        assert!(!looks_like_prerelease(""));
    }
}
