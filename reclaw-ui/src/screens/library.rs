use freya::prelude::*;

use crate::{metrics::*, prelude::*, typography::TypeStyle};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Filter {
    All,
    Installed,
    Updates,
}

impl Filter {
    fn matches(self, game: &GameEntry) -> bool {
        match self {
            Self::All => true,
            Self::Installed => game.status.is_installed(),
            Self::Updates => game.status == AppStatus::UpdateReady,
        }
    }
}

/// State behind the install dialog, created once per screen.
#[derive(Clone, Copy)]
struct InstallForm {
    location: State<String>,
    game_file: State<Option<String>>,
    shortcut: State<bool>,
    prerelease: State<bool>,
}

impl InstallForm {
    fn use_new() -> Self {
        Self {
            location: use_state(|| String::from("~/Reclaw/Apps")),
            game_file: use_state(|| None::<String>),
            shortcut: use_state(|| true),
            prerelease: use_state(|| false),
        }
    }
}

/// The Library page at all three layout classes. Wide: top nav + sidebar + hero. Compact: icon
/// rail + hero + capsule grid. Phone: capsule grid, a pushed game page and bottom tabs.
#[derive(Clone, PartialEq)]
pub struct LibraryScreen {
    games: Vec<GameEntry>,
    downloads: Vec<Download>,
    class: LayoutClass,
    density: Density,
}

impl LibraryScreen {
    pub fn new(
        games: Vec<GameEntry>,
        downloads: Vec<Download>,
        class: LayoutClass,
        density: Density,
    ) -> Self {
        Self {
            games,
            downloads,
            class,
            density,
        }
    }
}

impl Component for LibraryScreen {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let mut selected = use_state(|| self.games.first().map(|g| g.id));
        let mut page_open = use_state(|| false);
        let mut filter = use_state(|| Filter::All);
        let search = use_state(String::new);
        let mut nav = use_state(|| NavItem::Library);
        let mut dialog_open = use_state(|| false);
        let form = InstallForm::use_new();

