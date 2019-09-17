use crossterm::{Attribute, ClearType, Color::*, KeyEvent, Terminal};
use termimad::{
    ansi, gray, terminal_size, Alignment, Area, CompoundStyle, Event, InputField, ListView,
    ListViewCell, ListViewColumn, MadSkin,
};

use crate::deps::{
    latest_trusted_version_string, AccumulativeCrateDetails, CrateDetails, CrateStats, Progress,
};
use crate::opts::CargoOpts;
use crate::prelude::*;
use crate::repo::Repo;
use crev_lib::VerificationStatus;
use lazy_static::lazy_static;

/// the styles that can be applied to cells of the dep list
struct DepTableSkin {
    std: CompoundStyle,
    bad: CompoundStyle,
    medium: CompoundStyle,
    good: CompoundStyle,
    none: CompoundStyle,
}

impl Default for DepTableSkin {
    fn default() -> Self {
        Self {
            std: CompoundStyle::default(),
            bad: CompoundStyle::with_fgbg(White, Red),
            medium: CompoundStyle::with_fg(Yellow),
            good: CompoundStyle::with_fg(Green),
            none: CompoundStyle::with_fg(gray(10)),
        }
    }
}

fn list_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    skin.table.align = Alignment::Center;
    skin.bold.set_fg(gray(22));
    skin.italic.set_fg(ansi(153));
    skin.headers[0].compound_style = CompoundStyle::with_attr(Attribute::Bold);
    skin
}

/// The whole screen
pub struct VerifyScreen<'t> {
    pub title: String,
    title_area: Area,
    title_skin: MadSkin,
    status_area: Area,
    status_skin: MadSkin,
    input_field: InputField,
    hint_area: Area,
    list_view: ListView<'t, CrateStats>,
    skin: &'t MadSkin,
    last_dimensions: (u16, u16),
    progress: crate::deps::Progress,
}

const SIZE_NAMES: &[&str] = &["", "K", "M", "G", "T", "P", "E", "Z", "Y"];
/// format a number of as a string
pub fn u64_to_str(mut v: u64) -> String {
    if v == 0 {
        return "".to_owned();
    }
    let mut i = 0;
    while v >= 1200 && i < SIZE_NAMES.len() - 1 {
        v >>= 10;
        i += 1;
    }
    format!("{}{}", v, &SIZE_NAMES[i])
}

