//! How a search's results are introduced and what an empty one says: the line above them, with Clear, and a title with the
//! words that matched highlighted.
use freya::prelude::*;

use crate::{
    metrics::*,
    prelude::*,
    search::{SearchScope, highlight, results_line},
    typography::TypeStyle,
};

/// The line above a search's results: what was searched, where, how many were found, and Clear. `narrow` (a sidebar) drops
/// the place and leaves Clear as its icon.
pub fn results_bar(t: &Reclaw, scope: SearchScope, query: &str, found: usize, narrow: bool, on_clear: EventHandler<()>) -> Rect {
    let clear = ActionButton::new(ButtonVariant::Ghost).icon(IconName::X).alt("Clear search").on_press(move |_| on_clear.call(()));
    rect()
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(SPACE_3)
        .width(Size::fill())
        .padding(Gaps::new(SPACE_2, SPACE_2, SPACE_2, SPACE_4))
        .background(t.bg_panel)
        .corner_radius(RADIUS_MD)
        .border(Border::new().fill(t.accent).width(BorderWidth { left: 3., ..Default::default() }).alignment(BorderAlignment::Inner))
        .child(icon(IconName::Search, 16., t.accent))
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(SPACE_2)
                .width(Size::flex(1.))
                .child(TypeStyle::Label.text(results_line(found, query), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis))
                .maybe_child((!narrow).then(|| TypeStyle::Meta.text(scope.place(), t.ink_subtle))),
        )
        .child(if narrow { clear } else { clear.label("Clear search") })
}

/// A search that found nothing: say what was searched where, and offer the way back to everything.
pub fn no_results(t: &Reclaw, scope: SearchScope, query: &str, hint: &'static str, on_clear: EventHandler<()>) -> Rect {
    rect()
        .vertical()
        .spacing(SPACE_3)
        .width(Size::fill())
        .padding(SPACE_6)
        .center()
        .child(icon(IconName::Search, 28., t.ink_subtle))
        .child(TypeStyle::Heading.text(format!("Nothing {} matches \u{201c}{}\u{201d}", scope.place(), query.trim()), t.ink))
        .child(TypeStyle::Body.text(hint, t.ink_muted))
        .child(ActionButton::new(ButtonVariant::Secondary).icon(IconName::X).label("Clear search").on_press(move |_| on_clear.call(())))
}

/// `text` in `style`, the parts that match `query` in the accent colour and bold.
pub fn highlighted(t: &Reclaw, text: &str, query: &str, style: TypeStyle, color: Color) -> Paragraph {
    let (size, weight, family) = style.spec();
    paragraph()
        .font_size(size)
        .font_family(family)
        .font_weight(weight)
        .color(color)
        .max_lines(1)
        .text_overflow(TextOverflow::Ellipsis)
        .spans_iter(highlight(text, query).into_iter().map(|piece| {
            let span = Span::new(piece.text);
            if piece.hit { span.color(t.accent).font_weight(FontWeight::BOLD) } else { span }
        }))
}
