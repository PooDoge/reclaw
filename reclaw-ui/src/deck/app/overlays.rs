//! Slide-in panels, the cascading menu and the confirmation: everything that sits over a page.
use freya::prelude::*;

use super::frame::Frame;
use crate::{
    deck::{ConfirmCopy, ConfirmKind, ConfirmOverlay, MainMenu, NoticeDetails, NoticeToast, Overlay, PanelSide, QuickAccess, SlidePanel},
    metrics::*,
    surface::{MenuLevelView, MenuPlacement, ModalMenu},
};

pub(super) fn overlays(f: &Frame) -> Vec<Element> {
    let state = &f.state;
    let active = f.games.iter().find(|x| x.run.is_active()).cloned();
    let mut out = vec![
        SlidePanel::new(
            PanelSide::Left,
            state.overlay() == Overlay::MainMenu,
            f.window,
            f.dismiss.clone(),
            MainMenu::new(state.section(), state.focus(), f.ring, f.click.clone()),
        )
        .into_element(),
        SlidePanel::new(
            PanelSide::Right,
            state.overlay() == Overlay::QuickAccess,
            f.window,
            f.dismiss.clone(),
            QuickAccess::new(active, f.pad.clone(), f.downloads.len(), state.focus(), f.ring, f.click.clone()),
        )
        .into_element(),
    ];
    if let (Overlay::Menu(_), Some(menu)) = (state.overlay(), state.menu()) {
        out.push(
            ModalMenu::new(
                MenuLevelView::from_state(menu),
                MenuPlacement::Centered,
                f.window,
                Density::Controller,
                f.pick.clone(),
                f.dismiss.clone(),
            )
            .ring_visible(f.ring)
            .into_element(),
        );
    }
    if let Overlay::Confirm(ConfirmKind::Uninstall(id)) = state.overlay() {
        let name = f.game(id).map(|g| g.title.to_string()).unwrap_or_default();
        out.push(
            ConfirmOverlay::new(
                ConfirmCopy {
                    title: format!("Uninstall {name}?"),
                    message: "Removes it from this device. Your own game file is never touched.",
                    action_label: "Uninstall",
                },
                state.focus(),
                f.ring,
                f.window,
                f.click.clone(),
                f.dismiss.clone(),
            )
            .into_element(),
        );
    }
    if let Some(notice) = f.notice_details.clone() {
        out.push(NoticeDetails::new(notice, state.focus(), f.ring, f.window, f.click.clone(), f.dismiss.clone()).into_element());
    }
    // The toast is on screen only while nothing else is: the reducer decides (`toast_visible`).
    if let Some(notice) = f.toast.clone() {
        out.push(NoticeToast::new(notice, f.holding, f.kind, f.last_input, f.window).into_element());
    }
    out
}
