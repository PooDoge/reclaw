//! Cascading option menus as plain data, in the style of Big Picture's context menus: a flat list
//! with group separators, `>` entries that open a submenu to the right, and a Cancel row.
//!
//! `MenuState` is the whole behavior (focus per level, open submenus, Back), so it is tested
//! without a window. Rendering is `ModalMenu`.
use reclaw_input::Direction;

#[derive(Clone, PartialEq, Debug)]
pub enum EntryKind<A> {
    Action(A),
    Submenu { title: String, items: Vec<MenuEntry<A>> },
}

#[derive(Clone, PartialEq, Debug)]
pub struct MenuEntry<A> {
    pub label: String,
    pub kind: EntryKind<A>,
    /// Draw the thick group separator above this row.
    pub separator_before: bool,
    pub enabled: bool,
}

impl<A> MenuEntry<A> {
    pub fn action(label: impl Into<String>, action: A) -> Self {
        Self { label: label.into(), kind: EntryKind::Action(action), separator_before: false, enabled: true }
    }

    pub fn submenu(label: impl Into<String>, title: impl Into<String>, items: Vec<MenuEntry<A>>) -> Self {
        Self { label: label.into(), kind: EntryKind::Submenu { title: title.into(), items }, separator_before: false, enabled: true }
    }

    pub fn separated(mut self) -> Self {
        self.separator_before = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// What a press produced.
#[derive(Clone, PartialEq, Debug)]
pub enum Outcome<A> {
    /// Nothing happened (edge, disabled row).
    None,
    /// A submenu opened or closed; the menu stays up.
    Moved,
    Chose(A),
    /// Back at the root: dismiss the menu.
    Closed,
}

/// One visible level, for rendering.
#[derive(Clone, PartialEq, Debug)]
pub struct Level<'a, A> {
    pub title: &'a str,
    pub entries: &'a [MenuEntry<A>],
    pub focus: usize,
    /// The row in this level whose submenu is open (the next level), if any.
    pub open: Option<usize>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MenuState<A> {
    title: String,
    root: Vec<MenuEntry<A>>,
    /// For each open submenu, the index of the entry that opened it.
    path: Vec<usize>,
    /// Focused row per level; `focus.len() == path.len() + 1`.
    focus: Vec<usize>,
}

impl<A: Clone> MenuState<A> {
    pub fn new(title: impl Into<String>, items: Vec<MenuEntry<A>>) -> Self {
        let first = items.iter().position(|e| e.enabled).unwrap_or(0);
        Self { title: title.into(), root: items, path: Vec::new(), focus: vec![first] }
    }

    fn items_at(&self, depth: usize) -> &[MenuEntry<A>] {
        let mut items: &[MenuEntry<A>] = &self.root;
        for index in &self.path[..depth] {
            if let EntryKind::Submenu { items: sub, .. } = &items[*index].kind {
                items = sub;
            }
        }
        items
    }

    fn title_at(&self, depth: usize) -> &str {
        if depth == 0 {
            return &self.title;
        }
        match &self.items_at(depth - 1)[self.path[depth - 1]].kind {
            EntryKind::Submenu { title, .. } => title,
            EntryKind::Action(_) => &self.title,
        }
    }

    pub fn depth(&self) -> usize {
        self.path.len()
    }

    pub fn levels(&self) -> Vec<Level<'_, A>> {
        (0..=self.path.len())
            .map(|d| Level { title: self.title_at(d), entries: self.items_at(d), focus: self.focus[d], open: self.path.get(d).copied() })
            .collect()
    }

    pub fn navigate(&mut self, dir: Direction) -> Outcome<A> {
        let depth = self.path.len();
        match dir {
            Direction::Up | Direction::Down => {
                let items = self.items_at(depth);
                let step: isize = if dir == Direction::Down { 1 } else { -1 };
                let mut i = self.focus[depth] as isize + step;
                while i >= 0 && (i as usize) < items.len() {
                    if items[i as usize].enabled {
                        self.focus[depth] = i as usize;
                        return Outcome::Moved;
                    }
                    i += step;
                }
                Outcome::None
            }
            Direction::Right => {
                if matches!(self.items_at(depth)[self.focus[depth]].kind, EntryKind::Submenu { .. }) {
                    self.confirm()
                } else {
                    Outcome::None
                }
            }
            Direction::Left => {
                if depth > 0 {
                    self.pop();
                    Outcome::Moved
                } else {
                    Outcome::None
                }
            }
        }
    }

    fn pop(&mut self) {
        self.path.pop();
        self.focus.pop();
    }

    /// Confirm the focused row of the deepest level.
    pub fn confirm(&mut self) -> Outcome<A> {
        let depth = self.path.len();
        let index = self.focus[depth];
        let entry = &self.items_at(depth)[index];
        if !entry.enabled {
            return Outcome::None;
        }
        match &entry.kind {
            EntryKind::Action(a) => Outcome::Chose(a.clone()),
            EntryKind::Submenu { items, .. } => {
                let first = items.iter().position(|e| e.enabled).unwrap_or(0);
                self.path.push(index);
                self.focus.push(first);
                Outcome::Moved
            }
        }
    }

    /// Back: closes the deepest submenu, or dismisses the menu from the root.
    pub fn back(&mut self) -> Outcome<A> {
        if self.path.is_empty() {
            Outcome::Closed
        } else {
            self.pop();
            Outcome::Moved
        }
    }

    /// Pointer or touch press on `index` of `level`: close anything deeper, focus it, confirm it.
    pub fn pick(&mut self, level: usize, index: usize) -> Outcome<A> {
        if level > self.path.len() || index >= self.items_at(level).len() {
            return Outcome::None;
        }
        while self.path.len() > level {
            self.pop();
        }
        self.focus[level] = index;
        self.confirm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Debug)]
    enum A {
        Favorite,
        Collection(&'static str),
        Open,
        Properties,
        Cancel,
    }

