use freya::prelude::*;

use super::highlighted;
use crate::{
    components::{hoverable, pointer_cursor},
    effect::Effect,
    metrics::*,
    model::{ModEntry, ModProvider, ModStatus},
    nav::{Route, use_nav},
    prelude::*,
    shell::use_shell,
    store::now_secs,
    typography::TypeStyle,
};

/// One mod in a list: who made it, what it does, and the install button. Pressing the row opens the
/// mod's page.
#[derive(Clone, PartialEq)]
pub struct ModRow {
    pub entry: ModEntry,
    /// A search the row was found by: the words it matched are highlighted in its title.
    pub query: String,
    pub key: DiffKey,
}

impl ModRow {
    pub fn new(entry: ModEntry) -> Self {
        Self { entry, query: String::new(), key: DiffKey::None }
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }
}

/// "3 days ago": how long since `then`, roughly, for a line of small print.
pub fn ago(then: u64, now: u64) -> String {
    let secs = now.saturating_sub(then);
    let (n, unit) = match secs {
        0..=3_599 => return "just now".to_string(),
        3_600..=86_399 => (secs / 3_600, "hour"),
        86_400..=2_591_999 => (secs / 86_400, "day"),
        2_592_000..=31_535_999 => (secs / 2_592_000, "month"),
        _ => (secs / 31_536_000, "year"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

/// The small print under a mod's title: who, where, which version, how popular, how recent.
pub fn meta_line(e: &ModEntry, now: u64) -> String {
    let rating = match (e.rating, e.provider) {
        (0, _) => String::new(),
        (n, ModProvider::Thunderstore) => format!("{} ratings", compact_count(n)),
        (n, ModProvider::GameBanana) => format!("{} likes", compact_count(n)),
    };
    [
        e.author.clone(),
        e.provider.label().to_string(),
        version_text(e),
        format!("{} downloads", compact_count(e.downloads)),
        rating,
        e.updated.map(|u| format!("updated {}", ago(u, now))).unwrap_or_default(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" \u{b7} ")
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
        ModStatus::UpdateReady => ("Update", true),
    }
}

/// The icon beside that label: the button removes an installed mod, so it shows what pressing does, not the state.
pub fn mod_icon(status: ModStatus) -> IconName {
    match status {
        ModStatus::Available | ModStatus::UpdateReady => IconName::Download,
        ModStatus::Installing => IconName::Queue,
        ModStatus::Installed => IconName::X,
    }
}

/// The host command for pressing a mod's button.
pub fn mod_effect(entry: &ModEntry) -> Option<Effect> {
    let (game, provider, id) = (entry.game_id, entry.provider, entry.id.clone());
    match entry.status {
        ModStatus::Available | ModStatus::UpdateReady => Some(Effect::InstallMod { game, provider, id }),
        ModStatus::Installed => Some(Effect::RemoveMod { game, provider, id }),
        ModStatus::Installing => None,
    }
}

/// The version part of a mod's line: the newest one, or what is installed and what it would update to.
pub fn version_text(entry: &ModEntry) -> String {
    match (&entry.installed_version, entry.status) {
        (Some(installed), ModStatus::UpdateReady) => format!("v{installed} \u{2192} v{}", entry.version),
        (Some(installed), _) => format!("v{installed}"),
        (None, _) if entry.version.is_empty() => String::new(),
        (None, _) => format!("v{}", entry.version),
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
            .icon(mod_icon(e.status))
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
                    .child(if self.query.trim().is_empty() {
                        TypeStyle::Label.text(e.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis).into_element()
                    } else {
                        highlighted(&t, &e.title, &self.query, TypeStyle::Label, t.ink).into_element()
                    })
                    .child(TypeStyle::Meta.text(meta_line(e, now_secs()), t.ink_subtle).max_lines(1).text_overflow(TextOverflow::Ellipsis))
                    .child(TypeStyle::Body.text(e.summary.clone(), t.ink_muted).max_lines(2).text_overflow(TextOverflow::Ellipsis)),
            )
            .child(button);
        pointer_cursor(hoverable(row, hovering))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::sample_mods;

    #[test]
    fn ages_are_rough() {
        let now = 10_000_000;
        assert_eq!(ago(now - 10, now), "just now");
        assert_eq!(ago(now - 7_200, now), "2 hours ago");
        assert_eq!(ago(now - 86_400, now), "1 day ago");
        assert_eq!(ago(now - 90 * 86_400, now), "3 months ago");
        assert_eq!(ago(now + 50, now), "just now", "a clock behind the site's says nothing odd");
    }

    #[test]
    fn the_small_print_has_what_the_site_said() {
        let mut m = sample_mods().remove(0);
        m.updated = Some(1_000);
        assert_eq!(
            meta_line(&m, 1_000 + 2 * 86_400),
            "pixelwright \u{b7} Thunderstore \u{b7} v2.1.0 \u{b7} 48.2k downloads \u{b7} 120 ratings \u{b7} updated 2 days ago"
        );
        m.rating = 0;
        m.updated = None;
        assert!(!meta_line(&m, 0).contains("ratings") && !meta_line(&m, 0).contains("updated"));
    }

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
        let available = by(ModStatus::Available);
        assert_eq!(
            mod_effect(available),
            Some(Effect::InstallMod { game: available.game_id, provider: available.provider, id: available.id.clone() })
        );
        assert!(matches!(mod_effect(by(ModStatus::Installed)), Some(Effect::RemoveMod { .. })));
        assert_eq!(mod_effect(by(ModStatus::Installing)), None);
        assert_eq!(mod_action(ModStatus::Installing), ("Installing", false));
        let update = ModEntry { status: ModStatus::UpdateReady, installed_version: Some("1.0.0".into()), ..available.clone() };
        assert!(matches!(mod_effect(&update), Some(Effect::InstallMod { .. })), "Update installs the newer version");
        assert_eq!(mod_action(ModStatus::UpdateReady), ("Update", true));
    }

    #[test]
    fn the_version_says_what_an_update_would_change() {
        let mods = sample_mods();
        let base = mods.iter().find(|m| m.status == ModStatus::Available).expect("sample mod").clone();
        assert_eq!(version_text(&base), format!("v{}", base.version));
        let update =
            ModEntry { status: ModStatus::UpdateReady, installed_version: Some("1.0.0".into()), version: "1.1.0".into(), ..base.clone() };
        assert_eq!(version_text(&update), "v1.0.0 \u{2192} v1.1.0");
        assert_eq!(version_text(&ModEntry { version: String::new(), ..base }), "");
    }
}
