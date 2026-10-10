use freya::prelude::*;
use reclaw_catalog::site;

use super::ctx::Ctx;
use crate::{
    community::{
        PageData, PageState, RELEASES_NOTE, Rating, ReleaseLine, ReviewLine, Tone, VERIFIED_MEANS, checking_line, facts, share_title,
    },
    desktop::pages::common::{card, heading},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

fn tone_color(t: &Reclaw, tone: Tone) -> Color {
    match tone {
        Tone::Positive => t.ok,
        Tone::Caution => t.warn,
        Tone::Negative => t.danger,
        Tone::Muted => t.ink_subtle,
    }
}

fn loaded(c: &Ctx) -> Option<&PageData> {
    match c.community.page.as_ref()? {
        PageState::Loaded(page) => Some(page),
        PageState::Loading => None,
    }
}

fn status(c: &Ctx, text: String) -> Element {
    TypeStyle::Meta.text(text, c.t.ink_subtle).into_element()
}

/// The line under a part that is not there yet, or could not be read.
fn part_status<T>(c: &Ctx, part: Option<&Result<T, String>>, what: &str) -> Option<Element> {
    match part {
        None => Some(status(c, format!("Loading {what}…"))),
        Some(Err(error)) => Some(status(c, format!("Couldn't load {what}. {error}"))),
        Some(Ok(_)) => None,
    }
}

/// How it runs: what players said, then their reviews. Reviews are written on the website, so the button goes there.
pub(super) fn feedback(c: &Ctx) -> Option<Element> {
    let app = c.community.app.as_ref()?;
    let t = &c.t;
    let rating = Rating::of(app);
    let open = c.open_url();
    let review_url = site::review_page_url(&app.slug);
    let reviews = loaded(c).map(|p| &p.reviews);
    let lines: Vec<Element> = match reviews {
        Some(Ok(reviews)) => reviews.iter().map(|r| review(t, &ReviewLine::new(r))).collect(),
        _ => Vec::new(),
    };
    Some(
        rect()
            .vertical()
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(heading(t, "How it runs"))
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_2)
                    .child(TypeStyle::Heading.text(rating.label, tone_color(t, rating.tone)))
                    .child(TypeStyle::Meta.text(rating.counts.clone(), t.ink_muted)),
            )
            .children(lines)
            .maybe_child(part_status(c, reviews, "reviews"))
            .maybe(matches!(reviews, Some(Ok(r)) if r.is_empty()) && rating.players > 0, |el| {
                el.child(status(c, "Players rated it without writing a review.".into()))
            })
            .child(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_3)
                    .child(TypeStyle::Body.text(share_title(rating.players), t.ink_muted))
                    .child(
                        ActionButton::new(ButtonVariant::Secondary)
                            .icon(IconName::External)
                            .label("Share how it runs")
                            .on_press(move |_| open.call(review_url.clone())),
                    ),
            )
            .into_element(),
    )
}

fn review(t: &Reclaw, line: &ReviewLine) -> Element {
    card(
        t,
        [rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(TypeStyle::Label.text(line.author.clone(), t.ink))
            .child(
                rect()
                    .width(Size::flex(1.))
                    .maybe_child((!line.meta.is_empty()).then(|| TypeStyle::Meta.text(line.meta.clone(), t.ink_subtle))),
            )
            .child(TypeStyle::Label.text(line.result, tone_color(t, line.tone)))
            .into_element()]
        .into_iter()
        .chain((!line.body.is_empty()).then(|| TypeStyle::Body.text(line.body.clone(), t.ink_muted).into_element())),
    )
    .into_element()
}

/// The releases as quiverlauncher.com judges them: verified, unverified or blocked, and why. Nothing until the page is read, so the
/// catalog's own "Recent updates" stands in until then.
pub(super) fn releases(c: &Ctx) -> Option<Element> {
    c.community.app.as_ref()?;
    let page = loaded(c)?;
    let t = &c.t;
    let detail = page.detail.as_ref().ok().and_then(Option::as_ref);
    let rows: Vec<Element> = match &page.releases {
        Ok(releases) if releases.is_empty() => return None,
        Ok(releases) => releases.iter().take(5).map(|r| release(t, &ReleaseLine::new(r, c.now))).collect(),
        Err(error) => vec![status(c, format!("Couldn't load releases. {error}"))],
    };
    let checking = detail.and_then(|d| d.checking.as_ref()).and_then(|ch| checking_line(ch, c.now));
    let withdrawn: Vec<Element> = detail
        .map(|d| d.withdrawn.iter().map(|w| status(c, format!("{} was withdrawn: {}", w.version, w.reason.trim()))).collect())
        .unwrap_or_default();
    Some(
        rect()
            .vertical()
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(heading(t, "Releases"))
            .child(TypeStyle::Meta.text(format!("{RELEASES_NOTE} {VERIFIED_MEANS}"), t.ink_subtle))
            .maybe_child(checking.map(|text| TypeStyle::Body.text(text, t.warn).into_element()))
            .children(rows)
            .children(withdrawn)
            .into_element(),
    )
}

fn release(t: &Reclaw, line: &ReleaseLine) -> Element {
    card(
        t,
        [rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(TypeStyle::Mono.text(line.version.clone(), t.accent))
            .child(TypeStyle::Eyebrow.text(line.state, tone_color(t, line.tone)))
            .maybe(line.prerelease, |row| row.child(TypeStyle::Eyebrow.text("Pre-release", t.warn)))
            .child(rect().width(Size::flex(1.)))
            .maybe_child((!line.when.is_empty()).then(|| TypeStyle::Meta.text(line.when.clone(), t.ink_subtle)))
            .into_element()]
        .into_iter()
        .chain(line.reasons.iter().map(|r| TypeStyle::Body.text(r.clone(), t.ink_muted).into_element())),
    )
    .into_element()
}

/// Beside the game: who made it, where it runs, its latest and verified releases, AI use, and a link to its page on the site.
pub(super) fn about(c: &Ctx) -> Option<Element> {
    let app = c.community.app.as_ref()?;
    let t = &c.t;
    let project = loaded(c).and_then(|p| p.detail.as_ref().ok()).and_then(Option::as_ref).map(|d| &d.project);
    let rows = facts(app, project, c.now).into_iter().map(|(term, value)| {
        rect()
            .horizontal()
            .content(Content::Flex)
            .spacing(SPACE_3)
            .width(Size::fill())
            .child(rect().width(Size::px(88.)).child(TypeStyle::Meta.text(term, t.ink_subtle)))
            .child(rect().width(Size::flex(1.)).child(TypeStyle::Body.text(value, t.ink)))
            .into_element()
    });
    let open = c.open_url();
    let page_url = site::app_page_url(&app.slug);
    Some(
        rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(heading(t, "On quiverlauncher.com"))
            .child(card(t, rows))
            .child(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::External)
                    .label("Open its page")
                    .on_press(move |_| open.call(page_url.clone())),
            )
            .into_element(),
    )
}
