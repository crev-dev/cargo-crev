
use std::io;
use crossterm::{AlternateScreen, TerminalCursor, Color::*};
use minimad::Alignment;
use termimad::*;


fn make_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    //skin.table.align = Alignment::Center;
    skin.set_headers_fg(AnsiValue(178));
    skin.bold.set_fg(Yellow);
    skin.italic.set_fg(Magenta);
    skin.scrollbar.thumb.set_fg(AnsiValue(178));
    skin.code.align = Alignment::Center;
    skin
}

struct Col {
    text: String,
    align: Alignment,
}

pub struct Table {
    cols: Vec<Col>,
    rows: Vec<String>,
    pub skin: MadSkin,
}
impl Table {
    pub fn new() -> Self {
        Self {
            cols: Vec::new(),
            rows: Vec::new(),
            skin: make_skin(),
        }
    }
    pub fn left_col(&mut self, text: &str) {
        self.cols.push(Col {
            text: text.to_owned(),
            align: Alignment::Left,
        });
    }
    pub fn center_col(&mut self, text: &str) {
        self.cols.push(Col {
            text: text.to_owned(),
            align: Alignment::Center,
        });
    }
    pub fn right_col(&mut self, text: &str) {
        self.cols.push(Col {
            text: text.to_owned(),
            align: Alignment::Right,
        });
    }
    fn as_markdown(&self, nb_visible_rows: Option<u16>) -> String {
        let aligns: String = self.cols.iter()
            .map(|c| match c.align {
                Alignment::Center => "|:-:",
                Alignment::Right => "|-:",
                _ => "|:-",
            })
            .collect::<Vec<&str>>()
            .join("");
        let mut markdown = String::new();
        markdown.push_str(&aligns);
        markdown.push('\n');
        let ths: String = self.cols.iter()
            .map(|c| format!("|**{}**", c.text))
            .collect::<Vec<String>>()
            .join("");
        markdown.push_str(&ths);
        markdown.push('\n');
        markdown.push_str(&aligns);
        markdown.push('\n');
        let i = if let Some(nb_visible_rows) = nb_visible_rows {
            let nb_visible_rows = nb_visible_rows as usize;
            if nb_visible_rows >= self.rows.len() {
                0
            } else {
                self.rows.len() - nb_visible_rows
            }
        } else {
            0
        };
        markdown.push_str(&self.rows[i..].join("\n"));
        if nb_visible_rows.is_none() {
            markdown.push_str("\n|-");
        }
        markdown
    }
    pub fn add_row(&mut self, row: String) {
        self.rows.push(row);
    }
    pub fn display_view(&self) -> io::Result<()> {
        let mut area = Area::full_screen();
        if area.height < 5 {
            panic!("I need more space!");
        }
        let markdown = self.as_markdown(Some(area.height - 4));
        let text = self.skin.area_text(&markdown, &area);
        let mut view = TextView::from(&area, &text);
        view.write()
    }
    pub fn print(&self) {
        let markdown = self.as_markdown(None);
        println!("{}", self.skin.term_text(&markdown));
    }
}
