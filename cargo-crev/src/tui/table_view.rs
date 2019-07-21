
use std::io;
use crossterm::{
    Attribute::*,
    ClearType,
    Terminal,
    TerminalCursor,
};
use termimad::{
    compute_scrollbar,
    Area,
    CompoundStyle,
    Alignment,
    gray,
    ScrollBarStyle,
    Spacing,
};

pub struct Cell<'t> {
    con: String,
    style: &'t CompoundStyle,
    width: usize, // length of content in chars
}

pub struct Title {
    columns: Vec<usize>, // the column(s) below this title
}

pub struct Column<'t, T> {
    title: String,
    min_width: usize,
    grow: bool,
    spacing: Spacing,
    extract: Box<dyn Fn(&T) -> Cell<'t>>,
}

struct Row<'t> {
    cells: Vec<Cell<'t>>,
}

/// A skin for what's not defined by the table users
/// (meaning the style of the cells isn't here)
pub struct TableViewSkin {
    border: CompoundStyle,
    title: CompoundStyle,
    scrollbar: ScrollBarStyle,
}

pub struct TableView<'t, T> {
    titles: Vec<Title>,
    columns: Vec<Column<'t, T>>,
    rows: Vec<Row<'t>>,
    pub area: Area,
    pub scroll: i32, // 0 for no scroll, positive if scrolled
    pub skin: TableViewSkin,
}

impl<'t> Cell<'t> {
    pub fn new(con: String, style: &'t CompoundStyle) -> Self {
        let width = con.chars().count();
        Self {
            con,
            style,
            width,
        }
    }
}

impl<'t, T> Column<'t, T> {
    pub fn new(title: &str, width: usize, extract: Box<dyn Fn(&T) -> Cell<'t>>) -> Self {
        Self {
            title: title.to_owned(),
            min_width: width,
            grow: false,
            spacing: Spacing {
                width,
                align: Alignment::Center,
            },
            extract,
        }
    }
    pub fn with_align(mut self, align: Alignment) -> Self {
        self.spacing.align = align;
        self
    }
    pub fn print_cell(&self, cell: &Cell<'_>) {
        self.spacing.print_counted_str(&cell.con, cell.width, &cell.style);
    }
    pub fn set_grow(&mut self, grow: bool) {
        self.grow = grow;
    }
}

