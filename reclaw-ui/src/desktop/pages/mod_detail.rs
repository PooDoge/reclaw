use freya::prelude::*;

use super::common::{NotFound, PageHeader, card, heading, page_scroll};
use crate::{
    desktop::use_desktop_ui,
    effect::Effect,
    metrics::*,
    model::{ModProvider, ModStatus},
    nav::{Route, use_nav},
    prelude::*,
    shell::use_shell,
    store::{use_games, use_mods, use_projects},
    typography::TypeStyle,
};

/// One mod: what it does, who made it, which game it is for, and the install button.
#[derive(PartialEq)]
pub struct ModDetailPage {
    pub provider: String,
    pub mod_id: String,
}

impl Component for ModDetailPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui, nav) = (use_shell(), use_desktop_ui(), use_nav());
        let (mods, games, projects) = (use_mods(), use_games(), use_projects());
        let env = *ui.env.read();
        let entry =
            ModProvider::from_slug(&self.provider).and_then(|_| mods.iter().find(|m| m.matches(&self.provider, &self.mod_id)).cloned());
        let Some(entry) = entry else {
            return NotFound { what: "mod", id: format!("{}/{}", self.provider, self.mod_id) }.into_element();
        };
        let game = games
            .iter()
            .find(|g| g.id == entry.game_id)
            .map(|g| g.title.to_string())
            .or_else(|| projects.iter().find(|p| p.id == entry.game_id).map(|p| p.title.clone()));

        let line = |name: &'static str, value: String| {
            rect()
                .horizontal()
                .content(Content::Flex)
                .spacing(SPACE_3)
                .width(Size::fill())
                .child(rect().width(Size::px(96.)).child(TypeStyle::Meta.text(name, t.ink_subtle)))
                .child(rect().width(Size::flex(1.)).child(TypeStyle::Body.text(value, t.ink)))
                .into_element()
        };
        let mut lines = vec![line("Author", entry.author.clone()), line("Version", entry.version.clone())];
        if let Some(installed) = entry.installed_version.clone().filter(|v| *v != entry.version) {
            lines.push(line("Installed", installed));
        }
        lines.extend([line("Site", entry.provider.label().to_string()), line("Downloads", super::common::compact_count(entry.downloads))]);
        if !entry.tags.is_empty() {
            lines.push(line("Tags", entry.tags.join(", ")));
        }

        let game_id = entry.game_id;
        // An installed mod with an update offers Update first; it can still be removed from here.
        let remove = (entry.status == ModStatus::UpdateReady).then(|| {
            let (on_effect, provider, id) = (shell.on_effect.clone(), entry.provider, entry.id.clone());
            ActionButton::new(ButtonVariant::Secondary)
                .icon(IconName::X)
                .label("Remove")
                .on_press(move |_| on_effect.call(Effect::RemoveMod { game: game_id, provider, id: id.clone() }))
        });
        let open_page = entry.page_url.clone().map(|url| {
            let on_effect = shell.on_effect.clone();
            ActionButton::new(ButtonVariant::Ghost)
                .icon(IconName::External)
                .label(format!("Open on {}", entry.provider.label()))
                .on_press(move |_| on_effect.call(Effect::OpenUrl(url.clone())))
        });
        let for_game = game.map(|title| {
            ActionButton::new(ButtonVariant::Secondary)
                .icon(IconName::Library)
                .label(format!("For {title}"))
                .on_press(move |_| nav.open(Route::Game { id: game_id }))
        });

        // The same label and command as the button in the list, so pressing it here does the same.
        let (label, enabled) = super::common::mod_action(entry.status);
        let effect = super::common::mod_effect(&entry);
        let on_effect = shell.on_effect.clone();
        let variant = if entry.status == ModStatus::Installed { ButtonVariant::Secondary } else { ButtonVariant::Primary };
        let button = ActionButton::new(variant)
            .label(label)
            .icon(super::common::mod_icon(entry.status))
            .enabled(enabled)
            .size(ButtonSize::for_density(env.density, true))
            .on_press(move |_| {
                if let Some(effect) = effect.clone() {
                    on_effect.call(effect);
                }
            });

        page_scroll(
            &env,
            rect()
                .vertical()
                .spacing(SPACE_5)
                .width(Size::fill())
                .child(PageHeader::new(entry.title.clone()).eyebrow(entry.provider.label()).trailing(button))
                .child(
                    rect()
                        .vertical()
                        .spacing(SPACE_2)
                        .width(Size::fill())
                        .child(heading(&t, "About"))
                        .child(TypeStyle::Body.text(entry.summary.clone(), t.ink)),
                )
                .child(rect().vertical().spacing(SPACE_2).width(Size::fill()).child(heading(&t, "Details")).child(card(&t, lines)))
                .child(
                    rect()
                        .horizontal()
                        .content(Content::wrap_spacing(SPACE_2))
                        .spacing(SPACE_2)
                        .width(Size::fill())
                        .maybe_child(for_game)
                        .maybe_child(remove)
                        .maybe_child(open_page),
                ),
        )
    }
}
