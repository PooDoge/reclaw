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
    AddToLibrary,
    RemoveFromLibrary,
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
            Self::AddToLibrary => Effect::AddToLibrary(app),
            Self::RemoveFromLibrary => Effect::RemoveFromLibrary(app),
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

/// The Options menu of one app, in Big Picture's shape: a flat list with group separators,
/// `Manage` opening a submenu, Properties, then Cancel.
///
/// `with_properties` is false where there is no Properties page to open (the desktop for now).
pub fn options_menu(game: &GameEntry, with_properties: bool) -> MenuState<MenuAction> {
    let favorite = game.tags.iter().any(|t| t == "favorite");
    let installed = game.status.is_installed();
    // What needs the app on disk is disabled until it is installed; leaving the library is always possible.
    let on_disk = |entry: MenuEntry<MenuAction>| if installed { entry } else { entry.disabled() };
    let mut manage = vec![
        on_disk(MenuEntry::action("Open install folder", MenuAction::OpenFolder)),
        on_disk(MenuEntry::action("Verify files", MenuAction::Verify)),
        on_disk(MenuEntry::action("Check for updates", MenuAction::CheckUpdate)),
    ];
    if game.in_library {
        manage.push(MenuEntry::action("Remove from library", MenuAction::RemoveFromLibrary).separated());
    }
    let uninstall = MenuEntry::action("Uninstall", MenuAction::Uninstall);
    manage.push(on_disk(if game.in_library { uninstall } else { uninstall.separated() }));
    let mut items =
        vec![MenuEntry::action(if favorite { "Remove from favorites" } else { "Add to favorites" }, MenuAction::ToggleFavorite)];
    if !game.in_library {
        items.push(MenuEntry::action("Add to library", MenuAction::AddToLibrary));
    }
    items.push(MenuEntry::submenu("Manage", "Manage", manage));
    if with_properties {
        items.push(MenuEntry::action("Properties...", MenuAction::Properties).separated());
    }
    items.push(MenuEntry::action("Cancel", MenuAction::Cancel).separated());
    MenuState::new(game.title.to_string(), items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures::sample_games, surface::EntryKind};

    fn manage_entries(game: &GameEntry) -> Vec<bool> {
        let menu = options_menu(game, true);
        let root = &menu.levels()[0];
        let manage = root.entries.iter().find(|e| e.label == "Manage").expect("Manage is always listed");
        match &manage.kind {
            EntryKind::Submenu { items, .. } => items.iter().map(|e| e.enabled).collect(),
            EntryKind::Action(_) => panic!("Manage opens a submenu"),
        }
    }

    fn root_labels(game: &GameEntry) -> Vec<String> {
        options_menu(game, true).levels()[0].entries.iter().map(|e| e.label.to_string()).collect()
    }

    #[test]
    fn what_needs_files_on_disk_is_disabled_until_installed_but_leaving_the_library_is_not() {
        let games = sample_games();
        let installed = games.iter().find(|g| g.status.is_installed()).unwrap();
        let available = games.iter().find(|g| !g.status.is_installed()).unwrap();
        assert!(manage_entries(installed).iter().all(|on| *on));
        // Open folder, Verify, Check for updates, Remove from library, Uninstall.
        assert_eq!(manage_entries(available), [false, false, false, true, false]);
    }

    #[test]
    fn a_project_that_is_not_in_the_library_offers_to_add_it_and_one_that_is_offers_to_remove_it() {
        let mut game = sample_games().remove(3);
        assert!(game.in_library);
        assert!(!root_labels(&game).contains(&"Add to library".to_string()));
        game.in_library = false;
        assert!(root_labels(&game).contains(&"Add to library".to_string()));
        assert_eq!(manage_entries(&game), [false, false, false, false], "nothing to remove, and nothing installed to manage");
        assert_eq!(MenuAction::AddToLibrary.effect(7), Some(Effect::AddToLibrary(7)));
        assert_eq!(MenuAction::RemoveFromLibrary.effect(7), Some(Effect::RemoveFromLibrary(7)));
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