impl Default for TableViewSkin {
    fn default() -> Self {
        Self {
            border: CompoundStyle::with_fg(gray(7)),
            title: CompoundStyle::with_attr(Bold),
            scrollbar: ScrollBarStyle::new(),
        }
    }
}
impl<'t, T> TableView<'t, T> {
    pub fn new(area: Area, columns: Vec<Column<'t, T>>) -> Self {
        let mut titles: Vec<Title> = Vec::new();
        for (column_idx, column) in columns.iter().enumerate() {
            if let Some(last_title) = titles.last_mut() {
                if columns[last_title.columns[0]].title == column.title {
                    // we merge those columns titles
                    last_title.columns.push(column_idx);
                    continue;
                }
            }
            // this is a new title
            titles.push(Title {
                columns: vec![column_idx],
            });
        }
        Self {
            titles,
            columns,
            rows: Vec::new(),
            area,
            scroll: 0,
            skin: TableViewSkin::default(),
        }
    }
    /// return the height which is available for rows
    pub fn tbody_height(&self) -> i32 {
        self.area.height as i32 - 2
    }
    /// return an option which when filled contains
    ///  a tupple with the top and bottom of the vertical
    ///  scrollbar. Return none when the content fits
    ///  the available space.
    #[inline(always)]
    pub fn scrollbar(&self) -> Option<(u16, u16)> {
        compute_scrollbar(
            self.scroll,
            self.rows.len() as i32,
            self.tbody_height(),
        )
    }
    pub fn add_row(&mut self, t: &T) {
        let cells = self.columns.iter()
            .map(|column| (column.extract)(t))
            .collect();
        self.rows.push(Row {
            cells,
        });
    }
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    /// recompute the widths of all columns.
    /// This should be called when the area size is modified
    pub fn update_dimensions(&mut self) {
        let available_width: i32 =
            self.area.width as i32
            - (self.columns.len() as i32 - 1) // we remove the separator
            - 1; // we remove 1 to let space for the scrollbar
        let sum_min_widths: i32 = self.columns.iter().map(|c| c.min_width as i32).sum();
        // right now we assume there's only one growing column, because that's our current
        // use case.
        for i in 0..self.columns.len() {
            if self.columns[i].grow {
                self.columns[i].spacing.width =
                    self.columns[i].min_width
                    + (available_width - sum_min_widths).max(0) as usize;
                break;
            }
        }
    }
    pub fn display(&self) -> io::Result<()> {
        let terminal = Terminal::new();
        let cursor = TerminalCursor::new();
        let scrollbar = self.scrollbar();
        let sx = self.area.left + self.area.width;
        let mut row_idx = self.scroll as usize;
        let vbar = self.skin.border.apply_to("│");
        let tee = self.skin.border.apply_to("┬");
        let cross = self.skin.border.apply_to("┼");
        let hbar = self.skin.border.apply_to("─");
        // title line
        cursor.goto(self.area.left, self.area.top)?;
        for (title_idx, title) in self.titles.iter().enumerate() {
            if title_idx != 0 {
                print!("{}", vbar);
            }
            let width =
                title.columns.iter().map(|ci| self.columns[*ci].spacing.width).sum::<usize>()
                + title.columns.len() - 1;
            let spacing = Spacing {
                width,
                align: Alignment::Center,
            };
            spacing.print_str(
                &self.columns[title.columns[0]].title,
                &self.skin.title
            );
        }
        // separator line
        cursor.goto(self.area.left, self.area.top+1)?;
        for (title_idx, title) in self.titles.iter().enumerate() {
            if title_idx != 0 {
                print!("{}", cross);
            }
            for (col_idx_idx, col_idx) in title.columns.iter().enumerate() {
                if col_idx_idx > 0 {
                    print!("{}", tee);
                }
                for _ in 0..self.columns[*col_idx].spacing.width {
                    print!("{}", hbar);
                }
            }
        }
        // rows, maybe scrolled
        for y in 2..self.area.height {
            cursor.goto(self.area.left, self.area.top+y)?;
            if row_idx < self.rows.len() {
                let cells = &self.rows[row_idx].cells;
                for (col_idx, col) in self.columns.iter().enumerate() {
                    if col_idx != 0 {
                        print!("{}", vbar);
                    }
                    col.print_cell(&cells[col_idx]);
                }
                row_idx += 1;
            } else {
                terminal.clear(ClearType::UntilNewLine)?;
            }
            if let Some((sctop, scbottom)) = scrollbar {
                cursor.goto(sx, self.area.top+y)?;
                let y = y - 2;
                if sctop <= y && y <= scbottom {
                    print!("{}", self.skin.scrollbar.thumb);
                } else {
                    print!("{}", self.skin.scrollbar.track);
                }
            }
        }
        Ok(())
    }
    pub fn do_scroll_show_bottom(&self) -> bool {
        self.scroll + self.tbody_height() >= self.rows.len() as i32
    }
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = (self.rows.len() as i32 - self.tbody_height()).max(0);
    }
    /// set the scroll amount.
    /// lines_count can be negative
    pub fn try_scroll_lines(&mut self, lines_count: i32) {
        self.scroll = (self.scroll + lines_count)
            .min(self.rows.len() as i32 - self.tbody_height() + 1)
            .max(0);
    }
    /// set the scroll amount.
    /// pages_count can be negative
    pub fn try_scroll_pages(&mut self, pages_count: i32) {
        self.try_scroll_lines(pages_count * self.tbody_height())
    }

}
