use freya::prelude::*;

use crate::{
    components::{hoverable, pointer_cursor},
    effect::Effect,
    metrics::*,
    model::{ModEntry, ModStatus},
    nav::{Route, use_nav},
    prelude::*,
    shell::use_shell,
    typography::TypeStyle,
};

/// One mod in a list: who made it, what it does, and the install button. Pressing the row opens the
/// mod's page.
#[derive(Clone, PartialEq)]
pub struct ModRow {
    pub entry: ModEntry,
    pub key: DiffKey,
}

impl KeyExt for ModRow {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

/// "12.9k" for 12 904: downloads at a glance.
pub fn compact_count(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => format!("{:.1}k", n as f64 / 1_000.),
        _ => format!("{:.1}M", n as f64 / 1_000_000.),
    }
}

/// The button label and whether it can be pressed, for a mod's state.
pub fn mod_action(status: ModStatus) -> (&'static str, bool) {
    match status {
        ModStatus::Available => ("Install", true),
        ModStatus::Installing => ("Installing", false),
        ModStatus::Installed => ("Remove", true),
    }
}

/// The host command for pressing a mod's button.
pub fn mod_effect(entry: &ModEntry) -> Option<Effect> {
    let (provider, id) = (entry.provider, entry.id.clone());
    match entry.status {
        ModStatus::Available => Some(Effect::InstallMod { provider, id }),
        ModStatus::Installed => Some(Effect::RemoveMod { provider, id }),
        ModStatus::Installing => None,
    }
}

impl Component for ModRow {
    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }

    fn render(&self) -> impl IntoElement {
        let (t, nav, shell) = (use_reclaw(), use_nav(), use_shell());
        let hovering = use_state(|| false);
        let e = &self.entry;
        let (label, enabled) = mod_action(e.status);
        let route = Route::ModDetail { provider: e.provider.slug().to_string(), mod_id: e.id.clone() };
        let effect = mod_effect(e);
        let on_effect = shell.on_effect.clone();

        let button = ActionButton::new(if e.status == ModStatus::Installed { ButtonVariant::Secondary } else { ButtonVariant::Primary })
            .label(label)
            .icon(match e.status {
                ModStatus::Available => IconName::Download,
                ModStatus::Installing => IconName::Queue,
                ModStatus::Installed => IconName::Check,
            })
            .enabled(enabled)
            .on_press(move |ev: Event<PressEventData>| {
                // The button sits inside the pressable row; it must not also open the page.
                ev.stop_propagation();
                if let Some(effect) = effect.clone() {
                    on_effect.call(effect);
                }
            });

        let row = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_4)
            .width(Size::fill())
            .padding(SPACE_4)
            .background(if hovering() { t.bg_raised } else { t.bg_panel })
            .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
            .corner_radius(RADIUS_MD)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(format!("{} by {}", e.title, e.author))
            .on_press(move |_| nav.open(route.clone()))
            .child(
                rect()
                    .vertical()
                    .spacing(SPACE_1)
                    .width(Size::flex(1.))
                    .child(TypeStyle::Label.text(e.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis))
                    .child(
                        TypeStyle::Meta
                            .text(
                                format!(
                                    "{} · {} · v{} · {} downloads",
                                    e.author,
                                    e.provider.label(),
                                    e.version,
                                    compact_count(e.downloads)
                                ),
                                t.ink_subtle,
                            )
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis),
                    )
                    .child(TypeStyle::Body.text(e.summary.clone(), t.ink_muted).max_lines(2).text_overflow(TextOverflow::Ellipsis)),
            )
            .child(button);
        pointer_cursor(hoverable(row, hovering))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::sample_mods;

    #[test]
    fn downloads_are_abbreviated() {
        assert_eq!(compact_count(999), "999");
        assert_eq!(compact_count(12_904), "12.9k");
        assert_eq!(compact_count(2_500_000), "2.5M");
    }

    #[test]
    fn the_button_matches_the_mod_state() {
        let mods = sample_mods();
        let by = |status| mods.iter().find(|m| m.status == status).expect("sample mod");
        assert_eq!(
            mod_effect(by(ModStatus::Available)),
            Some(Effect::InstallMod { provider: by(ModStatus::Available).provider, id: by(ModStatus::Available).id.clone() })
        );
        assert!(matches!(mod_effect(by(ModStatus::Installed)), Some(Effect::RemoveMod { .. })));
        assert_eq!(mod_effect(by(ModStatus::Installing)), None);
        assert_eq!(mod_action(ModStatus::Installing), ("Installing", false));
    }
}