    fn menu() -> MenuState<A> {
        MenuState::new(
            "Starfall 64",
            vec![
                MenuEntry::action("Add to favorites", A::Favorite),
                MenuEntry::submenu(
                    "Add to",
                    "Add to",
                    vec![
                        MenuEntry::action("Controller", A::Collection("Controller")),
                        MenuEntry::action("Sandbox", A::Collection("Sandbox")),
                        MenuEntry::action("New collection...", A::Collection("")).separated(),
                    ],
                ),
                MenuEntry::submenu("Manage", "Manage", vec![MenuEntry::action("Open folder", A::Open)]),
                MenuEntry::action("Properties...", A::Properties).separated(),
                MenuEntry::action("Cancel", A::Cancel).separated(),
            ],
        )
    }

    #[test]
    fn starts_on_the_first_enabled_row() {
        let m = menu();
        assert_eq!(m.depth(), 0);
        assert_eq!(m.levels()[0].focus, 0);
        let off = MenuState::new("t", vec![MenuEntry::action("a", A::Open).disabled(), MenuEntry::action("b", A::Cancel)]);
        assert_eq!(off.levels()[0].focus, 1);
    }

    #[test]
    fn up_and_down_stop_at_the_ends_and_skip_disabled_rows() {
        let mut m = MenuState::new(
            "t",
            vec![MenuEntry::action("a", A::Open), MenuEntry::action("b", A::Favorite).disabled(), MenuEntry::action("c", A::Cancel)],
        );
        assert_eq!(m.navigate(Direction::Up), Outcome::None);
        m.navigate(Direction::Down);
        assert_eq!(m.levels()[0].focus, 2, "skipped the disabled row");
        assert_eq!(m.navigate(Direction::Down), Outcome::None);
    }

    #[test]
    fn confirming_an_action_chooses_it() {
        let mut m = menu();
        assert_eq!(m.confirm(), Outcome::Chose(A::Favorite));
    }

    #[test]
    fn a_submenu_opens_to_the_right_and_titles_the_new_level() {
        let mut m = menu();
        m.navigate(Direction::Down);
        assert_eq!(m.navigate(Direction::Right), Outcome::Moved);
        let levels = m.levels();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[1].title, "Add to");
        assert_eq!(levels[0].open, Some(1), "the parent remembers which row is open");
        assert_eq!(m.confirm(), Outcome::Chose(A::Collection("Controller")));
    }

    #[test]
    fn back_and_left_close_one_level_then_dismiss() {
        let mut m = menu();
        m.navigate(Direction::Down);
        m.confirm();
        assert_eq!(m.depth(), 1);
        assert_eq!(m.back(), Outcome::Moved);
        assert_eq!(m.depth(), 0);
        assert_eq!(m.levels()[0].focus, 1, "focus is where the submenu was opened from");
        m.navigate(Direction::Right);
        assert_eq!(m.navigate(Direction::Left), Outcome::Moved);
        assert_eq!(m.back(), Outcome::Closed);
    }

    #[test]
    fn right_on_an_action_does_nothing() {
        let mut m = menu();
        assert_eq!(m.navigate(Direction::Right), Outcome::None);
    }

    #[test]
    fn pointer_pick_in_a_parent_closes_the_child_first() {
        let mut m = menu();
        m.navigate(Direction::Down);
        m.confirm(); // open "Add to"
        // Click "Manage" in the root level while "Add to" is open: it replaces the open submenu.
        assert_eq!(m.pick(0, 2), Outcome::Moved);
        assert_eq!(m.levels()[1].title, "Manage");
        // Click an action in the child.
        assert_eq!(m.pick(1, 0), Outcome::Chose(A::Open));
        // Out of range is ignored.
        assert_eq!(m.pick(5, 0), Outcome::None);
        assert_eq!(m.pick(0, 99), Outcome::None);
    }

    #[test]
    fn disabled_rows_cannot_be_chosen() {
        let mut m = MenuState::new("t", vec![MenuEntry::action("a", A::Open).disabled()]);
        assert_eq!(m.pick(0, 0), Outcome::None);
    }
}
