//! How an app's name is shown. The catalog gives a `name` (the game) and a `project` (who made this version); the
//! user may also give a name of their own. Which of them shows, and where, is a setting.

/// Which of an app's names the library shows. The numbers are the ones Quiver stores in `settings.json`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NameStyle {
    NameOnly = 0,
    /// The name, with the project underneath. The default.
    #[default]
    NameAndProject = 1,
    ProjectOnly = 2,
    /// "Name (Project)" on one line.
    NameAndProjectInTitle = 3,
}

impl NameStyle {
    pub fn from_number(n: i64) -> Self {
        match n {
            0 => Self::NameOnly,
            2 => Self::ProjectOnly,
            3 => Self::NameAndProjectInTitle,
            _ => Self::NameAndProject,
        }
    }
}

fn blank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|t| !t.is_empty())
}

/// The title to show for an app. A name the user gave always wins; otherwise the style decides, falling back to
/// whichever of name and project exists.
pub fn resolve(name: &str, project: Option<&str>, custom: Option<&str>, style: NameStyle) -> String {
    if let Some(custom) = blank(custom) {
        return custom.to_string();
    }
    let (name, project) = (name.trim(), blank(project));
    match style {
        NameStyle::NameOnly | NameStyle::NameAndProject => {
            if name.is_empty() {
                project.unwrap_or("").to_string()
            } else {
                name.to_string()
            }
        }
        NameStyle::ProjectOnly => project.map(str::to_string).unwrap_or_else(|| name.to_string()),
        NameStyle::NameAndProjectInTitle => match project {
            Some(p) if !name.is_empty() && !p.eq_ignore_ascii_case(name) => format!("{name} ({p})"),
            Some(p) if name.is_empty() => p.to_string(),
            _ => name.to_string(),
        },
    }
}

/// The line under the title, when the style has one: the project, unless it only repeats the name or the user has
/// named the app themselves.
pub fn subtitle(name: &str, project: Option<&str>, custom: Option<&str>, style: NameStyle) -> Option<String> {
    if style != NameStyle::NameAndProject || blank(custom).is_some() {
        return None;
    }
    blank(project).filter(|p| !p.eq_ignore_ascii_case(name.trim())).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_the_user_gave_wins_over_every_style() {
        for style in [NameStyle::NameOnly, NameStyle::NameAndProject, NameStyle::ProjectOnly, NameStyle::NameAndProjectInTitle] {
            assert_eq!(resolve("Super Mario 64", Some("Ghostship"), Some("  Mario  "), style), "Mario");
        }
    }

    #[test]
    fn each_style_shows_its_part_and_falls_back_to_the_other() {
        let r = |name, project, style| resolve(name, project, None, style);
        assert_eq!(r("Super Mario 64", Some("Ghostship"), NameStyle::NameOnly), "Super Mario 64");
        assert_eq!(r("", Some("Ghostship"), NameStyle::NameOnly), "Ghostship");
        assert_eq!(r("Super Mario 64", Some("Ghostship"), NameStyle::ProjectOnly), "Ghostship");
        assert_eq!(r("Super Mario 64", None, NameStyle::ProjectOnly), "Super Mario 64");
        assert_eq!(r("Super Mario 64", Some("Ghostship"), NameStyle::NameAndProjectInTitle), "Super Mario 64 (Ghostship)");
        assert_eq!(r("Doom", Some("DOOM"), NameStyle::NameAndProjectInTitle), "Doom", "a project that repeats the name is not added");
    }

    #[test]
    fn the_subtitle_is_the_project_unless_it_repeats_the_name_or_the_user_named_the_app() {
        let s = |name, project, custom| subtitle(name, project, custom, NameStyle::NameAndProject);
        assert_eq!(s("Super Mario 64", Some("Ghostship"), None).as_deref(), Some("Ghostship"));
        assert_eq!(s("Doom", Some("doom"), None), None);
        assert_eq!(s("Super Mario 64", Some("Ghostship"), Some("Mario")), None);
        assert_eq!(subtitle("Super Mario 64", Some("Ghostship"), None, NameStyle::NameOnly), None);
    }

    #[test]
    fn the_stored_numbers_map_to_styles_and_unknown_ones_to_the_default() {
        assert_eq!(NameStyle::from_number(3), NameStyle::NameAndProjectInTitle);
        assert_eq!(NameStyle::from_number(99), NameStyle::NameAndProject);
    }
}
