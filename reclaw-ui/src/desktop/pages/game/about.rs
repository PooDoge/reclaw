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
            // The one-line summary leads, unless the description already begins with it.
            .maybe(!project.description.starts_with(&project.summary), |el| el.child(TypeStyle::Body.text(project.summary.clone(), t.ink)))
            .child(
                TypeStyle::Body
                    .text(project.description.clone(), if project.description.starts_with(&project.summary) { t.ink } else { t.ink_muted }),
            )
            .into_element(),
    )
}
