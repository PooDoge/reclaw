use crate::{
    activity::Changelog,
    notices::{Notice, NoticeKind, Notices},
};

#[test]
fn the_newest_notice_is_on_top_and_lists_are_newest_first() {
    let mut n = Notices::default();
    let a = n.push(Notice::update_available(1, "A"));
    let b = n.push(Notice::update_available(2, "B"));
    assert_eq!(n.top().map(|x| x.id), Some(b));
    assert_eq!(n.all().map(|x| x.id).collect::<Vec<_>>(), vec![b, a]);
}

#[test]
fn a_repeat_about_the_same_game_replaces_instead_of_stacking() {
    let mut n = Notices::default();
    n.push(Notice::download_failed(1, "A", "timeout", &[]));
    n.push(Notice::download_failed(1, "A", "timeout again", &[]));
    assert_eq!(n.len(), 1);
    assert_eq!(n.top().map(|x| x.body.as_str()), Some("timeout again"));
    n.push(Notice::download_failed(2, "B", "x", &[]));
    assert_eq!(n.len(), 2, "another game is another notice");
}

#[test]
fn a_finished_update_retires_the_update_ready_notice_for_that_game() {
    let mut n = Notices::default();
    n.push(Notice::update_available(1, "A"));
    n.push(Notice::update_available(2, "B"));
    n.push(Notice::update_finished(1, "A", None));
    let kinds: Vec<(NoticeKind, Option<u32>)> = n.all().map(|x| (x.kind, x.game_id)).collect();
    assert_eq!(kinds, vec![(NoticeKind::UpdateFinished, Some(1)), (NoticeKind::UpdateAvailable, Some(2))]);
}

#[test]
fn a_finished_update_carries_its_release_notes_as_details() {
    let changelog = Changelog {
        from: "v1".into(),
        to: "v2".into(),
        notes: "Fixes saves\n\n  Faster loads  \n".into(),
        url: Some("https://x/y".into()),
    };
    let notice = Notice::update_finished(3, "Game", Some(&changelog));
    assert_eq!(
        (notice.body.as_str(), notice.details.clone(), notice.url.clone()),
        ("v1 to v2", vec!["Fixes saves".to_string(), "Faster loads".to_string()], Some("https://x/y".to_string()))
    );
}

#[test]
fn dismissing_one_all_or_a_games_notices() {
    let mut n = Notices::default();
    let a = n.push(Notice::update_available(1, "A"));
    n.push(Notice::download_failed(1, "A", "x", &[]));
    n.push(Notice::update_available(2, "B"));
    assert!(n.dismiss(a) && !n.dismiss(a));
    n.dismiss_game(1);
    assert_eq!(n.len(), 1);
    assert_eq!(n.dismiss_all(), 1);
    assert!(n.is_empty() && n.top().is_none());
}

#[test]
fn only_the_newest_thirty_are_kept() {
    let mut n = Notices::default();
    for id in 0..40 {
        n.push(Notice::update_available(id, "g"));
    }
    assert_eq!(n.len(), 30);
    assert_eq!(n.all().last().and_then(|x| x.game_id), Some(10));
}
