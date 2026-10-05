use freya::prelude::*;
use reclaw_config::LaunchPrefs;
use reclaw_games::{project::ProjectInfo, settings::DisplayEnvironment};

use super::{text_row::TextRow, tokens::TokenBoxes};
use crate::{
    credentials::CredentialsStatus,
    desktop::{GameDialogs, OpenPicker},
    effect::Effect,
    metrics::*,
    prelude::*,
    settings::{
        CredentialPart, GlobalAction, LaunchContext, Row, RowAction, RowKind, Schema, SettingValue, SettingsTarget, SettingsValues,
        geometry::GROUP_GAP,
    },
    surface::{RowControl, SettingRow},
    typography::TypeStyle,
};

/// One section of a schema as rows. Pressing a row does what the same row does in Deck mode, by
/// sending the same effects: the shell applies the ones that are the UI's and the host gets the rest.
#[derive(Clone, PartialEq)]
pub(super) struct Form {
    pub target: SettingsTarget,
    pub schema: Schema,
    pub section: usize,
    pub values: SettingsValues,
    pub launch: LaunchPrefs,
    pub display: DisplayEnvironment,
    pub projects: Vec<ProjectInfo>,
    pub density: Density,
    pub on_effect: EventHandler<Effect>,
    pub dialogs: GameDialogs,
    pub credentials: CredentialsStatus,
    pub tokens: TokenBoxes,
}

