use freya::prelude::*;

use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Sections that have no content yet say so plainly.
#[derive(Clone, PartialEq)]
pub struct EmptyPage {
    title: &'static str,
    detail: &'static str,
}

impl EmptyPage {
    pub fn new(title: &'static str, detail: &'static str) -> Self {
        Self { title, detail }
    }
}

impl Component for EmptyPage {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        rect()
            .vertical()
            .center()
            .spacing(SPACE_2)
            .width(Size::fill())
            .height(Size::fill())
            .child(TypeStyle::DeckHeading.text(self.title, t.ink))
            .child(TypeStyle::DeckBody.text(self.detail, t.ink_muted))
    }
}
