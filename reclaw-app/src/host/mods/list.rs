//! Asking the sites what mods each game has, on a background thread. A listing that fails keeps the one before it; the person is
//! told only when they asked (Refresh on the Mods page), since a launch with no network would otherwise open on a pile of notices.
use std::{sync::atomic::Ordering, thread};

use reclaw_mods::{Package, Sort, Source};
use reclaw_ui::notices::Notice;

use super::Game;
use crate::host::Host;

/// How many mods of each site are listed per game: the most downloaded first, which is what a person browses.
const PER_SOURCE: usize = 60;

impl Host {
    /// List the mods of every game that takes them, in the background, then show them. `asked`: the person pressed Refresh, so
    /// the sites are asked again even when a recent answer is saved, and a failure is shown. A second request while one runs does
    /// nothing.
    pub(crate) fn refresh_mods(&self, asked: bool) {
        let Some(sites) = self.inner.mods.sites.clone() else {
            self.publish_mods();
            if asked {
                self.tell(Notice::problem("No network", "The network layer could not start, so mods cannot be listed", vec![]));
            }
            return;
        };
        if self.inner.mods.refreshing.swap(true, Ordering::SeqCst) {
            return;
        }
        let host = self.clone();
        let started = thread::Builder::new().name("reclaw-mods".into()).spawn(move || {
            host.run_mods_refresh(&sites, asked);
            host.inner.mods.refreshing.store(false, Ordering::SeqCst);
        });
        if let Err(error) = started {
            self.inner.mods.refreshing.store(false, Ordering::SeqCst);
            tracing::warn!(%error, "the mods refresh thread did not start");
            if asked {
                self.tell(Notice::problem("Mods could not be listed", "A background thread did not start", vec![error.to_string()]));
            }
        }
    }

    /// The body of a refresh: runs on its own thread, and directly in tests.
    pub(crate) fn run_mods_refresh(&self, sites: &reclaw_mods::ModSites, asked: bool) {
        // Show what is installed straight away; the listings follow.
        self.publish_mods();
        let mut problems = Vec::new();
        for game in self.moddable() {
            let (listing, failed) = list_game(sites, &game, asked);
            // A game none of whose sites answered keeps what was listed before.
            if !listing.is_empty() || failed.is_empty() {
                self.inner.mods.listings().insert(game.key.clone(), listing);
            }
            problems.extend(failed);
        }
        self.publish_mods();
        if asked && !problems.is_empty() {
            self.tell(Notice::problem(
                "Some mods could not be listed",
                "Showing what was listed before, where there is anything",
                problems,
            ));
        }
    }
}

/// Every source's packages for one game, and a line for each source that failed.
fn list_game(sites: &reclaw_mods::ModSites, game: &Game, fresh: bool) -> (Vec<Package>, Vec<String>) {
    let mut listing = Vec::new();
    let mut problems = Vec::new();
    for source in Source::all_of(&game.config) {
        match sites.list_up_to(&source, PER_SOURCE, Sort::MostDownloaded, fresh) {
            Ok(packages) => {
                tracing::debug!(game = %game.title, provider = source.provider.id(), key = %source.key, count = packages.len(), "listed mods");
                listing.extend(packages);
            }
            Err(error) => {
                tracing::warn!(game = %game.title, provider = source.provider.id(), key = %source.key, %error, "mods could not be listed");
                let mut line = format!("{} on {}: {error}", game.title, source.provider.label());
                if let Some(hint) = error.hint() {
                    line.push_str(&format!(" ({hint})"));
                }
                problems.push(line);
            }
        }
    }
    (listing, problems)
}
