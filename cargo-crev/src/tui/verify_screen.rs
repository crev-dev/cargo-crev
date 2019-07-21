use crossterm::{
    Color::*,
};
use termimad::*;

use crev_lib::VerificationStatus;
use crate::dep::{
    Dep, ComputedDep, DepTable,
    latest_trusted_version_string,
};
use crate::prelude::*;
use crate::repo::Repo;
use crate::tui::table_view::*;

/*
 Areas:
 - repo name
 - dep table
 - computation status / warnings
 - user action hint ("Hit ctrl-q to quit")
 - input
*/

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

pub struct VerifyScreen<'t> {
    pub title: String,
    title_area: Area,
    status_area: Area,
    input_area: Area,
    hint_area: Area,
    table_view: TableView<'t, Dep>,
    skin: MadSkin,
    status_skin: MadSkin,
}


fn u64_to_str(i: u64) -> String {
    if i==0 {
        "".to_owned()
    } else {
        format!("{}", i)
    }
}

impl<'t> VerifyScreen<'t> {
    pub fn new() -> Result<Self> {
        lazy_static! {
            static ref TS: DepTableSkin = DepTableSkin::default();
        }

        let mut columns = vec![
            Column::new(
                "crate",
                10,
                Box::new(|dep: &Dep| Cell::new(dep.name.to_string(), &TS.std)),
            ).with_align(Alignment::Left),
            Column::new(
                "version",
                9,
                Box::new(|dep: &Dep| Cell::new(dep.version.to_string(), &TS.std)),
            ).with_align(Alignment::Right),
            Column::new(
                "trust",
                6,
                Box::new(|dep: &Dep| {
                    if let Some(cdep) = dep.computed() {
                        match cdep.trust {
                            VerificationStatus::Verified => Cell::new("high".to_owned(), &TS.good),
                            VerificationStatus::Insufficient => Cell::new("none".to_owned(), &TS.none),
                            VerificationStatus::Negative => Cell::new("NO".to_owned(), &TS.bad),
                        }
                    } else {
                        Cell::new("?".to_string(), &TS.medium)
                    }
                }),
            ),
            Column::new(
                "last trusted",
                12,
                Box::new(|dep: &Dep| Cell::new(
                    dep.computed().map_or(
                        "?".to_owned(),
                        |cdep| latest_trusted_version_string(&dep.version, &cdep.latest_trusted_version)
                    ),
                    &TS.std
                )),
            ).with_align(Alignment::Right),
            Column::new(
                "reviews",
                3,
                Box::new(|dep: &Dep| Cell::new(
                    dep.computed().map_or(
                        "?".to_owned(),
                        |cdep| u64_to_str(cdep.reviews.version)
                    ),
                    &TS.std
                )),
            ).with_align(Alignment::Center),
            Column::new(
                "reviews",
                3,
                Box::new(|dep: &Dep| Cell::new(
                    dep.computed().map_or(
                        "?".to_owned(),
                        |cdep| u64_to_str(cdep.reviews.total)
                    ),
                    &TS.std
                )),
            ).with_align(Alignment::Center),
            Column::new(
                "downloads",
                8,
                Box::new(|dep: &Dep| {
                    if let Some(ComputedDep{downloads:Some(downloads),..}) = dep.computed() {
                        Cell::new(
                            format!("{}", downloads.version),
                            if downloads.version < 1000 { &TS.medium } else  { &TS.std },
                        )
                    } else {
                        Cell::new("".to_string(), &TS.std)
                    }
                }),
            ).with_align(Alignment::Right),
            Column::new(
                "downloads",
                9,
                Box::new(|dep: &Dep| {
                    if let Some(ComputedDep{downloads:Some(downloads),..}) = dep.computed() {
                        Cell::new(
                            format!("{}", downloads.total),
                            if downloads.total < 1000 { &TS.medium } else  { &TS.std },
                        )
                    } else {
                        Cell::new("".to_string(), &TS.std)
                    }
                }),
            ).with_align(Alignment::Right),
            Column::new(
                "owners",
                2,
                Box::new(|dep: &Dep| {
                    if let Some(ComputedDep{owners:Some(owners),..}) = dep.computed() {
                        Cell::new(
                            format!("{}", owners.trusted),
                            if owners.trusted > 0 { &TS.good } else  { &TS.std },
                        )
                    } else {
                        Cell::new("".to_string(), &TS.std)
                    }
                }),
            ).with_align(Alignment::Right),
            Column::new(
                "owners",
                3,
                Box::new(|dep: &Dep| {
                    if let Some(ComputedDep{owners:Some(owners),..}) = dep.computed() {
                        Cell::new(
                            format!("{}", owners.total),
                            &TS.std,
                        )
                    } else {
                        Cell::new("".to_string(), &TS.std)
                    }
                }),
            ).with_align(Alignment::Right),
            Column::new(
                "issues",
                3,
                Box::new(|dep: &Dep| {
                    if let Some(cdp) = dep.computed() {
                        Cell::new(
                            format!("{}", cdp.issues.trusted),
                            if cdp.issues.trusted > 0 { &TS.bad } else { &TS.std },
                        )
                    } else {
                        Cell::new("".to_string(), &TS.std)
                    }
                }),
            ).with_align(Alignment::Right),
            Column::new(
                "issues",
                3,
                Box::new(|dep: &Dep| {
                    if let Some(cdp) = dep.computed() {
                        Cell::new(
                            format!("{}", cdp.issues.total),
                            if cdp.issues.total > 0 { &TS.medium } else { &TS.std },
                        )
                    } else {
                        Cell::new("".to_string(), &TS.std)
                    }
                }),
            ).with_align(Alignment::Right),
            Column::new(
                "l.o.c.",
                6,
                Box::new(|dep: &Dep| {
                    if let Some(ComputedDep{loc:Some(loc),..}) = dep.computed() {
                        Cell::new(
                            format!("{}", loc),
                            &TS.std,
                        )
                    } else {
                        Cell::new("".to_string(), &TS.std)
                    }
                }),
            ).with_align(Alignment::Right),
        ];
        columns[0].set_grow(true);

        let table_view = TableView::new(
            Area::new(0, 1, 10, 10),
            columns,
        );

        let repo = Repo::auto_open_cwd()?; // TODO not extra clean
        let title = repo.name().to_string();
        let mut screen = Self {
            title,
            title_area: Area::new(0, 0, 10, 1),
            status_area: Area::new(0, 2, 10, 1),
            input_area: Area::new(0, 3, 10, 1),
            hint_area: Area::new(0, 3, 10, 1),
            table_view,
            skin: MadSkin::default(),
            status_skin: MadSkin::default(),
        };
        screen.resize();
        screen.make_skins();
        Ok(screen)
    }
    pub fn make_skins(&mut self) {
        self.skin.table.align = Alignment::Center;
        self.skin.set_headers_fg(AnsiValue(178));
        self.skin.bold.set_fg(Yellow);
        self.skin.italic.set_fg(ansi(153));
        self.skin.scrollbar.thumb.set_fg(ansi(178));
        self.status_skin.paragraph.set_bg(gray(4));
    }
    pub fn resize(&mut self) {
        let (w, h) = terminal_size();
        self.title_area.width = w;
        self.table_view.area.width = w;
        self.table_view.area.height = h - 4;
        self.table_view.update_dimensions();
        self.status_area.top = h - 3;
        self.status_area.width = w;
        self.input_area.top = h - 2;
        self.input_area.width = w / 2;
        self.hint_area.top = h - 2;
        self.hint_area.left = self.input_area.width;
        self.hint_area.width = w - self.hint_area.left;
    }
    fn update_title(&self, _table: &DepTable) {
        self.skin.write_in_area(
            &format!("# {}", &self.title),
            &self.title_area
        ).unwrap();
    }
    fn update_table_view(&mut self, table: &DepTable) {
        if table.computation_status.is_before_deps() {
            self.skin.write_in_area(
                &format!("\n*preparing table... You may quit at any time with ctrl-q*"),
                &self.table_view.area
            ).unwrap();
        } else {
            let iab = self.table_view.do_scroll_show_bottom();
            for i in self.table_view.row_count()..table.deps.len() {
                self.table_view.add_row(&table.deps[i]);
            }
            if iab {
                self.table_view.scroll_to_bottom();
            }
            self.table_view.display().unwrap();
        }
    }
    fn update_status(&self, table: &DepTable) {
        self.status_skin.write_in_area(
            &format!("status: {:?}", &table.computation_status),
            &self.status_area
        ).unwrap();
    }
    fn update_input(&self, _table: &DepTable) {
        // temporary. Main purpose is to clean the area (in case of resize)
        self.skin.write_in_area("", &self.input_area).unwrap();
    }
    fn update_hint(&self, table: &DepTable) {
        self.skin.write_in_area(
            if table.computation_status.is_before_deps() {
                "Hit *ctrl-q* to quit"
            } else {
                "Hit *ctrl-q* to quit, *PageUp* or *PageDown* to scroll"
            },
            &self.hint_area
        ).unwrap();
    }
    pub fn update_for(&mut self, table: &DepTable) {
        self.resize();
        self.update_title(table);
        self.update_table_view(table);
        self.update_status(table);
        self.update_input(table);
        self.update_hint(table);
    }
    #[allow(dead_code)]
    pub fn try_scroll_lines(&mut self, lines_count: i32) {
        self.table_view.try_scroll_lines(lines_count);
    }
    /// set the scroll amount.
    /// pages_count can be negative
    pub fn try_scroll_pages(&mut self, pages_count: i32) {
        self.table_view.try_scroll_pages(pages_count);
    }
}

