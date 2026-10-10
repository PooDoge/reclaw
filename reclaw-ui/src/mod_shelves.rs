//! The Mods tab's shelves: what is installed, then the mods the sites rank highest by downloads, by rating, by last update and
//! by release. Each shelf shows a few and opens into the whole list a page at a time. A search replaces the shelves with one
//! list of what it finds. Pure; `desktop/pages/mods` draws it.
//!
//! A shelf is built from where the sites put each mod in that order ([`ModRanks`](crate::model::ModRanks)), so "Recently
//! updated" is what the site says, not a guess from the most downloaded few. Mods of two sites on one shelf are merged by the
//! number the order is about (downloads, ratings, dates) where both say it, and by their place on their own site otherwise.
use std::cmp::Reverse;

use crate::{model::ModEntry, search::matches_all};

/// How many mods a shelf shows before it is opened.
pub const PREVIEW: usize = 4;
/// How many mods an opened shelf, or a search, shows on a page.
pub const PAGE_SIZE: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shelf {
    Installed,
    Popular,
    TopRated,
    Updated,
    Newest,
}

impl Shelf {
    pub const ALL: [Shelf; 5] = [Self::Installed, Self::Popular, Self::TopRated, Self::Updated, Self::Newest];

    pub fn title(self) -> &'static str {
        match self {
            Self::Installed => "Installed",
            Self::Popular => "Most downloaded",
            Self::TopRated => "Top rated",
            Self::Updated => "Recently updated",
            Self::Newest => "New releases",
        }
    }

    /// One line under the title saying what the order is.
    pub fn note(self) -> &'static str {
        match self {
            Self::Installed => "Updates first",
            Self::Popular => "All-time downloads",
            Self::TopRated => "Thunderstore ratings and GameBanana likes",
            Self::Updated => "Newest version first",
            Self::Newest => "Most recently published",
        }
    }
}

fn rank(place: Option<u32>) -> u32 {
    place.unwrap_or(u32::MAX)
}

/// The mods on one shelf, in its order.
pub fn shelf(mods: &[ModEntry], shelf: Shelf) -> Vec<ModEntry> {
    let mut on: Vec<ModEntry> = match shelf {
        Shelf::Installed => mods.iter().filter(|m| m.status.is_installed()).cloned().collect(),
        Shelf::Popular => mods.iter().filter(|m| m.ranks.downloads.is_some()).cloned().collect(),
        Shelf::TopRated => mods.iter().filter(|m| m.ranks.rating.is_some()).cloned().collect(),
        Shelf::Updated => mods.iter().filter(|m| m.ranks.updated.is_some()).cloned().collect(),
        Shelf::Newest => mods.iter().filter(|m| m.ranks.newest.is_some()).cloned().collect(),
    };
    // Sorts are stable, and the title last keeps two equal mods in one order between renders.
    match shelf {
        Shelf::Installed => {
            on.sort_by_cached_key(|m| (m.status != crate::model::ModStatus::UpdateReady, m.title.to_lowercase()));
        }
        Shelf::Popular => on.sort_by_cached_key(|m| (Reverse(m.downloads), rank(m.ranks.downloads), m.title.to_lowercase())),
        Shelf::TopRated => on.sort_by_cached_key(|m| (Reverse(m.rating), rank(m.ranks.rating), m.title.to_lowercase())),
        Shelf::Updated => on.sort_by_cached_key(|m| (Reverse(m.updated.unwrap_or(0)), rank(m.ranks.updated), m.title.to_lowercase())),
        Shelf::Newest => on.sort_by_cached_key(|m| (Reverse(m.created.unwrap_or(0)), rank(m.ranks.newest), m.title.to_lowercase())),
    }
    on
}

/// The shelves that have anything on them, in order.
pub fn shelves(mods: &[ModEntry]) -> Vec<(Shelf, Vec<ModEntry>)> {
    Shelf::ALL.into_iter().map(|s| (s, shelf(mods, s))).filter(|(_, on)| !on.is_empty()).collect()
}

/// What a search finds among `mods`: every word in the title, author, summary or tags. Mods whose title has every word come
/// first, then the most downloaded.
pub fn search(mods: &[ModEntry], query: &str) -> Vec<ModEntry> {
    let mut found: Vec<ModEntry> = mods
        .iter()
        .filter(|m| {
            let mut fields = vec![m.title.as_str(), m.author.as_str(), m.summary.as_str()];
            fields.extend(m.tags.iter().map(String::as_str));
            matches_all(&fields, query)
        })
        .cloned()
        .collect();
    found.sort_by_cached_key(|m| (!matches_all(&[m.title.as_str()], query), Reverse(m.downloads), m.title.to_lowercase()));
    found
}

