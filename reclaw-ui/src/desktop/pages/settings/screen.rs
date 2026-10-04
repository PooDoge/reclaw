use freya::prelude::*;

use super::form::Form;
use crate::{
    catalog::GameView,
    desktop::{
        pages::common::{NotFound, PageHeader},
        use_desktop_ui,
    },
    metrics::*,
    nav::{Route, use_nav},
    prelude::*,
    settings::{LaunchContext, Schema, SettingsTarget, app_properties, global_settings},
    shell::use_shell,
    store::{use_display, use_games, use_launch, use_mods, use_projects, use_settings},
    typography::TypeStyle,
};

/// Where a game's data comes from, for the properties page.
#[derive(Clone, PartialEq)]
pub(super) enum GameSource {
    None,
    Library(u32),
}

/// A schema's sections beside its rows (wide and compact) or one after the other (phone).
#[derive(PartialEq)]
pub(super) struct SettingsScreen {
    pub target: SettingsTarget,
    /// The section named by the URL: opens it, and on a phone is the page being shown.
    pub section: Option<String>,
    pub source: GameSource,
}

impl SettingsScreen {
    fn route_for(&self, section: &str) -> Route {
        match self.target {
            SettingsTarget::Global => Route::SettingsSection { section: section.to_string() },
            SettingsTarget::App(id) => Route::GameSettingsSection { id, section: section.to_string() },
        }
    }
}

impl Component for SettingsScreen {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let (shell, ui, nav) = (use_shell(), use_desktop_ui(), use_nav());
        let (values, launch, display) = (use_settings(), use_launch(), use_display());
        let (games, projects, mods) = (use_games(), use_projects(), use_mods());
        let env = *ui.env.read();

        let ctx = LaunchContext { env: &display, projects: &projects, prefs: &launch };
        let specs = ctx.specs(self.target);
        let (schema, title, eyebrow): (Schema, String, &str) = match (&self.source, self.target) {
            (GameSource::Library(id), SettingsTarget::App(_)) => match GameView::resolve(*id, &games, &projects, &mods) {
                Some(view) => (app_properties(&view.game, &specs), view.game.title.to_string(), "Properties"),
                None => return NotFound { what: "game", id: id.to_string() }.into_element(),
            },
            _ => (global_settings(&specs, &display), "Settings".to_string(), ""),
        };

        let first = schema.sections.first().map(|s| s.id.to_string()).unwrap_or_default();
        let current = self.section.clone().filter(|id| schema.sections.iter().any(|s| s.id == id)).unwrap_or(first);
        let mut selected = use_state({
            let current = current.clone();
            move || current
        });
        let phone = env.class == LayoutClass::Phone;
        // The URL names the section on a phone; elsewhere the choice is local, so changing section
        // does not fill the history with entries.
        let showing = if phone { current.clone() } else { selected.read().clone() };
        let section_ix = schema.sections.iter().position(|s| s.id == showing).unwrap_or(0);

        let buttons: Vec<Element> = schema
            .sections
            .iter()
            .map(|s| {
                let (id, route) = (s.id, self.route_for(s.id));
                let active = !phone && s.id == showing;
                SectionButton {
                    title: s.title,
                    active,
                    on_press: EventHandler::new(move |()| {
                        if phone {
                            nav.open(route.clone());
                        } else {
                            selected.set(id.to_string());
                        }
                    }),
                    key: DiffKey::None,
                }
                .key(id)
                .into_element()
            })
            .collect();

        let form = Form {
            target: self.target,
            schema: schema.clone(),
            section: section_ix,
            values,
            launch,
            display,
            projects,
            density: env.density,
            on_effect: shell.on_effect.clone(),
            dialogs: ui.dialogs,
        };
        // On a phone the page is the list, or one section with its title.
        let show_list = phone && self.section.is_none();
        let heading = if phone && !show_list {
            schema.sections.get(section_ix).map(|s| s.title.to_string()).unwrap_or(title.clone())
        } else {
            title.clone()
        };
        let eyebrow = if eyebrow.is_empty() { None } else { Some(eyebrow.to_string()) };
        let header = {
            let h = PageHeader::new(heading);
            match eyebrow {
                Some(e) => h.eyebrow(e),
                None => h,
            }
        };

        let list = rect().vertical().spacing(2.).width(if phone { Size::fill() } else { Size::px(SIDEBAR_W) }).children(buttons);
        let body = if phone {
            if show_list {
                ScrollView::new().show_scrollbar(false).child(rect().padding(SPACE_4).child(list)).into_element()
            } else {
                ScrollView::new().show_scrollbar(false).child(rect().padding(SPACE_4).child(form)).into_element()
            }
        } else {
            rect()
                .horizontal()
                .content(Content::Flex)
                .spacing(SPACE_5)
                .width(Size::fill())
                .height(Size::flex(1.))
                .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                .child(list)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .child(ScrollView::new().show_scrollbar(false).child(rect().padding(Gaps::new(0., 0., SPACE_5, 0.)).child(form))),
                )
                .into_element()
        };
        rect()
            .vertical()
            .content(Content::Flex)
            .expanded()
            .background(t.bg_base)
            .child(rect().width(Size::fill()).padding(Gaps::new(SPACE_4, SPACE_5, SPACE_3, SPACE_5)).child(header))
            .child(body)
            .into_element()
    }
}

/// One entry of the section list.
#[derive(Clone, PartialEq)]
struct SectionButton {
    title: &'static str,
    active: bool,
    on_press: EventHandler<()>,
    key: DiffKey,
}

impl KeyExt for SectionButton {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for SectionButton {
    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }

    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let on_press = self.on_press.clone();
        let body = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .height(Size::px(TARGET_MIN))
            .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
            .corner_radius(RADIUS_MD)
            .background(if self.active {
                t.bg_raised
            } else if hovering() {
                t.bg_panel
            } else {
                crate::components::CLEAR
            })
            .border(
                Border::new()
                    .fill(if self.active { t.accent } else { crate::components::CLEAR })
                    .width(BorderWidth { left: 3., ..Default::default() })
                    .alignment(BorderAlignment::Inner),
            )
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(self.title)
            .on_press(move |_| on_press.call(()))
            .child(TypeStyle::Label.text(self.title, if self.active { t.ink } else { t.ink_muted }));
        crate::components::pointer_cursor(crate::components::hoverable(body, hovering))
    }
}
