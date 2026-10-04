use super::*;

#[test]
fn every_system_is_listed_once_in_the_sort_order() {
    for (i, platform) in ALL.iter().enumerate() {
        assert_eq!(platform.rank(), i);
    }
    let mut seen = std::collections::HashSet::new();
    assert!(ALL.iter().all(|p| seen.insert(*p)), "no system appears twice");
}

#[test]
fn systems_are_grouped_by_maker_in_release_order() {
    let makers: Vec<Maker> = ALL.iter().map(|p| p.maker()).collect();
    // Each maker's systems are one unbroken run, so sorting by system also groups by maker.
    let mut runs = makers.clone();
    runs.dedup();
    let mut distinct = runs.clone();
    distinct.sort_by_key(|m| m.label());
    distinct.dedup();
    assert_eq!(runs.len(), distinct.len(), "{makers:?}");
    assert!(Platform::N64.rank() < Platform::GameCube.rank(), "older first within a maker");
    assert!(Platform::Gba.rank() < Platform::Ds.rank());
    assert_eq!(*ALL.last().expect("non-empty"), Platform::Other, "the catch-all sorts last");
}

#[test]
fn names_marks_and_kinds() {
    assert_eq!((Platform::N64.label(), Platform::N64.short()), ("Nintendo 64", "N64"));
    assert_eq!(Platform::Gba.kind(), SystemKind::Handheld);
    assert_eq!(Platform::Ps2.kind(), SystemKind::Console);
    assert_eq!(Platform::Ps2.maker().label(), "Sony");
    assert!(Platform::N64.is_known() && !Platform::Other.is_known());
    assert!(ALL.iter().filter(|p| p.is_known()).all(|p| p.short().len() <= 4), "marks fit on a card");
}

#[test]
fn tags_map_to_systems_ignoring_case_and_spacing() {
    for (tag, expected) in [
        ("n64", Platform::N64),
        ("N64", Platform::N64),
        ("GameCube", Platform::GameCube),
        ("game boy advance", Platform::Gba),
        ("Mega-Drive", Platform::Genesis),
        ("PSX", Platform::Ps1),
        ("ps2", Platform::Ps2),
        ("dreamcast", Platform::Dreamcast),
    ] {
        assert_eq!(Platform::from_tag(tag), Some(expected), "{tag}");
    }
    for not_a_system in ["mods", "favorite", "recomp", ""] {
        assert_eq!(Platform::from_tag(not_a_system), None, "{not_a_system:?}");
    }
}

#[test]
fn every_short_mark_and_label_maps_back_to_its_system() {
    for platform in ALL.iter().filter(|p| p.is_known()) {
        assert_eq!(Platform::from_tag(platform.short()), Some(*platform), "{platform:?} by its mark");
    }
}

#[test]
fn saved_names_stay_what_older_files_wrote() {
    // The first four systems were saved before the others existed; their names must not change.
    for (platform, name) in [(Platform::N64, "n64"), (Platform::Ps2, "ps2"), (Platform::Gba, "gba"), (Platform::Other, "other")] {
        assert_eq!(serde_json::to_string(&platform).expect("serializes"), format!("\"{name}\""));
        assert_eq!(serde_json::from_str::<Platform>(&format!("\"{name}\"")).expect("reads back"), platform);
    }
    assert_eq!(serde_json::to_string(&Platform::GameCube).expect("serializes"), "\"game_cube\"");
}

#[test]
fn a_whole_tag_list_is_weighed_so_a_brand_tag_does_not_hide_the_system() {
    // The real catalog tags a PlayStation 2 game "playstation" before "ps2"; the first tag alone says PS1.
    assert_eq!(Platform::from_tags(["recomp", "playstation", "ps2", "playstation 2"]), Some(Platform::Ps2));
    assert_eq!(Platform::from_tags(["xbox", "x360", "xbox 360"]), Some(Platform::Xbox360), "an Xbox 360 game is not an original Xbox game");
    assert_eq!(Platform::from_tags(["xbox"]), Some(Platform::Xbox), "a brand tag alone still gives the system the brand began with");
    assert_eq!(Platform::from_tags(["playstation"]), Some(Platform::Ps1));
    assert_eq!(Platform::from_tags(["xbox", "original xbox"]), Some(Platform::Xbox));
}

#[test]
fn where_a_game_runs_today_never_beats_where_it_came_from() {
    assert_eq!(Platform::from_tags(["recomp", "pc", "n64"]), Some(Platform::N64), "an N64 game ported to PC is still an N64 game");
    assert_eq!(Platform::from_tags(["recreation", "pc", "club penguin"]), Some(Platform::Pc), "a PC game is");
    assert_eq!(Platform::from_tags(["brew", "mobile"]), Some(Platform::Mobile));
    assert_eq!(Platform::from_tags(["playstation", "pc"]), Some(Platform::Ps1), "a brand tag still beats a host tag");
}

#[test]
fn equally_strong_tags_go_to_the_first_listed_and_no_system_tag_is_none() {
    assert_eq!(Platform::from_tags(["gba", "n64"]), Some(Platform::Gba));
    assert_eq!(Platform::from_tags(["n64", "gba"]), Some(Platform::N64));
    assert_eq!(Platform::from_tags(["recomp", "mario", "favorite"]), None);
    assert_eq!(Platform::from_tags(Vec::<String>::new()), None);
}

#[test]
fn the_tags_the_real_catalog_uses_map_to_their_systems() {
    for (tag, expected) in [
        ("super nintendo entertainment system", Platform::Snes),
        ("nintendo entertainment system", Platform::Nes),
        ("nintendo 64", Platform::N64),
        ("game boy color", Platform::GameBoy),
        ("nintendo ds", Platform::Ds),
        ("3ds", Platform::N3ds),
        ("wii u", Platform::WiiU),
        ("sega mega drive", Platform::Genesis),
        ("sega genesis", Platform::Genesis),
        ("smd", Platform::Genesis),
        ("playstation portable", Platform::Psp),
        ("arcade", Platform::Arcade),
        ("pc", Platform::Pc),
        ("mobile", Platform::Mobile),
        ("brew", Platform::Mobile),
        ("x360", Platform::Xbox360),
    ] {
        assert_eq!(Platform::from_tag(tag), Some(expected), "{tag}");
    }
    for brand_only in ["nintendo", "sega", "sony", "recomp", "recompilation", "decompilation"] {
        assert_eq!(Platform::from_tag(brand_only), None, "{brand_only} names a maker or a kind of project, not a system");
    }
}

#[test]
fn systems_added_later_keep_their_own_saved_names() {
    for (platform, name) in [
        (Platform::Xbox360, "xbox360"),
        (Platform::WiiU, "wii_u"),
        (Platform::N3ds, "n3ds"),
        (Platform::Pc, "pc"),
        (Platform::Mobile, "mobile"),
        (Platform::Arcade, "arcade"),
    ] {
        assert_eq!(serde_json::to_string(&platform).expect("serializes"), format!("\"{name}\""));
        assert_eq!(serde_json::from_str::<Platform>(&format!("\"{name}\"")).expect("reads back"), platform);
    }
    assert_eq!(Platform::Pc.kind(), SystemKind::Computer);
    assert_eq!(Platform::N3ds.kind(), SystemKind::Handheld);
    assert_eq!(Platform::Xbox360.maker(), Maker::Microsoft);
}