impl<'t> VerifyScreen<'t> {
    pub fn new(total_crate_count: usize, cargo_opts: CargoOpts) -> Result<Self> {
        lazy_static! {
            static ref TS: DepTableSkin = DepTableSkin::default();
            static ref SKIN: MadSkin = list_skin();
        }

        let mut status_skin = MadSkin::default();
        status_skin.paragraph.set_bg(gray(4));
        status_skin.italic.set_fg(ansi(225));

        let mut title_skin = MadSkin::default();
        title_skin.headers[0].compound_style =
            CompoundStyle::new(Some(gray(22)), None, vec![Attribute::Bold]);

        let columns = vec![
            ListViewColumn::new(
                "crate",
                10,
                80,
                Box::new(|dep: &CrateStats| {
                    ListViewCell::new(dep.info.id.name().to_string(), &TS.std)
                }),
            )
            .with_align(Alignment::Left),
            ListViewColumn::new(
                "version",
                9,
                13,
                Box::new(|dep: &CrateStats| {
                    ListViewCell::new(dep.info.id.version().to_string(), &TS.std)
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "trust",
                6,
                6,
                Box::new(|dep: &CrateStats| {
                    if let Some(details) = dep.details() {
                        match details.accumulative.trust {
                            VerificationStatus::Verified => {
                                ListViewCell::new("pass".to_owned(), &TS.good)
                            }
                            VerificationStatus::Insufficient => {
                                ListViewCell::new("none".to_owned(), &TS.none)
                            }
                            VerificationStatus::Negative => {
                                ListViewCell::new("fail".to_owned(), &TS.bad)
                            }
                        }
                    } else {
                        ListViewCell::new("?".to_string(), &TS.medium)
                    }
                }),
            ),
            ListViewColumn::new(
                "last trusted",
                12,
                16,
                Box::new(|dep: &CrateStats| {
                    ListViewCell::new(
                        dep.details().map_or("?".to_owned(), |cdep| {
                            latest_trusted_version_string(
                                &dep.info.id.version(),
                                &cdep.latest_trusted_version,
                            )
                        }),
                        &TS.std,
                    )
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "reviews",
                3,
                3,
                Box::new(|dep: &CrateStats| {
                    ListViewCell::new(
                        dep.details()
                            .map_or("?".to_owned(), |cdep| u64_to_str(cdep.reviews.version)),
                        &TS.std,
                    )
                }),
            )
            .with_align(Alignment::Center),
            ListViewColumn::new(
                "reviews",
                3,
                3,
                Box::new(|dep: &CrateStats| {
                    ListViewCell::new(
                        dep.details()
                            .map_or("?".to_owned(), |cdep| u64_to_str(cdep.reviews.total)),
                        &TS.std,
                    )
                }),
            )
            .with_align(Alignment::Center),
            ListViewColumn::new(
                "downloads",
                6,
                6,
                Box::new(|dep: &CrateStats| {
                    if let Some(CrateDetails {
                        downloads: Some(downloads),
                        ..
                    }) = dep.details()
                    {
                        ListViewCell::new(
                            u64_to_str(downloads.version),
                            if downloads.version < 1000 {
                                &TS.medium
                            } else {
                                &TS.std
                            },
                        )
                    } else {
                        ListViewCell::new("".to_string(), &TS.std)
                    }
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "downloads",
                6,
                6,
                Box::new(|dep: &CrateStats| {
                    if let Some(CrateDetails {
                        downloads: Some(downloads),
                        ..
                    }) = dep.details()
                    {
                        ListViewCell::new(
                            u64_to_str(downloads.total),
                            if downloads.total < 1000 {
                                &TS.medium
                            } else {
                                &TS.std
                            },
                        )
                    } else {
                        ListViewCell::new("".to_string(), &TS.std)
                    }
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "owners",
                2,
                2,
                Box::new(|dep: &CrateStats| match dep.details() {
                    Some(CrateDetails {
                        owners: Some(owners),
                        ..
                    }) if owners.trusted > 0 => {
                        ListViewCell::new(format!("{}", owners.trusted), &TS.good)
                    }
                    _ => ListViewCell::new("".to_owned(), &TS.std),
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "owners",
                3,
                3,
                Box::new(|dep: &CrateStats| {
                    ListViewCell::new(
                        match dep.details() {
                            Some(CrateDetails {
                                owners: Some(owners),
                                ..
                            }) if owners.total > 0 => format!("{}", owners.total),
                            _ => "".to_owned(),
                        },
                        &TS.std,
                    )
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "issues",
                2,
                2,
                Box::new(|dep: &CrateStats| match dep.details() {
                    Some(CrateDetails { accumulative, .. }) if accumulative.issues.trusted > 0 => {
                        ListViewCell::new(format!("{}", accumulative.issues.trusted), &TS.bad)
                    }
                    _ => ListViewCell::new("".to_owned(), &TS.std),
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "issues",
                3,
                3,
                Box::new(|dep: &CrateStats| match dep.details() {
                    Some(CrateDetails { accumulative, .. }) if accumulative.issues.total > 0 => {
                        ListViewCell::new(format!("{}", accumulative.issues.total), &TS.medium)
                    }
                    _ => ListViewCell::new("".to_owned(), &TS.std),
                }),
            )
            .with_align(Alignment::Right),
            ListViewColumn::new(
                "l.o.c.",
                6,
                6,
                Box::new(|dep: &CrateStats| {
                    ListViewCell::new(
                        match dep.details() {
                            Some(CrateDetails {
                                accumulative: AccumulativeCrateDetails { loc: Some(loc), .. },
                                ..
                            }) => u64_to_str(*loc as u64),
                            _ => "".to_string(),
                        },
                        &TS.std,
                    )
                }),
            )
            .with_align(Alignment::Right),
        ];

        let list_view = ListView::new(Area::new(0, 1, 10, 1), columns, &SKIN);

        let repo = Repo::auto_open_cwd(cargo_opts)?; // TODO not extra clean
        let title = repo.name().to_string();
        let mut screen = Self {
            title,
            title_area: Area::new(0, 0, 10, 1),
            title_skin,
            status_area: Area::new(0, 2, 10, 1),
            input_field: InputField::new(Area::new(0, 3, 10, 1)),
            hint_area: Area::new(0, 3, 10, 1),
            list_view,
            skin: &SKIN,
            status_skin,
            last_dimensions: (0, 0),
            progress: Progress {
                done: 0,
                total: total_crate_count,
            },
        };
        screen.resize();
        Ok(screen)
    }
    pub fn set_computation_status(&mut self, done: usize) {
        self.progress.done = done;
        assert!(self.progress.is_valid());
    }
    pub fn add_dep(&mut self, dep: CrateStats) {
        self.list_view.add_row(dep);
    }

    pub fn resize(&mut self) {
        let (w, h) = terminal_size();
        // XXX: TODO: https://github.com/Canop/termimad/issues/6
        let (w, h) = (w - 1, h - 1);
        if (w, h) == self.last_dimensions {
            return;
        }
        self.last_dimensions = (w, h);
        self.title_area.top = 0;
        self.title_area.width = w;
        self.list_view.area.top = 1;
        self.list_view.area.width = w;
        self.list_view.area.height = h - 4;
        self.list_view.update_dimensions();
        self.status_area.top = h - 3;
        self.status_area.height = 3;
        self.status_area.width = w;
        self.input_field.change_area(0, h - 2, w / 2);
        self.hint_area.top = h - 2;
        self.hint_area.left = self.input_field.area.width;
        self.hint_area.width = w - self.hint_area.left;

        Terminal::new().clear(ClearType::All).unwrap();
    }

    fn update_title(&self) {
        self.title_skin
            .write_in_area(&format!("# *crev* : {}", &self.title), &self.title_area)
            .unwrap();
    }

    fn update_list_view(&mut self) {
        self.list_view.display().unwrap();
    }

    fn all_deps_ready(&self) -> bool {
        self.progress.is_complete()
    }

    fn update_status(&self) {
        let mut status = if self.all_deps_ready() {
            "Computation finished".to_owned()
        } else {
            format!(
                "Computing: *{}* / {}",
                self.progress.done, self.progress.total
            )
        };
        let (displayed, total) = self.list_view.row_counts();
        if displayed < total {
            status.push_str(&format!(
                " - **Filtered list** displays *{}* / *{}*. Hit `<esc>` to show all",
                displayed, total
            ));
        }
        self.status_skin
            .write_in_area(&status, &self.status_area)
            .unwrap();
    }
    fn update_input(&self) {
        self.input_field.display();
    }
    fn update_hint(&self) {
        self.skin
            .write_in_area(
                if self.all_deps_ready() {
                    "Hit *ctrl-q* to quit, *PageUp* or *PageDown* to scroll"
                } else {
                    "Hit *ctrl-q* to quit"
                },
                &self.hint_area,
            )
            .unwrap();
    }
    pub fn update(&mut self) {
        self.resize();
        self.update_title();
        self.update_list_view();
        self.update_status();
        self.update_input();
        self.update_hint();
    }
    #[allow(dead_code)]
    pub fn try_scroll_lines(&mut self, lines_count: i32) {
        self.list_view.try_scroll_lines(lines_count);
    }
    /// set the scroll amount.
    /// pages_count can be negative
    pub fn try_scroll_pages(&mut self, pages_count: i32) {
        self.list_view.try_scroll_pages(pages_count);
    }
    /// handle a user event
    pub fn apply_event(&mut self, user_event: &Event) {
        match user_event {
            Event::Key(KeyEvent::PageUp) => {
                self.try_scroll_pages(-1);
            }
            Event::Key(KeyEvent::PageDown) => {
                self.try_scroll_pages(1);
            }
            Event::Wheel(lines_count) => {
                self.try_scroll_lines(*lines_count);
            }
            Event::Key(KeyEvent::Esc) => {
                self.input_field.set_content("");
                self.list_view.remove_filter();
            }
            _ => {
                if self.input_field.apply_event(user_event) {
                    let pattern = self.input_field.get_content();
                    if !pattern.is_empty() {
                        self.list_view.set_filter(Box::new(move |dep: &CrateStats| {
                            dep.info.id.name().contains(&pattern)
                        }));
                    } else {
                        self.list_view.remove_filter();
                    }
                }
            }
        }
    }
}
