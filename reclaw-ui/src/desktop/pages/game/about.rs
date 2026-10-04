use freya::prelude::*;

use super::ctx::Ctx;
use crate::{desktop::pages::common::heading, metrics::*, typography::TypeStyle};

/// The project's own description. Nothing when the catalog has none.
pub(super) fn view(c: &Ctx) -> Option<Element> {
    let project = c.view.project.as_ref()?;
    let t = &c.t;
    Some(
        rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(heading(t, "About"))
            .child(TypeStyle::Body.text(project.summary.clone(), t.ink))
            .child(TypeStyle::Body.text(project.description.clone(), t.ink_muted))
            .into_element(),
    )
}
