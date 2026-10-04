use url::Url;

/// Where a README came from, so its relative links and images can be resolved the way the site
/// that hosts it resolves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadmeContext {
    /// Folder holding the README's raw files; relative images resolve here.
    raw_base: Url,
    /// The same folder on the site's web pages; relative links resolve here.
    page_base: Url,
    readme: Url,
}

/// Where a link in a README goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An address to open in the browser.
    Web(String),
    /// A heading in the same README (`#install`).
    Anchor(String),
    /// Nothing safe to do: `javascript:`, `mailto:`, a path out of the repository.
    None,
}

/// Owner and repository names as the hosts allow them. Checked because they become part of an address.
fn name_ok(part: &str, allow_slash: bool) -> bool {
    !part.is_empty()
        && !part.starts_with('.')
        && !part.split('/').any(|seg| seg.is_empty() || seg == "." || seg == "..")
        && part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') || (allow_slash && c == '/'))
}

impl ReadmeContext {
    /// A GitHub repository's README on `branch` (`HEAD` for the default one).
    pub fn github(owner: &str, name: &str, branch: &str) -> Option<Self> {
        if !name_ok(owner, false) || !name_ok(name, false) || !name_ok(branch, true) {
            return None;
        }
        Self::build(
            &format!("https://raw.githubusercontent.com/{owner}/{name}/{branch}/"),
            &format!("https://github.com/{owner}/{name}/blob/{branch}/"),
            "README.md",
        )
    }

    /// A GitLab project's README; `owner` may be a nested group (`group/subgroup`).
    pub fn gitlab(owner: &str, name: &str, branch: &str) -> Option<Self> {
        if !name_ok(owner, true) || !name_ok(name, false) || !name_ok(branch, true) {
            return None;
        }
        Self::build(
            &format!("https://gitlab.com/{owner}/{name}/-/raw/{branch}/"),
            &format!("https://gitlab.com/{owner}/{name}/-/blob/{branch}/"),
            "README.md",
        )
    }

    fn build(raw: &str, page: &str, file: &str) -> Option<Self> {
        let raw_base = Url::parse(raw).ok()?;
        let readme = raw_base.join(file).ok()?;
        Some(Self { raw_base, page_base: Url::parse(page).ok()?, readme })
    }

    /// The address of the README itself.
    pub fn readme_url(&self) -> &Url {
        &self.readme
    }

    /// An image's address as something to fetch: absolute, https, and inside the repository when it
    /// was relative. `None` for anything else.
    pub fn image(&self, src: &str) -> Option<String> {
        resolve(src, &self.raw_base).map(String::from)
    }

    /// Where a link goes.
    pub fn link(&self, href: &str) -> LinkTarget {
        let href = href.trim();
        if let Some(anchor) = href.strip_prefix('#') {
            return if anchor.is_empty() { LinkTarget::None } else { LinkTarget::Anchor(anchor.to_string()) };
        }
        resolve(href, &self.page_base).map_or(LinkTarget::None, |u| LinkTarget::Web(u.into()))
    }
}

/// Resolve `text` against `base` (a folder address ending in `/`), keeping relative paths inside it.
fn resolve(text: &str, base: &Url) -> Option<Url> {
    let text = text.trim();
    if text.is_empty() || text.len() > 2048 {
        return None;
    }
    // `//host/path` follows the page's scheme, which is https.
    let text = if text.starts_with("//") { format!("https:{text}") } else { text.to_string() };
    match Url::parse(&text) {
        Ok(mut absolute) => match absolute.scheme() {
            "https" => Some(absolute),
            // Old READMEs link http images that the site serves over https as well.
            "http" => absolute.set_scheme("https").ok().map(|()| absolute),
            _ => None,
        },
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            // A path with a leading slash is from the repository's root, not the site's.
            let relative = text.trim_start_matches('/');
            let joined = base.join(relative).ok()?;
            // `../../other/repo` must not climb out of this repository.
            (joined.host() == base.host() && joined.path().starts_with(base.path())).then_some(joined)
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests;
