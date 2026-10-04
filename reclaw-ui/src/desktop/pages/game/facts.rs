use freya::prelude::*;
use reclaw_games::project::SpecSheet;

use super::ctx::Ctx;
use crate::{
    desktop::pages::common::{card, heading},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

fn line(t: &Reclaw, name: &'static str, value: String) -> Element {
    rect()
        .horizontal()
        .content(Content::Flex)
        .spacing(SPACE_3)
        .width(Size::fill())
        .child(rect().width(Size::px(88.)).child(TypeStyle::Meta.text(name, t.ink_subtle)))
        .child(rect().width(Size::flex(1.)).child(TypeStyle::Body.text(value, t.ink)))
        .into_element()
}

/// Platform, where it comes from, the latest and the installed version.
pub(super) fn details(c: &Ctx) -> Option<Element> {
    let (t, game) = (&c.t, &c.view.game);
    let mut lines = vec![line(t, "Source", game.source.host().to_string())];
    if let Some(project) = &c.view.project {
        lines.insert(0, line(t, "Platform", project.platform.label().to_string()));
        if let Some(latest) = project.latest_release() {
            lines.push(line(t, "Latest", latest.tag.clone()));
        }
    }
    if !game.version.is_empty() {
        lines.push(line(t, "Installed", game.version.to_string()));
    }
    Some(rect().vertical().spacing(SPACE_2).width(Size::fill()).child(heading(t, "Details")).child(card(t, lines)).into_element())
}

fn sheet(t: &Reclaw, title: &'static str, sheet: &SpecSheet) -> Vec<Element> {
    std::iter::once(TypeStyle::Label.text(title, t.ink).into_element())
        .chain(sheet.lines().into_iter().map(|(name, value)| line(t, name, value.to_string())))
        .collect()
}

/// Minimum and recommended hardware, if the project publishes any.
pub(super) fn requirements(c: &Ctx) -> Option<Element> {
    let t = &c.t;
    let req = c.view.project.as_ref()?.requirements.as_ref()?;
    if req.minimum.is_empty() && req.recommended.as_ref().is_none_or(SpecSheet::is_empty) {
        return None;
    }
    let mut rows = sheet(t, "Minimum", &req.minimum);
    if let Some(recommended) = req.recommended.as_ref().filter(|r| !r.is_empty()) {
        rows.extend(sheet(t, "Recommended", recommended));
    }
    if let Some(notes) = &req.notes {
        rows.push(TypeStyle::Meta.text(notes.clone(), t.ink_muted).into_element());
    }
    Some(
        rect().vertical().spacing(SPACE_2).width(Size::fill()).child(heading(t, "System requirements")).child(card(t, rows)).into_element(),
    )
}

/// The project's page and its issue tracker.
pub(super) fn links(c: &Ctx) -> Option<Element> {
    let project = c.view.project.as_ref()?;
    let open = c.open_url();
    let (home, issues) = (project.repo.url(), format!("{}/issues", project.repo.url()));
    let (open_home, open_issues) = (open.clone(), open);
    Some(
        rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(heading(&c.t, "Links"))
            .child(
                ActionButton::new(ButtonVariant::Secondary)
                    .icon(IconName::External)
                    .label("Project page")
                    .on_press(move |_| open_home.call(home.clone())),
            )
            .child(
                ActionButton::new(ButtonVariant::Ghost)
                    .icon(IconName::External)
                    .label("Report a problem")
                    .on_press(move |_| open_issues.call(issues.clone())),
            )
            .into_element(),
    )
}
