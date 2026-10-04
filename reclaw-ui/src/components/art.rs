use freya::prelude::*;

use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Stand-in for catalog art (capsule 3:4, hero 16:5). Real art replaces it; this never
/// pretends to be a game image, it names the art role in mono.
#[derive(Clone, PartialEq)]
pub struct ArtPlaceholder {
    role: &'static str,
    width: Size,
    height: Size,
    tag_at_end: bool,
}

impl ArtPlaceholder {
    pub fn new(role: &'static str, width: Size, height: Size) -> Self {
        Self { role, width, height, tag_at_end: true }
    }

    pub fn capsule(width: Size) -> Self {
        Self::new("CAPSULE 3:4", width, Size::px(CAPSULE_H))
    }

    pub fn hero(height: f32) -> Self {
        Self { tag_at_end: false, ..Self::new("HERO 16:5", Size::fill(), Size::px(height)) }
    }

    pub fn thumb(width: f32, height: f32) -> Self {
        Self::new("HDR", Size::px(width), Size::px(height))
    }
}

impl Component for ArtPlaceholder {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        rect()
            .width(self.width.clone())
            .height(self.height.clone())
            .background(t.bg_raised)
            .main_align(if self.tag_at_end { Alignment::End } else { Alignment::Start })
            .child(
                rect()
                    .padding(Gaps::new(SPACE_1, SPACE_2, SPACE_1, SPACE_2))
                    .background(t.bg_panel)
                    .child(TypeStyle::Mono.text(self.role, t.ink_subtle)),
            )
    }
}
