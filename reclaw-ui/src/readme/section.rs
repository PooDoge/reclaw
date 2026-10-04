use std::rc::Rc;

use freya::prelude::*;
use reclaw_games::project::{RepoHost, RepoRef};
use reclaw_media::{
    Want,
    readme::{Document, ReadmeContext},
};

use super::view::ReadmeView;
use crate::{
    desktop::pages::common::heading,
    media::{RemoteFile, use_remote_file},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

/// About how much reading shows before "Show the whole README", in characters.
const PREVIEW_CHARS: usize = 1800;

/// The README of a project, fetched from its repository and shown on the game page. Collapsed to its
/// first blocks until asked to show the rest. Nothing at all when the repository's address is not
/// one the README can be fetched from, or when downloads are switched off.
#[derive(Clone, PartialEq)]
pub struct ReadmeSection {
    repo: RepoRef,
    on_open: EventHandler<String>,
    key: DiffKey,
}

impl KeyExt for ReadmeSection {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl ReadmeSection {
    pub fn new(repo: RepoRef, on_open: EventHandler<String>) -> Self {
        Self { repo, on_open, key: DiffKey::None }
    }

    fn context(&self) -> Option<ReadmeContext> {
        match self.repo.host {
            RepoHost::Github => ReadmeContext::github(&self.repo.owner, &self.repo.name, "HEAD"),
            RepoHost::Gitlab => ReadmeContext::gitlab(&self.repo.owner, &self.repo.name, "HEAD"),
        }
    }
}

impl Component for ReadmeSection {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let context = self.context();
        let file = use_remote_file(context.as_ref().map(|c| c.readme_url().as_str()), Want::Text);
        let mut doc = use_state(|| None::<Rc<Document>>);
        let mut expanded = use_state(|| false);

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
            doc.set(parsed);
        });

        let Some(context) = context else { return rect().into_element() };
        let page = self.repo.url();
        let on_open = self.on_open.clone();
        let open_page = {
            let (on_open, page) = (on_open.clone(), page.clone());
            move |_| on_open.call(page.clone())
        };

        let body: Element = match (&file, doc.read().clone()) {
            (RemoteFile::Off, _) => return rect().into_element(),
            (_, Some(parsed)) if parsed.is_empty() => TypeStyle::Meta.text("This project's README is empty.", t.ink_subtle).into_element(),
            (_, Some(parsed)) => {
                let start = parsed.preview_len(PREVIEW_CHARS);
                let more = start < parsed.blocks.len();
                let view = ReadmeView::new((*parsed).clone(), on_open);
                let view = if expanded() || !more { view } else { view.first(start) };
                rect()
                    .vertical()
                    .spacing(SPACE_3)
                    .width(Size::fill())
                    .child(view)
                    .maybe(more, |el| {
                        el.child(
                            ActionButton::new(ButtonVariant::Ghost)
                                .label(if expanded() { "Show less" } else { "Show the whole README" })
                                .size(ButtonSize::Md)
                                .on_press(move |_| expanded.set(!expanded())),
                        )
                    })
                    .into_element()
            }
            (RemoteFile::Failed(_), None) => rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(SPACE_3)
                .child(TypeStyle::Meta.text("The README could not be loaded.", t.ink_subtle))
                .child(ActionButton::new(ButtonVariant::Ghost).label("Open the repository").size(ButtonSize::Md).on_press(open_page))
                .into_element(),
            (_, None) => TypeStyle::Meta.text("Loading the README...", t.ink_subtle).into_element(),
        };
        let _ = context;
        rect().vertical().spacing(SPACE_2).width(Size::fill()).child(heading(&t, "README")).child(body).into_element()
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(DiffKey::from(&self.repo.url()))
    }
}
