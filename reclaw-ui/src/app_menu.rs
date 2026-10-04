//! The per-app Options menu, shared by every interface: Deck mode opens it centered over a darkened
//! screen, the desktop opens it anchored under the Manage button. Only the presentation differs.
use crate::{
    effect::Effect,
    model::GameEntry,
    surface::{MenuEntry, MenuState},
};

/// Choices in the Options menu and in pickers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    ToggleFavorite,
    AddTo(&'static str),
    NewCollection,
    OpenFolder,
    Verify,
    CheckUpdate,
    Uninstall,
    Properties,
    Cancel,
    Choice(usize),
}

impl MenuAction {
    /// The host command for a choice that needs nothing more from the UI. `Uninstall` asks for a
    /// confirmation first and `Properties` navigates, so both are `None`, as are `Cancel` and
    /// picker choices.
    pub fn effect(self, app: u32) -> Option<Effect> {
        Some(match self {
            Self::ToggleFavorite => Effect::ToggleFavorite(app),
            Self::AddTo(name) => Effect::AddToCollection { app, name },
            Self::NewCollection => Effect::NewCollection(app),
            Self::OpenFolder => Effect::OpenFolder(app),
            Self::Verify => Effect::Verify(app),
            Self::CheckUpdate => Effect::CheckUpdate(app),
            Self::Uninstall | Self::Properties | Self::Cancel | Self::Choice(_) => return None,
        })
    }
}

impl MenuAction {
    /// Like [`effect`](Self::effect), for an interface that has already asked "are you sure?":
    /// `Uninstall` is now just the command.
    pub fn confirmed_effect(self, app: u32) -> Option<Effect> {
        match self {
            Self::Uninstall => Some(Effect::Uninstall(app)),
            other => other.effect(app),
        }
    }
}

/// Collections an app can be filed under. A catalog will supply real ones.
pub const COLLECTIONS: [&str; 3] = ["Handheld friendly", "Backlog", "Recompiled N64"];

/// The Options menu of one app, in Big Picture's shape: a flat list with group separators,
/// `Add to` and `Manage` opening submenus, Properties, then Cancel.
///
/// `with_properties` is false where there is no Properties page to open (the desktop for now).
pub fn options_menu(game: &GameEntry, with_properties: bool) -> MenuState<MenuAction> {
    let favorite = game.tags.iter().any(|t| t == "favorite");
    let installed = game.status.is_installed();
    let add_to: Vec<_> = COLLECTIONS
        .iter()
        .map(|c| MenuEntry::action(*c, MenuAction::AddTo(c)))
        .chain([MenuEntry::action("New collection...", MenuAction::NewCollection).separated()])
        .collect();
    let manage = vec![
        MenuEntry::action("Open install folder", MenuAction::OpenFolder),
        MenuEntry::action("Verify files", MenuAction::Verify),
        MenuEntry::action("Check for updates", MenuAction::CheckUpdate),
        MenuEntry::action("Uninstall", MenuAction::Uninstall).separated(),
    ];
    // Nothing to manage until the app is installed.
    let manage: Vec<_> = manage.into_iter().map(|e| if installed { e } else { e.disabled() }).collect();
    let mut items = vec![
        MenuEntry::action(if favorite { "Remove from favorites" } else { "Add to favorites" }, MenuAction::ToggleFavorite),
        MenuEntry::submenu("Add to", "Add to", add_to),
        MenuEntry::submenu("Manage", "Manage", manage),
    ];
    if with_properties {
        items.push(MenuEntry::action("Properties...", MenuAction::Properties).separated());
    }
    items.push(MenuEntry::action("Cancel", MenuAction::Cancel).separated());
    MenuState::new(game.title.to_string(), items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sample::sample_games, surface::EntryKind};

    fn manage_entries(game: &GameEntry) -> Vec<bool> {
        let menu = options_menu(game, true);
        let root = &menu.levels()[0];
        let manage = root.entries.iter().find(|e| e.label == "Manage").expect("Manage is always listed");
        match &manage.kind {
            EntryKind::Submenu { items, .. } => items.iter().map(|e| e.enabled).collect(),
            EntryKind::Action(_) => panic!("Manage opens a submenu"),
        }
    }

    #[test]
    fn manage_is_disabled_until_installed() {
        let games = sample_games();
        let installed = games.iter().find(|g| g.status.is_installed()).unwrap();
        let available = games.iter().find(|g| !g.status.is_installed()).unwrap();
        assert!(manage_entries(installed).iter().all(|on| *on));
        assert!(manage_entries(available).iter().all(|on| !*on));
    }

    #[test]
    fn properties_can_be_left_out() {
        let game = sample_games().remove(0);
        let has = |with| options_menu(&game, with).levels()[0].entries.iter().any(|e| e.label == "Properties...");
        assert!(has(true));
        assert!(!has(false));
    }

    #[test]
    fn only_self_contained_choices_become_effects() {
        assert_eq!(MenuAction::Verify.effect(7), Some(Effect::Verify(7)));
        assert_eq!(MenuAction::AddTo("Backlog").effect(7), Some(Effect::AddToCollection { app: 7, name: "Backlog" }));
        for needs_ui in [MenuAction::Uninstall, MenuAction::Properties, MenuAction::Cancel, MenuAction::Choice(1)] {
            assert_eq!(needs_ui.effect(7), None, "{needs_ui:?}");
        }
        assert_eq!(MenuAction::Uninstall.confirmed_effect(7), Some(Effect::Uninstall(7)));
    }

    #[test]
    fn favorites_label_follows_the_tag() {
        let mut game = sample_games().remove(0);
        let label = |g: &GameEntry| options_menu(g, true).levels()[0].entries[0].label.to_string();
        assert_eq!(label(&game), "Add to favorites");
        game.tags.push("favorite".into());
        assert_eq!(label(&game), "Remove from favorites");
    }
}
