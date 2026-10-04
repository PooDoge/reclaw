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
