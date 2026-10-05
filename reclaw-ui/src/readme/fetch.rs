use std::rc::Rc;

use freya::prelude::*;
use reclaw_games::project::{RepoHost, RepoRef};
use reclaw_media::{
    Want,
    readme::{Document, ReadmeContext},
};

use crate::media::{RemoteFile, use_remote_file};

/// Where a repository's README lives, or `None` when its owner or name is not one an address can be made from.
pub fn readme_context(repo: &RepoRef) -> Option<ReadmeContext> {
    match repo.host {
        RepoHost::Github => ReadmeContext::github(&repo.owner, &repo.name, "HEAD"),
        RepoHost::Gitlab => ReadmeContext::gitlab(&repo.owner, &repo.name, "HEAD"),
    }
}

/// A project's README as a page sees it: where the fetch is, and the cut-up text once it has arrived.
#[derive(Clone, PartialEq)]
pub struct Readme {
    pub context: Option<ReadmeContext>,
    pub file: RemoteFile,
    pub document: Option<Rc<Document>>,
}

/// The README of `repo`, fetched in the background and cut into blocks once. A hook: the component
/// must be keyed by the repository. The game page's README section and its banner both call this;
/// the media hub fetches each address once, so the second caller finds the file already on disk.
pub fn use_readme(repo: Option<&RepoRef>) -> Readme {
    let context = repo.and_then(readme_context);
    let file = use_remote_file(context.as_ref().map(|c| c.readme_url().as_str()), Want::Text);
    let mut document = use_state(|| None::<Rc<Document>>);

    // Parse once when the file arrives. Reading and cutting a README takes a few milliseconds.
    let path = match &file {
        RemoteFile::Ready(cached) => Some(cached.path.clone()),
        _ => None,
    };
    use_side_effect_with_deps(&(path, context.clone()), move |(path, context)| {
        let parsed = path.as_ref().zip(context.as_ref()).and_then(|(path, context)| {
            let text = std::fs::read_to_string(path).ok()?;
            Some(Rc::new(reclaw_media::readme::parse(&text, context)))
        });
        document.set(parsed);
    });

    let document = document.read().clone();
    Readme { context, file, document }
}