/// One page of a long list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Paging {
    /// 0 first; past the end it is the last page.
    pub page: usize,
    pub pages: usize,
    /// The items it shows, `start..end`.
    pub start: usize,
    pub end: usize,
    pub total: usize,
}

impl Paging {
    pub fn of(total: usize, page: usize) -> Self {
        let pages = total.div_ceil(PAGE_SIZE).max(1);
        let page = page.min(pages - 1);
        let start = (page * PAGE_SIZE).min(total);
        Self { page, pages, start, end: (start + PAGE_SIZE).min(total), total }
    }

    pub fn has_previous(&self) -> bool {
        self.page > 0
    }

    pub fn has_next(&self) -> bool {
        self.page + 1 < self.pages
    }

    /// "21–40 of 57"; "3 of 3" when it all fits.
    pub fn label(&self) -> String {
        if self.pages == 1 {
            format!("{} of {}", self.total, self.total)
        } else {
            format!("{}\u{2013}{} of {}", self.start + 1, self.end, self.total)
        }
    }
}

/// Which shelf is opened, and on which page.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OpenShelf {
    pub shelf: Shelf,
    pub page: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fixtures::{ranked, sample_mods},
        model::{ModRanks, ModStatus},
    };

    fn ids(mods: &[ModEntry]) -> Vec<&str> {
        mods.iter().map(|m| m.id.as_str()).collect()
    }

    #[test]
    fn installed_comes_first_with_updates_on_top() {
        let mut mods = sample_mods();
        mods[1].status = ModStatus::UpdateReady;
        assert_eq!(ids(&shelf(&mods, Shelf::Installed)), ["free-camera", "hd-textures"]);
        assert_eq!(shelves(&mods)[0].0, Shelf::Installed);
    }

    #[test]
    fn two_sites_merge_by_the_number_the_order_is_about() {
        let mods = sample_mods();
        assert_eq!(ids(&shelf(&mods, Shelf::Popular)), ["hd-textures", "randomizer", "free-camera", "hard-mode", "ghost-data"]);
        assert_eq!(ids(&shelf(&mods, Shelf::Updated))[0], "ghost-data", "the sample's least downloaded was updated last");
        assert_eq!(ids(&shelf(&mods, Shelf::Newest))[0], "hd-textures");
    }

    #[test]
    fn without_dates_a_shelf_follows_the_sites_own_places() {
        let mut mods = ranked(sample_mods());
        for m in &mut mods {
            m.updated = None;
        }
        let updated = shelf(&mods, Shelf::Updated);
        assert!(updated.windows(2).all(|w| w[0].ranks.updated <= w[1].ranks.updated), "{:?}", ids(&updated));
    }

    #[test]
    fn a_mod_the_site_did_not_rank_in_an_order_is_not_on_that_shelf() {
        let mut mods = sample_mods();
        mods[0].ranks = ModRanks { downloads: Some(0), ..ModRanks::default() };
        assert!(!ids(&shelf(&mods, Shelf::Newest)).contains(&"hd-textures"));
        assert!(ids(&shelf(&mods, Shelf::Popular)).contains(&"hd-textures"));
        let none: Vec<ModEntry> = sample_mods().into_iter().map(|m| ModEntry { ranks: ModRanks::default(), ..m }).collect();
        assert!(shelves(&none).iter().all(|(s, _)| *s == Shelf::Installed), "only Installed is left");
    }

    #[test]
    fn a_search_puts_title_matches_first() {
        let mods = sample_mods();
        assert_eq!(ids(&search(&mods, "camera")), ["free-camera"]);
        assert_eq!(ids(&search(&mods, "hard")), ["hard-mode"]);
        // "mode" is in Hard Mode's title and in Free Camera's summary ("photo mode").
        assert_eq!(ids(&search(&mods, "mode")), ["hard-mode", "free-camera"]);
        assert!(search(&mods, "zzz").is_empty());
        assert_eq!(search(&mods, "").len(), mods.len());
    }

    #[test]
    fn pages_are_cut_and_clamped() {
        let p = Paging::of(57, 1);
        assert_eq!((p.start, p.end, p.pages, p.label()), (20, 40, 3, "21\u{2013}40 of 57".to_string()));
        assert!(p.has_previous() && p.has_next());
        let last = Paging::of(57, 9);
        assert_eq!((last.page, last.start, last.end), (2, 40, 57));
        assert!(!last.has_next());
        let one = Paging::of(3, 0);
        assert_eq!((one.pages, one.label()), (1, "3 of 3".to_string()));
        assert_eq!(Paging::of(0, 0).pages, 1);
    }
}
