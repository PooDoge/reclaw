/// Which service hosts a project's releases.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum RepoSource {
    #[default]
    Github,
    Gitlab,
}

impl RepoSource {
    /// The source a stored value names: `github` or `gitlab` in any case; blank, missing or unknown is GitHub, as in
    /// the launcher this format comes from.
    pub fn normalize(text: Option<&str>) -> Self {
        match text.map(|t| t.trim().to_lowercase()).as_deref() {
            Some("gitlab") => Self::Gitlab,
            _ => Self::Github,
        }
    }

    /// The word the format and the identity keys use.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_is_gitlab_only_when_it_says_so() {
        for (text, expected) in
            [(Some("gitlab"), RepoSource::Gitlab), (Some(" GitLab "), RepoSource::Gitlab), (Some("github"), RepoSource::Github)]
        {
            assert_eq!(RepoSource::normalize(text), expected, "{text:?}");
        }
        for text in [None, Some(""), Some("  "), Some("bitbucket")] {
            assert_eq!(RepoSource::normalize(text), RepoSource::Github, "{text:?} falls back to GitHub");
        }
    }
}
