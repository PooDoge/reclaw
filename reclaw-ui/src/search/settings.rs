//! A settings schema narrowed to what a search finds, so the results are the real rows, working, under their section's title.
use super::text::matches_all;
use crate::settings::{Group, Row, RowKind, Schema, Section};

/// What a row can be found by: its label, its description, and the choices it offers ("Daylight" finds Theme).
fn row_fields(row: &Row) -> Vec<&str> {
    let mut fields = vec![row.label];
    fields.extend(row.description);
    if let RowKind::Choice { options, .. } = &row.kind {
        fields.extend(options.iter().copied());
    }
    fields
}

/// The sections, groups and rows `query` finds. A section or group whose own title matches keeps all its rows (searching
/// "network" shows the Network section); otherwise a row stays when it matches by itself, or when its label, description and
/// choices together with its group's and section's titles hold every word ("github token"). Empty sections and groups go.
/// An empty query keeps everything.
pub fn filter_schema(schema: &Schema, query: &str) -> Schema {
    let sections = schema
        .sections
        .iter()
        .filter_map(|section| {
            if matches_all(&[section.title], query) {
                return Some(section.clone());
            }
            let groups: Vec<Group> = section
                .groups
                .iter()
                .filter_map(|group| {
                    if group.heading.is_some_and(|h| matches_all(&[h], query)) {
                        return Some(group.clone());
                    }
                    let rows: Vec<Row> = group
                        .rows
                        .iter()
                        .filter(|row| {
                            let mut fields = row_fields(row);
                            fields.push(section.title);
                            fields.extend(group.heading);
                            matches_all(&fields, query)
                        })
                        .cloned()
                        .collect();
                    (!rows.is_empty()).then(|| Group { rows, ..group.clone() })
                })
                .collect();
            (!groups.is_empty()).then(|| Section { groups, ..section.clone() })
        })
        .collect();
    Schema { title: schema.title.clone(), sections }
}

/// How many rows a schema has.
pub fn row_count(schema: &Schema) -> usize {
    schema.sections.iter().map(|s| s.rows().count()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Group, Row, RowKind};

    fn schema() -> Schema {
        let toggle = |key, label| Row::new(key, label, RowKind::Toggle { default: false });
        Schema {
            title: "Settings".into(),
            sections: vec![
                Section {
                    id: "interface",
                    title: "Interface",
                    groups: vec![Group::new(vec![
                        Row::new("theme", "Theme", RowKind::Choice { options: &["Midnight", "Daylight"], default: 0 }),
                        toggle("sounds", "Sounds").described("Clicks and chimes"),
                    ])],
                },
                Section {
                    id: "network",
                    title: "Network",
                    groups: vec![
                        Group::new(vec![toggle("gh_save", "Save token")]).headed("GitHub"),
                        Group::new(vec![toggle("proxy", "Use a proxy")]),
                    ],
                },
            ],
        }
    }

    fn keys(schema: &Schema) -> Vec<&'static str> {
        schema.sections.iter().flat_map(|s| s.rows().map(|r| r.key)).collect()
    }

    #[test]
    fn rows_are_found_by_label_description_or_choice() {
        assert_eq!(keys(&filter_schema(&schema(), "theme")), ["theme"]);
        assert_eq!(keys(&filter_schema(&schema(), "chimes")), ["sounds"]);
        assert_eq!(keys(&filter_schema(&schema(), "daylight")), ["theme"]);
        assert!(filter_schema(&schema(), "nothing like this").sections.is_empty());
    }

    #[test]
    fn a_matching_section_or_group_keeps_its_rows() {
        assert_eq!(keys(&filter_schema(&schema(), "network")), ["gh_save", "proxy"]);
        assert_eq!(keys(&filter_schema(&schema(), "github")), ["gh_save"]);
        assert_eq!(keys(&filter_schema(&schema(), "github token")), ["gh_save"], "the group's title and the row's label together");
    }

    #[test]
    fn an_empty_search_keeps_everything() {
        let all = filter_schema(&schema(), "");
        assert_eq!(row_count(&all), 4);
        assert_eq!(all, schema());
    }
}
