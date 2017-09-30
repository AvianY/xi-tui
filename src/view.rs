use std::io::Write;
use std::collections::HashMap;

use xrl::{Line, Update};
use termion::clear::CurrentLine as ClearLine;
use termion::cursor::Goto;

use cache::LineCache;
use xrl::Style;
use window::Window;

use errors::*;

const TAB_LENGTH: u16 = 4;

#[derive(Debug, Default)]
pub struct Cursor {
    pub line: u64,
    pub column: u64,
}


#[derive(Debug)]
pub struct View {
    cache: LineCache,
    cursor: Cursor,
    window: Window,
    styles: HashMap<u64, Style>,
}

impl View {
    pub fn new() -> View {
        View {
            cache: LineCache::new(),
            cursor: Default::default(),
            window: Window::new(),
            styles: HashMap::new(),
        }
    }

    pub fn set_style(&mut self, style: Style) {
        self.styles.insert(style.id, style);
    }

    pub fn update_cache(&mut self, update: Update) {
        self.cache.update(update)
    }

    pub fn set_cursor(&mut self, line: u64, column: u64) {
        self.cursor = Cursor {
            line: line,
            column: column,
        };
        self.window.update(&self.cursor);
    }

    pub fn render<W: Write>(&mut self, w: &mut W) -> Result<()> {
        self.render_lines(w)?;
        self.render_cursor(w)?;
        Ok(())
    }

    pub fn resize(&mut self, height: u16) {
        let cursor_line = self.cursor.line;
        let nb_lines = self.cache.lines.len() as u64;
        self.window.resize(height, cursor_line, nb_lines);
    }

    fn render_lines<W: Write>(&self, w: &mut W) -> Result<()> {
        debug!("Rendering lines");

        // Get the lines that are within the displayed window
        let lines = self.cache
            .lines
            .iter()
            .skip(self.window.start() as usize)
            .take(self.window.size() as usize);

        // Draw the valid lines within this range
        for (lineno, line) in lines.enumerate() {
            // Get the line vertical offset so that we know where to draw it.
            let line_pos = self.window
                .offset(self.window.start() + lineno as u64)
                .ok_or_else(|| {
                    error!("Could not find line position within the window");
                    ErrorKind::DisplayError
                })?;

            self.render_line(w, line, line_pos + 1);
        }
        Ok(())
    }

    fn render_line<W: Write>(&self, w: &mut W, line: &Line, offset: u16) -> Result<()> {
        let mut text = line.text.clone();
        trim_new_line(&mut text);
        // self.add_styles(&line.styles, &mut text)?;
        write!(w, "{}{}{}", Goto(1, offset), ClearLine, text)
            .chain_err(|| ErrorKind::DisplayError)?;
        Ok(())
    }

    fn add_styles(&self, styles: &Vec<u64>, text: &mut String) -> Result<()> {
        //if self.styles.is_empty() {
        //    return Ok(());
        //}
        // FIXME: this fails with multiple style.
        // especially if the offset is negative in which case it even panics
        // also we don't handle style ids
        //let mut style_idx = 0;
        //for style in self.style {
        //    let start = style.offset as usize;
        //    let end = start + style.length as usize;

        //    if end >= text.len() {
        //        text.push_str(&format!("{}", termion::style::Reset));
        //    } else {
        //        text.insert_str(end, &format!("{}", termion::style::Reset));
        //    }
        //    text.insert_str(start, &format!("{}", termion::style::Invert));
        //}
        Ok(())
    }

    pub fn render_cursor<W: Write>(&self, w: &mut W) -> Result<()> {
        debug!("Rendering cursor");
        if !self.window.is_within_window(self.cursor.line) {
            error!(
                "Cursor is on line {} which is not within the displayed window",
                self.cursor.line
            );
            bail!(ErrorKind::DisplayError)
        }

        // Get the line that has the cursor
        let line = self.cache
            .lines
            .get(self.cursor.line as usize)
            .ok_or_else(|| {
                error!("No valid line at cursor index {}", self.cursor.line);
                ErrorKind::DisplayError
            })?;

        // Get the line vertical offset so that we know where to draw it.
        let line_pos = self.window.offset(self.cursor.line).ok_or_else(|| {
            error!("Could not find line position within the window: {:?}", line);
            ErrorKind::DisplayError
        })?;

        // Calculate the cursor position on the line. The trick is that we know the position within
        // the string, but characters may have various lengths. For the moment, we only handle
        // tabs, and we assume the terminal has tabstops of TAB_LENGTH. We consider that all the
        // other characters have a width of 1.
        let column = line.text
            .chars()
            .take(self.cursor.column as usize)
            .fold(0, add_char_width);

        // Draw the cursor
        let cursor_pos = Goto(column as u16 + 1, line_pos + 1);
        write!(w, "{}", cursor_pos).chain_err(|| ErrorKind::DisplayError)?;
        debug!("Cursor set at line {} column {}", line_pos, column);
        Ok(())
    }
}

fn add_char_width(acc: u16, c: char) -> u16 {
    if c == '\t' {
        acc + TAB_LENGTH - (acc % TAB_LENGTH)
    } else {
        acc + 1
    }
}

fn trim_new_line(text: &mut String) {
    if let Some('\n') = text.chars().last() {
        text.pop();
    }
}
