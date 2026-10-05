use freya::prelude::*;

use crate::{
    metrics::Density,
    surface::{RowControl, SettingRow},
};

/// A text setting. It owns the field's text (a hook per row, so it is its own component) and
/// reports every change; the store coalesces the saving.
#[derive(Clone, PartialEq)]
pub(super) struct TextRow {
    pub label: &'static str,
    pub description: Option<&'static str>,
    pub placeholder: &'static str,
    /// The saved text, the first time the row is shown.
    pub initial: String,
    pub density: Density,
    pub on_change: EventHandler<String>,
    pub key: DiffKey,
}

impl KeyExt for TextRow {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for TextRow {
    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }

    fn render(&self) -> impl IntoElement {
        let input = use_state({
            let initial = self.initial.clone();
            move || initial
        });
        let mut last = use_state({
            let initial = self.initial.clone();
            move || initial
        });
        let a11y = use_a11y();
        let on_change = self.on_change.clone();
        use_side_effect(move || {
            let text = input.read().clone();
            if text != *last.peek() {
                last.set(text.clone());
                on_change.call(text);
            }
        });
        let mut row = SettingRow::new(
            self.label,
            RowControl::Text { input, placeholder: self.placeholder.to_string(), a11y, secret: false },
            self.density,
        );
        if let Some(d) = self.description {
            row = row.description(d);
        }
        row
    }
}
