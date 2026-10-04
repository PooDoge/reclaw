//! The rules that make two spellings of a value the same value. Quiver applies these when it reads a file and again
//! when it writes one, and compares on the result, so a catalog entry and a library entry only match if both went
//! through them. They are reproduced exactly; the places where this port is deliberately different are noted.
use std::collections::HashSet;

/// Tags as the format keeps them: trimmed, lower case, no blanks, no repeats (the first spelling wins), in the order
/// given.
pub fn tags<I, S>(input: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = HashSet::new();
    input.into_iter().map(|t| t.as_ref().trim().to_lowercase()).filter(|t| !t.is_empty() && seen.insert(t.clone())).collect()
}

/// Whether every tag the catalog gives an app is also on the library's copy. Extra tags the user added are fine; a
/// catalog with no tags always passes.
pub fn contains_all_tags(local: &[String], catalog: &[String]) -> bool {
    let have: HashSet<&str> = local.iter().map(String::as_str).collect();
    catalog.iter().all(|t| have.contains(t.as_str()))
}

/// A character no file name may contain. Quiver asks the operating system, so what it accepts differs between Linux
/// and Windows; this uses the stricter set everywhere, so an entry that is fine here is fine on every system.
pub(crate) fn is_invalid_file_name_char(c: char) -> bool {
    c.is_control() || matches!(c, '"' | '<' | '>' | '|' | ':' | '*' | '?' | '/' | '\\')
}

/// One name from a `filesToAdd` list: trimmed, and not blank, `.` or `..`, and free of path separators and of
/// characters a file name cannot hold.
fn file_name(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty() && name != "." && name != ".." && !name.chars().any(is_invalid_file_name_char)).then(|| name.to_string())
}

/// The files an app's folder gets when it is installed (marker files such as `portable.txt`; they are created empty,
/// not supplied by the user). Invalid names are dropped; repeats (ignoring case) keep the first spelling and place.
pub fn files_to_add<I, S>(input: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = HashSet::new();
    input.into_iter().filter_map(|n| file_name(n.as_ref())).filter(|n| seen.insert(n.to_uppercase())).collect()
}

/// Two `filesToAdd` lists are the same when they read the same, ignoring case: order matters, unlike for mod sources.
pub fn same_files(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_uppercase() == y.to_uppercase())
}

/// A release asset filter, trimmed, with a blank one meaning none.
pub fn asset_filter(text: Option<&str>) -> Option<String> {
    text.map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

/// Whether a release asset's name passes the filter: a case-insensitive "contains". No filter passes everything.
pub fn asset_matches(filter: Option<&str>, asset_name: &str) -> bool {
    filter.is_none_or(|f| asset_name.to_lowercase().contains(&f.to_lowercase()))
}

/// Two strings are the same key: equal ignoring case, the way every identity comparison in the format works.
pub(crate) fn same_key(a: &str, b: &str) -> bool {
    a.to_uppercase() == b.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_trimmed_lowered_and_deduplicated_keeping_the_first_place() {
        assert_eq!(tags([" Recomp ", "N64", "recomp", "", "  ", "Zelda", "n64"]), ["recomp", "n64", "zelda"]);
        assert!(tags(Vec::<String>::new()).is_empty());
    }

    #[test]
    fn extra_local_tags_are_fine_but_a_missing_catalog_tag_is_a_change() {
        let local = tags(["recomp", "n64", "mine"]);
        assert!(contains_all_tags(&local, &tags(["recomp", "n64"])));
        assert!(contains_all_tags(&local, &[]), "a catalog with no tags always passes");
        assert!(!contains_all_tags(&local, &tags(["recomp", "zelda"])));
        assert!(!contains_all_tags(&[], &tags(["recomp"])), "a catalog with tags against none fails");
    }

    #[test]
    fn files_to_add_keeps_plain_file_names_only() {
        assert_eq!(
            files_to_add(["portable.txt", " Portable.TXT ", "a/b", "..", ".", "", "x:y", "ok.cfg", "back\\slash", "tab\tname"]),
            ["portable.txt", "ok.cfg"]
        );
    }

    #[test]
    fn files_to_add_compares_in_order_and_ignoring_case() {
        let a = files_to_add(["portable.txt", "x.cfg"]);
        assert!(same_files(&a, &files_to_add(["PORTABLE.txt", "X.cfg"])));
        assert!(!same_files(&a, &files_to_add(["x.cfg", "portable.txt"])), "order matters");
    }

    #[test]
    fn an_asset_filter_is_a_trimmed_case_insensitive_contains() {
        assert_eq!(asset_filter(Some("  FireRed ")).as_deref(), Some("FireRed"));
        assert_eq!(asset_filter(Some("   ")), None);
        assert!(asset_matches(Some("firered"), "Pokemon-FireRed-linux.zip"));
        assert!(!asset_matches(Some("LeafGreen"), "Pokemon-FireRed-linux.zip"));
        assert!(asset_matches(None, "anything"));
    }
}
