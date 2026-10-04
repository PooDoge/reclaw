use freya::prelude::*;
use reclaw_games::project::Release;

use super::ctx::Ctx;
use crate::{desktop::pages::common::heading, metrics::*, prelude::*, typography::TypeStyle};

/// How many releases the page shows; the link at the end goes to the rest.
const SHOWN: usize = 3;
/// Lines of release notes before the rest is left to the project page.
const NOTE_LINES: usize = 3;

/// The project's recent releases with the first lines of their notes, newest first as the catalog
/// lists them. Like Steam's "recent updates", from the project's own page.
pub(super) fn view(c: &Ctx) -> Option<Element> {
    let project = c.view.project.as_ref().filter(|p| !p.releases.is_empty())?;
    let t = &c.t;
    let open = c.open_url();
    let all_releases = format!("{}/releases", project.repo.url());
    let rows = project.releases.iter().take(SHOWN).map(|r| release_row(t, r, &open));
    Some(
        rect()
            .vertical()
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(heading(t, "Recent updates"))
            .children(rows)
            .child(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::External)
                    .label("All releases")
                    .on_press(move |_| open.call(all_releases.clone())),
            )
            .into_element(),
    )
}

fn release_row(t: &Reclaw, release: &Release, open: &EventHandler<String>) -> Element {
    let url = release.url.clone();
    let open = open.clone();
    let notes: String = release.notes.lines().take(NOTE_LINES).collect::<Vec<_>>().join("\n");
    rect()
        .vertical()
        .spacing(SPACE_1)
        .width(Size::fill())
        .padding(SPACE_4)
        .background(t.bg_panel)
        .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
        .corner_radius(RADIUS_MD)
        .child(
            rect()
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .spacing(SPACE_3)
                .width(Size::fill())
                .child(TypeStyle::Mono.text(release.tag.clone(), t.accent))
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .child(TypeStyle::Label.text(release.name.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis)),
                )
                .maybe(release.prerelease, |row| row.child(TypeStyle::Eyebrow.text("Pre-release", t.warn)))
                .child(TypeStyle::Meta.text(release.published.clone(), t.ink_subtle)),
        )
        .maybe(!notes.is_empty(), |card| card.child(TypeStyle::Body.text(notes, t.ink_muted)))
        .child(
            rect().horizontal().child(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::External)
                    .label("View release")
                    .on_press(move |_| open.call(url.clone())),
            ),
        )
        .into_element()
}
