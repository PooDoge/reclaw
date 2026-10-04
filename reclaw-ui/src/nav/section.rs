/// The top-level destinations: the tabs on a desktop, the bumper-switched tabs in Deck mode.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Section {
    Library,
    Catalog,
    Downloads,
    Mods,
}

impl Section {
    pub const ALL: [Section; 4] = [Self::Library, Self::Catalog, Self::Downloads, Self::Mods];

    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Catalog => "Catalog",
            Self::Downloads => "Downloads",
            Self::Mods => "Mods",
        }
    }

    /// The next section, wrapping; `forward = false` goes the other way.
    pub fn step(self, forward: bool) -> Self {
        let i = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        let n = Self::ALL.len();
        Self::ALL[if forward { (i + 1) % n } else { (i + n - 1) % n }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_wraps_both_ways() {
        assert_eq!(Section::Mods.step(true), Section::Library);
        assert_eq!(Section::Library.step(false), Section::Mods);
        assert_eq!(Section::Library.step(true), Section::Catalog);
    }
}
