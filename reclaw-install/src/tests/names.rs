use crate::names::*;

#[test]
fn ios_is_recognised_by_word_not_by_substring() {
    assert!(is_ios("game-ios.zip"));
    assert!(is_ios("game_ipad_build.zip"));
    assert!(is_ios("game.ipa"));
    assert!(is_ios("game.ipa.zip"));
    assert!(!is_ios("bios-dump.zip"), "BIOS is not iOS");
    assert!(!is_ios("studios-linux.tar.gz"), "nor Studios");
    assert!(!is_ios("game.ipads"), "a longer word");
    assert!(!is_ios("ipadder-game.zip"));
}

#[test]
fn dedicated_device_packages_are_recognised() {
    assert!(is_dedicated_device("game-xbox.zip"));
    assert!(is_dedicated_device("game.portmaster.zip"));
    assert!(is_dedicated_device("game-stockos2.zip"));
    assert!(is_dedicated_device("game-rg35xx.zip"));
    assert!(is_dedicated_device("game_rg353.zip"));
    assert!(!is_dedicated_device("game-xboxlive.zip"), "a longer word is a different word");
    assert!(!is_dedicated_device("rg5-game.zip"), "two digits at least");
    assert!(!is_dedicated_device("graph-game.zip"));
}

#[test]
fn debug_symbols_are_pieces_of_the_name() {
    assert!(is_debug_symbols("game-windows.pdb.zip"));
    assert!(is_debug_symbols("game_symbols.zip"));
    assert!(is_debug_symbols("game-debugsymbols.7z"));
    assert!(!is_debug_symbols("symbolsmith-windows.zip"), "a longer word");
}

#[test]
fn auxiliary_files_are_not_the_program() {
    for name in [
        "release.json",
        "game.sha256",
        "game.zip.sha256.txt",
        "game.zip.sig",
        "game.zip.asc",
        "game.minisig",
        "checksums.txt",
        "game-sha256sums.txt",
        "SHA1SUMS",
        "md5sums.txt",
        "game-src.zip",
        "game_source.tar.gz",
        "game-source-code.zip",
        "notices.7z",
        "game.pdb",
        "Game.dSYM.zip",
        "game-debug.symbols.zip",
    ] {
        assert!(is_auxiliary(&name.to_ascii_lowercase()), "{name}");
    }
}

#[test]
fn programs_whose_names_only_look_like_those_are_kept() {
    for name in [
        "resource-linux.zip",
        "sourcegame-windows.zip",
        "Starship-Linux.AppImage",
        "game-windows.zip",
        "resources.7z",
        "ssrc.zip",
        "game-linux.tar.gz",
    ] {
        assert!(!is_auxiliary(&name.to_ascii_lowercase()), "{name}");
    }
}

#[test]
fn mac_and_win_are_words_of_their_own() {
    assert!(has_mac_word("game-mac.zip"));
    assert!(has_mac_word("game mac build.zip"));
    assert!(!has_mac_word("macro-game.zip"));
    assert!(!has_mac_word("imac-game.zip"));
    assert!(has_win_token("game_win_x64.zip"));
    assert!(has_win_token("game-win64.zip"));
    assert!(has_win_token("win-game.zip"));
    assert!(!has_win_token("darwin-game.zip"), "win inside a word");
    assert!(!has_win_token("twin-peaks.zip"));
    assert!(!has_win_token("game.zip"));
}