impl Form {
    fn row(&self, row: &Row) -> Element {
        let (target, density) = (self.target, self.density);
        let on_effect = self.on_effect.clone();
        let described = |r: SettingRow| match row.description {
            Some(d) => r.description(d),
            None => r,
        };
        match &row.kind {
            RowKind::Toggle { default } => {
                let current = self.values.toggle(target, row.key, *default);
                let key = row.key;
                described(SettingRow::new(row.label, RowControl::Toggle(current), density).on_press(move |()| {
                    for effect in Effect::setting(target, key, SettingValue::Bool(!current)) {
                        on_effect.call(effect);
                    }
                }))
                .into_element()
            }
            RowKind::Choice { options, default } => {
                let current = self.values.choice(target, row.key, *default);
                let (label, key, dialogs) = (row.label, row.key, self.dialogs);
                let labels: Vec<String> = options.iter().map(|o| (*o).to_string()).collect();
                described(
                    SettingRow::new(
                        row.label,
                        RowControl::Value { text: options.get(current).copied().unwrap_or("").to_string(), opens_menu: true },
                        density,
                    )
                    .on_press_at(move |at: (f32, f32)| {
                        let on_effect = on_effect.clone();
                        let pick = EventHandler::new(move |i: usize| {
                            for effect in Effect::setting(target, key, SettingValue::Choice(i)) {
                                on_effect.call(effect);
                            }
                        });
                        dialogs.pick(OpenPicker::new(label, labels.clone(), current, at, pick));
                    }),
                )
                .into_element()
            }
            RowKind::Text { field, placeholder } => {
                let Some(key) = field.key() else { return rect().into_element() };
                let field = *field;
                TextRow {
                    label: row.label,
                    description: row.description,
                    placeholder,
                    initial: self.values.text(target, key).unwrap_or_default().to_string(),
                    density,
                    on_change: EventHandler::new(move |value: String| {
                        on_effect.call(Effect::TextCommitted { app: target.app(), field, value });
                    }),
                    key: DiffKey::None,
                }
                .key(key)
                .into_element()
            }
            RowKind::Action { action, danger } => {
                let (action, app, dialogs) = (*action, target.app(), self.dialogs);
                described(SettingRow::new(row.label, RowControl::Action { danger: *danger }, density).on_press(move |()| {
                    let Some(app) = app else { return };
                    match action {
                        RowAction::OpenFolder => on_effect.call(Effect::OpenFolder(app)),
                        RowAction::Verify => on_effect.call(Effect::Verify(app)),
                        RowAction::CheckUpdate => on_effect.call(Effect::CheckUpdate(app)),
                        RowAction::Uninstall => dialogs.uninstall(app),
                    }
                }))
                .into_element()
            }
            RowKind::Info { value } => {
                described(SettingRow::new(row.label, RowControl::Value { text: value.clone(), opens_menu: false }, density)).into_element()
            }
            RowKind::Credential { provider, part } => {
                let provider = *provider;
                let action = |danger: bool, press: Box<dyn Fn()>| {
                    described(SettingRow::new(row.label, RowControl::Action { danger }, density).on_press(move |()| press())).into_element()
                };
                match part {
                    CredentialPart::Status => described(SettingRow::new(
                        row.label,
                        RowControl::Value { text: self.credentials.of(provider).short(provider), opens_menu: false },
                        density,
                    ))
                    .into_element(),
                    CredentialPart::Token => {
                        let (input, a11y) = self.tokens.get(provider);
                        let control = RowControl::Text { input, placeholder: "Paste your token".to_string(), a11y, secret: true };
                        described(SettingRow::new(row.label, control, density)).into_element()
                    }
                    CredentialPart::Save => {
                        let (input, _) = self.tokens.get(provider);
                        action(
                            false,
                            Box::new(move || {
                                // The pasted text becomes a `Secret` here and the box is emptied.
                                let mut input = input;
                                let token = reclaw_log::Secret::new(input.peek().trim());
                                input.set(String::new());
                                on_effect.call(Effect::SaveToken { provider, token });
                            }),
                        )
                    }
                    CredentialPart::Check => action(false, Box::new(move || on_effect.call(Effect::CheckToken(provider)))),
                    CredentialPart::Create => {
                        action(false, Box::new(move || on_effect.call(Effect::OpenUrl(provider.token_page().to_string()))))
                    }
                    CredentialPart::Remove => action(true, Box::new(move || on_effect.call(Effect::RemoveToken(provider)))),
                }
            }
            RowKind::Global { action } => {
                let action = *action;
                described(SettingRow::new(row.label, RowControl::Action { danger: false }, density).on_press(move |()| {
                    on_effect.call(match action {
                        GlobalAction::OpenLogFolder => Effect::OpenLogFolder,
                        GlobalAction::SaveDiagnostics => Effect::SaveDiagnostics,
                        GlobalAction::UpdateSources => Effect::UpdateSources,
                    });
                }))
                .into_element()
            }
            RowKind::Launch { key } => {
                let ctx = LaunchContext { env: &self.display, projects: &self.projects, prefs: &self.launch };
                let Some(control) = ctx.control(target, *key) else { return rect().into_element() };
                let (label, key, dialogs) = (row.label, *key, self.dialogs);
                let labels: Vec<String> = control.options.iter().map(|o| o.label.clone()).collect();
                let values: Vec<_> = control.options.iter().map(|o| o.value.clone()).collect();
                described(
                    SettingRow::new(row.label, RowControl::Value { text: control.summary.clone(), opens_menu: true }, density).on_press_at(
                        move |at: (f32, f32)| {
                            let (on_effect, values) = (on_effect.clone(), values.clone());
                            let pick = EventHandler::new(move |i: usize| {
                                if let Some(value) = values.get(i) {
                                    on_effect.call(Effect::LaunchSetting { app: target.app(), key, value: value.clone() });
                                }
                            });
                            dialogs.pick(OpenPicker::new(label, labels.clone(), control.selected, at, pick));
                        },
                    ),
                )
                .into_element()
            }
        }
    }
}

impl Component for Form {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let Some(section) = self.schema.sections.get(self.section) else { return rect().into_element() };
        let groups = section.groups.iter().enumerate().map(|(g, group)| {
            rect()
                .vertical()
                .spacing(SPACE_2)
                .width(Size::fill())
                .maybe(g > 0, |el| el.margin(Gaps::new(GROUP_GAP, 0., 0., 0.)))
                .maybe_child(
                    group.heading.map(|h| rect().padding(Gaps::new(SPACE_2, 0., 0., 0.)).child(TypeStyle::Eyebrow.text(h, t.ink_subtle))),
                )
                .maybe_child(group.note.map(|n| TypeStyle::Meta.text(n, t.ink_muted)))
                .children(group.rows.iter().map(|row| self.row(row)))
                .into_element()
        });
        rect().vertical().width(Size::fill()).children(groups).into_element()
    }
}
