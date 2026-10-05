use freya::prelude::*;
use reclaw_input::FocusId;

use crate::{
    deck::{
        TextBoxes, ids,
        settings::{
            CredentialPart, RowKind, Schema, Section, SettingsTarget, SettingsValues, TextField,
            geometry::{GROUP_GAP, HEADING_H, NOTE_H, ROW_GAP},
        },
    },
    metrics::*,
    prelude::*,
    surface::{RevealScroll, RowControl, SettingRow},
    typography::TypeStyle,
};

/// One entry of the section list: a blue wash when it is the section being shown, a raised fill
/// and accent bar when the pad is on it.
#[derive(Clone, PartialEq)]
struct NavItem {
    title: &'static str,
    selected: bool,
    focused: bool,
    on_press: EventHandler<()>,
}

impl Component for NavItem {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let on_press = self.on_press.clone();
        let wash = LinearGradient::new()
            .angle(90.)
            .stop((Color::from_argb(110, t.accent.r(), t.accent.g(), t.accent.b()), 0.))
            .stop((Color::from_argb(0, t.accent.r(), t.accent.g(), t.accent.b()), 100.));
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .height(Size::px(DECK_ROW_H))
            .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
            .maybe(self.selected, |el| el.background(wash.clone()))
            .maybe(self.focused, |el| el.background(t.bg_raised))
            .border(
                Border::new()
                    .fill(if self.focused { t.accent } else { crate::components::CLEAR })
                    .width(BorderWidth { left: 4., ..Default::default() })
                    .alignment(BorderAlignment::Inner),
            )
            .child(TypeStyle::DeckBody.text(self.title, if self.selected || self.focused { t.ink } else { t.ink_muted }))
            .on_press(move |_| on_press.call(()))
    }
}

/// The Settings and Properties body: sections beside rows on a wide window, a list and then a
/// section's rows on a narrow one. The reducer decides which (`two_pane`, `drilled`).
#[derive(Clone, PartialEq)]
pub struct SettingsBody {
    pub schema: Schema,
    pub target: SettingsTarget,
    pub values: SettingsValues,
    /// The text on each launch row ("Default (1920x1080)", "Game's own", "Off"), by setting.
    pub launch_text: std::collections::HashMap<reclaw_games::settings::SettingKey, String>,
    /// What is known about the access tokens, for the Network section's status lines.
    pub credentials: crate::credentials::CredentialsStatus,
    pub section: usize,
    pub two_pane: bool,
    pub drilled: bool,
    pub texts: TextBoxes,
    pub focus: FocusId,
    pub ring_visible: bool,
    /// Visible height of the rows pane and the focused row's span, for the two-pane scroll.
    pub viewport: f32,
    pub reveal: Option<(f32, f32)>,
    pub on_click: EventHandler<FocusId>,
}

impl SettingsBody {
    fn nav(&self, width: Size) -> Element {
        let items = self.schema.sections.iter().enumerate().map(|(i, s)| {
            let id = ids::settings_nav(i);
            let on_click = self.on_click.clone();
            NavItem {
                title: s.title,
                selected: i == self.section,
                focused: self.ring_visible && self.focus == id,
                on_press: EventHandler::new(move |()| on_click.call(id)),
            }
            .into_element()
        });
        rect().vertical().width(width).children(items).into_element()
    }

    fn row_element(&self, section_index: usize, row_index: usize, row: &crate::deck::settings::Row) -> Element {
        let t = self.target;
        let id = ids::settings_row(section_index, row_index);
        let control = match &row.kind {
            RowKind::Toggle { default } => RowControl::Toggle(self.values.toggle(t, row.key, *default)),
            RowKind::Choice { options, default } => RowControl::Value {
                text: options.get(self.values.choice(t, row.key, *default)).copied().unwrap_or("").to_string(),
                opens_menu: true,
            },
            RowKind::Text { field, placeholder } => {
                let (input, a11y) = self.texts.get(*field);
                RowControl::Text { input, placeholder: (*placeholder).to_string(), a11y, secret: field.is_secret() }
            }
            RowKind::Credential { provider, part } => match part {
                CredentialPart::Status => RowControl::Value { text: self.credentials.of(*provider).short(*provider), opens_menu: false },
                CredentialPart::Token => {
                    let (input, a11y) = self.texts.get(TextField::for_provider(*provider));
                    RowControl::Text { input, placeholder: "Paste your token".to_string(), a11y, secret: true }
                }
                CredentialPart::Remove => RowControl::Action { danger: true },
                CredentialPart::Save | CredentialPart::Check | CredentialPart::Create => RowControl::Action { danger: false },
            },
            RowKind::Global { .. } => RowControl::Action { danger: false },
            RowKind::Action { danger, .. } => RowControl::Action { danger: *danger },
            RowKind::Info { value } => RowControl::Value { text: value.clone(), opens_menu: false },
            RowKind::Launch { key } => RowControl::Value { text: self.launch_text.get(key).cloned().unwrap_or_default(), opens_menu: true },
        };
        let on_click = self.on_click.clone();
        let mut r = SettingRow::new(row.label, control, Density::Controller)
            .focused(self.ring_visible && self.focus == id)
            .on_press(EventHandler::new(move |()| on_click.call(id)));
        if let Some(d) = row.description {
            r = r.description(d);
        }
        r.into_element()
    }

    /// Every group of a section, headings and notes included, spaced exactly as `section_slots` says.
    fn rows(&self, t: Reclaw, section_index: usize, section: &Section) -> Element {
        let mut index = 0;
        let groups = section.groups.iter().enumerate().map(|(g, group)| {
            let rows: Vec<Element> = group
                .rows
                .iter()
                .map(|row| {
                    let el = self.row_element(section_index, index, row);
                    index += 1;
                    el
                })
                .collect();
            rect()
                .vertical()
                .width(Size::fill())
                .maybe(g > 0, |el| el.margin(Gaps::new(GROUP_GAP, 0., 0., 0.)))
                .maybe_child(group.heading.map(|h| {
                    rect()
                        .height(Size::px(HEADING_H))
                        .main_align(Alignment::End)
                        .padding(Gaps::new(0., 0., 8., 0.))
                        .child(TypeStyle::DeckLabel.text(h, t.ink))
                }))
                .maybe_child(
                    group.note.map(|n| rect().height(Size::px(NOTE_H)).child(TypeStyle::DeckMeta.text(n, t.ink_muted).max_lines(2))),
                )
                .child(rect().vertical().spacing(ROW_GAP).width(Size::fill()).children(rows))
                .into_element()
        });
        rect().vertical().width(Size::fill()).children(groups).into_element()
    }
}

impl Component for SettingsBody {
    fn render(&self) -> impl IntoElement {
        // Read the tokens here: `rows` runs only in some modes, and hooks must not be conditional.
        let t = use_reclaw();
        let section_ix = self.section.min(self.schema.sections.len().saturating_sub(1));
        let Some(section) = self.schema.sections.get(section_ix) else { return rect().into_element() };
        if self.two_pane {
            return rect()
                .horizontal()
                .content(Content::Flex)
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    rect()
                        .width(Size::px(DECK_SETTINGS_NAV_W))
                        .height(Size::fill())
                        .padding(Gaps::new(SPACE_4, 0., 0., 0.))
                        .child(self.nav(Size::fill())),
                )
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .padding(Gaps::new(SPACE_4, SPACE_5, 0., 48.))
                        .child(RevealScroll::new(self.rows(t, section_ix, section), self.reveal, self.viewport)),
                )
                .into_element();
        }
        if self.drilled { self.rows(t, section_ix, section) } else { self.nav(Size::fill()) }
    }
}
