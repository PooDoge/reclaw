//! Small tests on a file's name. Quiver writes these as regular expressions; they are written out here (the patterns are
//! few and simple, and the program then needs no regex engine). Every function takes the name **lower-cased**.

/// Whether `word` appears in `name` with a non-alphanumeric character (or an end) on each side.
fn has_word(name: &str, word: &str) -> bool {
    tokens(name).any(|token| token == word)
}

/// The maximal runs of letters and digits.
fn tokens(name: &str) -> impl Iterator<Item = &str> {
    name.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty())
}

/// The pieces between `.`, `_` and `-`.
fn segments(name: &str) -> impl Iterator<Item = &str> {
    name.split(['.', '_', '-']).filter(|s| !s.is_empty())
}

/// `ios`, `ipad`... as words, or an `.ipa` file. An iOS build is a known platform that is not ours, not an unknown archive.
pub fn is_ios(name: &str) -> bool {
    const WORDS: [&str; 6] = ["ios", "ipados", "iphone", "ipad", "iphoneos", "iphonesimulator"];
    WORDS.iter().any(|w| has_word(name, w))
        || name.match_indices(".ipa").any(|(at, m)| matches!(name[at + m.len()..].chars().next(), None | Some('.' | '_' | '-')))
}

/// Packages made for a console or a dedicated handheld's own system: never a generic desktop archive.
pub fn is_dedicated_device(name: &str) -> bool {
    tokens(name).any(|t| {
        t == "xbox"
            || t == "portmaster"
            || t.strip_prefix("stockos").is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_digit()))
            || t.strip_prefix("rg").is_some_and(|rest| rest.bytes().take_while(u8::is_ascii_digit).count() >= 2)
    })
}

/// Debug symbols: downloadable, but not a build anyone can run.
pub fn is_debug_symbols(name: &str) -> bool {
    segments(name).any(|s| matches!(s, "pdb" | "symbols" | "debugsymbols"))
}

/// Not the program: metadata, checksums, signatures, symbols, source code and notices.
pub fn is_auxiliary(name: &str) -> bool {
    name.ends_with(".json") || is_source_or_notice_archive(name) || has_checksum_extension(name) || has_auxiliary_segment(name)
}

fn is_source_or_notice_archive(name: &str) -> bool {
    const EXTENSIONS: [&str; 11] = [".zip", ".7z", ".rar", ".tar", ".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tgz", ".tbz2", ".txz"];
    let is_sep = |c: char| matches!(c, '.' | '_' | '-') || c.is_whitespace();
    // The label must be a whole piece of the name: `game-src.zip` is a source archive, `Resource.zip` is not.
    let ends_with_label =
        |stem: &str, label: &str| stem.strip_suffix(label).is_some_and(|before| before.chars().next_back().is_none_or(is_sep));
    EXTENSIONS.iter().filter_map(|ext| name.strip_suffix(ext)).any(|stem| {
        ["notice", "notices", "source", "sources", "src"].iter().any(|label| ends_with_label(stem, label))
            || stem.strip_suffix("code").and_then(|rest| rest.strip_suffix(is_sep)).is_some_and(|rest| ends_with_label(rest, "source"))
    })
}

/// `.sha256`, `.md5`, `.sig`, `.asc`, `.minisig`, `.signature`, `.debug`, each optionally followed by `.txt`.
fn has_checksum_extension(name: &str) -> bool {
    let name = name.strip_suffix(".txt").unwrap_or(name);
    let Some((_, extension)) = name.rsplit_once('.') else { return false };
    matches!(
        extension,
        "sha" | "sha1" | "sha224" | "sha256" | "sha384" | "sha512" | "md5" | "sig" | "asc" | "minisig" | "signature" | "debug"
    )
}

fn has_auxiliary_segment(name: &str) -> bool {
    segments(name).any(|s| {
        matches!(s, "checksum" | "checksums" | "md5sum" | "md5sums" | "pdb" | "dsym" | "symbols" | "debugsymbols")
            || s.strip_prefix("sha").is_some_and(|rest| {
                let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
                matches!(rest, "sum" | "sums")
            })
    })
}

/// `mac` as a word of its own (not `macro`, not `Imac`).
pub fn has_mac_word(name: &str) -> bool {
    has_word(name, "mac")
}

/// `win` followed by a separator or a digit, as in `game_win_x64` or `win64`; or at the start (`win-game`).
pub fn has_win_token(name: &str) -> bool {
    let bytes = name.as_bytes();
    name.match_indices("win").any(|(at, _)| {
        let before = at.checked_sub(1).and_then(|i| bytes.get(i)).copied();
        let after = bytes.get(at + 3).copied();
        let sep = |b: u8| b == b'_' || b == b'-';
        (before.is_some_and(sep) && after.is_some_and(|a| sep(a) || a.is_ascii_digit())) || (at == 0 && after.is_some_and(sep))
    })
}
