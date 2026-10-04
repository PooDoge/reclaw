use freya::prelude::*;

use super::ctx::Ctx;
use crate::readme::ReadmeSection;

/// The project's README, when the catalog knows where the project lives.
pub(super) fn view(c: &Ctx) -> Option<Element> {
    let project = c.view.project.as_ref()?;
    Some(ReadmeSection::new(project.repo.clone(), c.open_url()).into_element())
}
