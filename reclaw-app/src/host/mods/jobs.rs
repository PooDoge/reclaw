//! Installing, updating and removing one mod. An install is a job on its own thread, reported as activity like a game's install
//! (the board and the notices take it from there); a removal is quick and only reports when it fails.
use std::thread;

use reclaw_mods::{Document, ModError, Package, Provider, Stage, Step, Target, install_with_dependencies, uninstall};
use reclaw_net::Cancel;
use reclaw_ui::{
    activity::{ActivityEvent, Kind, Stage as UiStage},
    model::ModProvider,
    notices::Notice,
    store::AppAction,
};

use super::{Game, JobKey, Running, entries};
use crate::host::Host;

fn ui_stage(stage: Stage) -> UiStage {
    match stage {
        Stage::Downloading => UiStage::Downloading,
        Stage::Unpacking => UiStage::Extracting,
        Stage::Placing => UiStage::Finishing,
    }
}

/// The first line of an error, for the activity row.
fn short(error: &ModError) -> String {
    error.to_string().lines().next().unwrap_or_default().to_string()
}

impl Host {
    fn mod_game(&self, game_id: u32) -> Option<Game> {
        let found = self.moddable().into_iter().find(|g| g.id == game_id);
        if found.is_none() {
            tracing::warn!(game = game_id, "a mod was asked for a game that is not installed or takes no mods");
            self.tell(Notice::problem(
                "That game cannot take mods now",
                "It is not installed, or its folder is missing, or the catalog lists no mods for it",
                vec![],
            ));
        }
        found
    }

    /// What to install for a mod: the listed package, or, for an installed mod the site no longer lists, one made from its record.
    fn package_for(&self, game: &Game, provider: Provider, id: &str) -> Option<Package> {
        let listed = self
            .inner
            .mods
            .listings()
            .get(&game.key)
            .and_then(|l| l.iter().find(|p| p.provider == provider && p.id.eq_ignore_ascii_case(id)).cloned());
        listed.or_else(|| {
            let document = Document::load(&game.folder).ok()?;
            document.find(provider, id).next().map(|r| entries::package_from_record(r, provider))
        })
    }

    /// Install a mod, or update it when it is installed, with whatever it needs first.
    pub(crate) fn install_mod(&self, game_id: u32, provider: ModProvider, id: &str) {
        let mods = &self.inner.mods;
        let (Some(sites), Some(installer)) = (mods.sites.clone(), mods.installer.clone()) else {
            self.tell(Notice::problem("No network", "The network layer could not start, so nothing can be downloaded", vec![]));
            self.publish_mods();
            return;
        };
        let Some(game) = self.mod_game(game_id) else {
            self.publish_mods();
            return;
        };
        let provider = entries::provider_of(provider);
        let Some(package) = self.package_for(&game, provider, id) else {
            tracing::warn!(game = %game.title, provider = provider.id(), mod_id = id, "a mod was asked for that is not listed");
            self.tell(Notice::problem(
                "That mod is not listed",
                &format!("{} does not list it for {} now", provider.label(), game.title),
                vec!["Refresh the Mods page and try again.".to_string()],
            ));
            self.publish_mods();
            return;
        };
        let key: JobKey = (game.key.clone(), provider, package.id.to_ascii_lowercase());
        let (activity, cancel) = (self.inner.installs.next_activity(), Cancel::new());
        {
            let mut jobs = mods.jobs();
            if jobs.contains_key(&key) {
                tracing::debug!(mod_id = %package.id, "this mod already has a job running");
                return;
            }
            jobs.insert(key.clone(), Running { activity, cancel: cancel.clone() });
        }
        let title = entries::from_package(game.id, &package, None, false).title;
        self.send(AppAction::Activity(ActivityEvent::Started {
            id: activity,
            game_id: game.id,
            kind: Kind::Mod { provider: entries::ui_provider(provider), id: package.id.clone() },
            title,
            bytes_total: package.size,
        }));
        self.publish_mods();

        let host = self.clone();
        let work = move || {
            // A recomp reads its mods folder only when `portable.txt` is beside it; an install made before Reclaw wrote it lacks it.
            host.add_markers(&game.title, &game.folder, &game.markers, true);
            let target = Target { folder: game.folder.clone(), config: game.config.clone() };
            let sink = |step: Step| {
                host.send(AppAction::Activity(ActivityEvent::Progress {
                    id: activity,
                    stage: ui_stage(step.stage),
                    bytes_done: step.done,
                    bytes_total: step.total,
                    rate: step.rate,
                }));
            };
            let result = install_with_dependencies(&sites, &installer, &target, &package, &cancel, &mut |step| sink(step));
            host.inner.mods.jobs().remove(&key);
            match result {
                Ok(records) => {
                    let names: Vec<&str> = records.iter().map(|r| r.title()).collect();
                    tracing::info!(game = %game.title, mod_id = %package.id, installed = ?names, "mod installed");
                    host.send(AppAction::Activity(ActivityEvent::Finished { id: activity, changelog: None }));
                }
                Err(error) if error.is_cancelled() => host.send(AppAction::Activity(ActivityEvent::Cancelled { id: activity })),
                Err(error) => {
                    tracing::warn!(game = %game.title, mod_id = %package.id, %error, "the mod install failed");
                    let mut details = vec![error.to_string()];
                    details.extend(error.hint());
                    host.send(AppAction::Activity(ActivityEvent::Failed { id: activity, reason: short(&error), details }));
                }
            }
            host.publish_mods();
        };
        if let Err(error) = thread::Builder::new().name("reclaw-mod-install".into()).spawn(work) {
            self.inner.mods.jobs().retain(|_, job| job.activity != activity);
            self.send(AppAction::Activity(ActivityEvent::Failed {
                id: activity,
                reason: "A background thread did not start".to_string(),
                details: vec![error.to_string()],
            }));
            self.publish_mods();
        }
    }

    /// Remove a mod's files and its record. Files another mod also lists stay (they were refused at install, so there are none).
    pub(crate) fn remove_mod(&self, game_id: u32, provider: ModProvider, id: &str) {
        let Some(game) = self.mod_game(game_id) else { return };
        let provider = entries::provider_of(provider);
        if self.inner.mods.is_busy(&game.key, provider, id) {
            tracing::debug!(mod_id = id, "a mod with a job running was not removed");
            return;
        }
        let (host, id) = (self.clone(), id.to_string());
        let work = move || {
            let target = Target { folder: game.folder.clone(), config: game.config.clone() };
            match uninstall(&target, provider, &id) {
                Ok(true) => tracing::info!(game = %game.title, mod_id = %id, "mod removed"),
                Ok(false) => tracing::debug!(game = %game.title, mod_id = %id, "a mod to remove was not recorded"),
                Err(error) => {
                    tracing::warn!(game = %game.title, mod_id = %id, %error, "the mod could not be removed");
                    let mut details = vec![error.to_string()];
                    details.extend(error.hint());
                    host.tell(Notice::problem("A mod could not be removed", &format!("From {}", game.title), details));
                }
            }
            host.publish_mods();
        };
        if let Err(error) = thread::Builder::new().name("reclaw-mod-remove".into()).spawn(work) {
            self.tell(Notice::problem("The mod could not be removed", "A background thread did not start", vec![error.to_string()]));
        }
    }
}