        let query = search.read().to_lowercase();
        let visible: Vec<GameEntry> = self
            .games
            .iter()
            .filter(|g| filter().matches(g))
            .filter(|g| {
                query.is_empty()
                    || g.title.to_lowercase().contains(&query)
                    || g.project.to_lowercase().contains(&query)
                    || g.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();
        let current = self
            .games
            .iter()
            .find(|g| Some(g.id) == selected())
            .cloned();

        let class = self.class;
        let density = self.density;
        let touch = density == Density::Touch;
        let installed = self
            .games
            .iter()
            .filter(|g| g.status.is_installed())
            .count() as u32;
        let updates = self
            .games
            .iter()
            .filter(|g| g.status == AppStatus::UpdateReady)
            .count() as u32;

        let chips = rect()
            .horizontal()
            .content(Content::wrap_spacing(SPACE_2))
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(
                FilterChip::new("All")
                    .count(self.games.len() as u32)
                    .selected(filter() == Filter::All)
                    .on_press(move |_| filter.set(Filter::All)),
            )
            .child(
                FilterChip::new("Installed")
                    .count(installed)
                    .selected(filter() == Filter::Installed)
                    .on_press(move |_| filter.set(Filter::Installed)),
            )
            .child(
                FilterChip::new("Updates")
                    .count(updates)
                    .selected(filter() == Filter::Updates)
                    .on_press(move |_| filter.set(Filter::Updates)),
            );

        let hero = |game: GameEntry, narrow: bool| {
            let needs_install = matches!(game.status, AppStatus::Available | AppStatus::NeedsFile);
            HeroHeader::new(game)
                .narrow(narrow)
                .density(density)
                .on_primary(move |_| {
                    if needs_install {
                        dialog_open.set(true);
                    }
                })
        };

        let downloads_list = rect()
            .vertical()
            .spacing(SPACE_2)
            .width(Size::fill())
            .child(TypeStyle::Eyebrow.text("Downloads", t.ink_subtle))
            .children(
                self.downloads
                    .iter()
                    .cloned()
                    .map(|d| DownloadItem::new(d).into_element()),
            );

        // Capsule grid in fixed-column rows of fluid capsules, so the last row keeps its column width.
        let grid = |columns: usize| {
            rect()
                .vertical()
                .spacing(SPACE_3)
                .width(Size::fill())
                .children(visible.chunks(columns).enumerate().map(|(row, chunk)| {
                    let mut line = rect()
                        .horizontal()
                        .content(Content::Flex)
                        .spacing(SPACE_3)
                        .width(Size::fill());
                    for game in chunk {
                        let id = game.id;
                        line = line.child(
                            rect().width(Size::flex(1.)).child(
                                GameCapsule::new(game.clone())
                                    .fluid(true)
                                    .selected(Some(id) == selected())
                                    .on_press(move |_| {
                                        selected.set(Some(id));
                                        page_open.set(true);
                                    })
                                    .key(id),
                            ),
                        );
                    }
                    for _ in chunk.len()..columns {
                        line = line.child(rect().width(Size::flex(1.)));
                    }
                    line.key(row).into_element()
                }))
        };

        let on_nav = move |item: NavItem| nav.set(item);
        let search_field = SearchField::new(search).density(density);
        // The 40px top bar always takes the pointer-height field, even at touch density.
        let topbar_search = SearchField::new(search);

        match class {
            LayoutClass::Wide => {
                let rows = visible.iter().cloned().map(|g| {
                    let id = g.id;
                    LibraryRow::new(g)
                        .selected(Some(id) == selected())
                        .density(density)
                        .on_press(move |_| selected.set(Some(id)))
                        .key(id)
                        .into_element()
                });
                let sidebar = rect()
                    .vertical()
                    .spacing(SPACE_2)
                    .width(Size::px(SIDEBAR_W))
                    .height(Size::fill())
                    .padding(SPACE_3)
                    .background(t.bg_base)
                    .border(Border::new().fill(t.line).width(BorderWidth {
                        right: 1.,
                        ..Default::default()
                    }))
                    .child(chips)
                    .child(
                        rect()
                            .padding(Gaps::new(SPACE_2, 0., 0., SPACE_1))
                            .child(TypeStyle::Eyebrow.text("Library", t.ink_subtle)),
                    )
                    .child(
                        ScrollView::new()
                            .show_scrollbar(false)
                            .height(Size::flex(1.))
                            .child(rect().vertical().spacing(2.).children(rows)),
                    );
                let main = rect()
                    .vertical()
                    .spacing(SPACE_5)
                    .width(Size::flex(1.))
                    .height(Size::fill())
                    .padding(SPACE_5)
                    .child(
                        ScrollView::new().show_scrollbar(false).child(
                            rect()
                                .vertical()
                                .spacing(SPACE_5)
                                .width(Size::fill())
                                .maybe_child(current.clone().map(|g| hero(g, false)))
                                .child(downloads_list),
                        ),
                    );
                let status = rect()
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .width(Size::fill())
                    .height(Size::px(28.))
                    .padding(Gaps::new(0., SPACE_4, 0., SPACE_4))
                    .background(t.bg_deep)
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .child(TypeStyle::Meta.text("Library synced", t.ink_subtle)),
                    )
                    .child(TypeStyle::Mono.text(
                        format!(
                            "{} apps  {} {}",
                            self.games.len(),
                            updates,
                            if updates == 1 { "update" } else { "updates" }
                        ),
                        t.ink_subtle,
                    ));

                rect()
                    .vertical()
                    .content(Content::Flex)
                    .expanded()
                    .background(t.bg_base)
                    .child(
                        Nav::new(NavMode::Top, nav())
                            .downloads(self.downloads.len() as u32)
                            .trailing(topbar_search)
                            .on_select(on_nav),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .content(Content::Flex)
                            .width(Size::fill())
                            .height(Size::flex(1.))
                            .child(sidebar)
                            .child(main),
                    )
                    .child(status)
                    .child(install_dialog(
                        dialog_open(),
                        &current,
                        form,
                        touch,
                        dialog_open,
                    ))
                    .into_element()
            }
            LayoutClass::Compact => rect()
                .horizontal()
                .content(Content::Flex)
                .expanded()
                .background(t.bg_base)
                .child(
                    Nav::new(NavMode::Rail, nav())
                        .downloads(self.downloads.len() as u32)
                        .on_select(on_nav),
                )
                .child(
                    rect()
                        .vertical()
                        .content(Content::Flex)
                        .width(Size::flex(1.))
                        .height(Size::fill())
                        .child(
                            rect()
                                .width(Size::fill())
                                .padding(Gaps::new(SPACE_3, SPACE_4, SPACE_3, SPACE_4))
                                .child(search_field),
                        )
                        .child(
                            ScrollView::new()
                                .show_scrollbar(false)
                                .height(Size::flex(1.))
                                .child(
                                    rect()
                                        .vertical()
                                        .spacing(SPACE_4)
                                        .width(Size::fill())
                                        .padding(SPACE_4)
                                        .child(chips)
                                        .maybe_child(current.clone().map(|g| hero(g, true)))
                                        .child(grid(4)),
                                ),
                        ),
                )
                .child(install_dialog(
                    dialog_open(),
                    &current,
                    form,
                    touch,
                    dialog_open,
                ))
                .into_element(),
            LayoutClass::Phone => {
                let body = if page_open() {
                    rect()
                        .vertical()
                        .spacing(SPACE_4)
                        .width(Size::fill())
                        .padding(SPACE_4)
                        .child(
                            ActionButton::new(ButtonVariant::Ghost)
                                .icon(IconName::Chevron)
                                .label("Library")
                                .size(ButtonSize::Touch)
                                .on_press(move |_| page_open.set(false)),
                        )
                        .maybe_child(current.clone().map(|g| hero(g, true)))
                        .child(downloads_list)
                        .into_element()
                } else {
                    rect()
                        .vertical()
                        .spacing(SPACE_4)
                        .width(Size::fill())
                        .padding(SPACE_4)
                        .child(TypeStyle::Eyebrow.text("Library", t.ink_muted))
                        .child(search_field)
                        .child(chips)
                        .child(grid(2))
                        .into_element()
                };
                rect()
                    .vertical()
                    .content(Content::Flex)
                    .expanded()
                    .background(t.bg_base)
                    .child(
                        ScrollView::new()
                            .show_scrollbar(false)
                            .height(Size::flex(1.))
                            .child(body),
                    )
                    .child(
                        Nav::new(NavMode::Bottom, nav())
                            .downloads(self.downloads.len() as u32)
                            .on_select(on_nav),
                    )
                    .child(install_dialog(
                        dialog_open(),
                        &current,
                        form,
                        touch,
                        dialog_open,
                    ))
                    .into_element()
            }
        }
    }
}

fn install_dialog(
    open: bool,
    current: &Option<GameEntry>,
    form: InstallForm,
    touch: bool,
    mut dialog_open: State<bool>,
) -> InstallDialog {
    let title = current
        .as_ref()
        .map(|g| g.title.to_string())
        .unwrap_or_default();
    let mut game_file = form.game_file;
    InstallDialog::new(
        open,
        &title,
        form.location,
        form.game_file,
        form.shortcut,
        form.prerelease,
    )
    .touch(touch)
    // The real app opens a native file picker here; the gallery fills in a sample path.
    .on_choose_file(move |_| game_file.set(Some("~/Games/starfall64.z64".to_string())))
    .on_cancel(move |_| dialog_open.set(false))
    .on_confirm(move |_| dialog_open.set(false))
}
