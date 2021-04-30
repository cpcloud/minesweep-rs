use crate::{
    error::Error,
    events::{Event, Events},
    sweep::Board,
};
use std::{
    convert::TryFrom,
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use termion::event::Key;
use tui::{
    backend::TermionBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Terminal,
};

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let Rect {
        width: grid_width,
        height: grid_height,
        ..
    } = r;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(grid_height / 2 - height / 2),
                Constraint::Length(height),
                Constraint::Length(grid_height / 2 - height / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(grid_width / 2 - width / 2),
                Constraint::Length(width),
                Constraint::Length(grid_width / 2 - width / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn align_strings_to_char(strings: &[&str], c: char) -> Vec<String> {
    let (firsts, rests): (Vec<_>, Vec<_>) = strings
        .iter()
        .map(|&s| s.split_at(s.find(c).unwrap()))
        .unzip();
    let max_firsts = firsts.iter().map(|&f| f.len()).max().unwrap();
    let max_rests = rests.iter().map(|&r| r.len()).max().unwrap();
    firsts
        .into_iter()
        .zip(rests.into_iter())
        .map(|(first, rest)| {
            format!(
                "{:>left_length$}{:<right_length$}",
                first,
                rest,
                left_length = max_firsts,
                right_length = max_rests
            )
        })
        .collect()
}

#[derive(typed_builder::TypedBuilder)]
pub(crate) struct Ui<W>
where
    W: Write,
{
    rows: u16,
    columns: u16,
    mines: u16,
    cell_width: u16,
    cell_height: u16,
    terminal: Terminal<TermionBackend<W>>,
}

const BOMB: &str = "💣";
const FLAG: &str = "⛳";

struct App {
    x_pos: u16,
    y_pos: u16,
    board: Board,
}

impl App {
    fn new(board: Board) -> Self {
        Self {
            board,
            x_pos: 0,
            y_pos: 0,
        }
    }

    fn up(&mut self) {
        if let Some(y_pos) = self.y_pos.checked_sub(1) {
            self.y_pos = y_pos;
        }
    }

    fn down(&mut self) {
        if self.y_pos < self.board.nrows - 1 {
            self.y_pos += 1;
        }
    }

    fn left(&mut self) {
        if let Some(x_pos) = self.x_pos.checked_sub(1) {
            self.x_pos = x_pos;
        }
    }

    fn right(&mut self) {
        if self.x_pos < self.board.ncolumns - 1 {
            self.x_pos += 1;
        }
    }

    fn home(&mut self) {
        self.x_pos = 0;
    }

    fn end(&mut self) {
        self.x_pos = self.board.ncolumns - 1;
    }

    fn page_left(&mut self) {
        self.x_pos = self.x_pos.saturating_sub(self.board.ncolumns / 2);
    }

    fn page_right(&mut self) {
        self.x_pos = (self.x_pos + self.board.ncolumns / 2).min(self.board.ncolumns - 1)
    }

    fn page_up(&mut self) {
        self.y_pos = self.y_pos.saturating_sub(self.board.nrows / 2);
    }

    fn page_down(&mut self) {
        self.y_pos = (self.y_pos + self.board.nrows / 2).min(self.board.nrows - 1)
    }

    fn active(&self) -> (u16, u16) {
        (self.y_pos, self.x_pos)
    }

    fn expose(&mut self) -> Result<bool, Error> {
        let (i, j) = self.active();
        self.board.expose(i, j)
    }

    fn expose_all(&mut self) -> Result<(), Error> {
        self.board.expose_all()
    }

    fn win(&self) -> bool {
        self.board.win()
    }

    fn flag(&mut self) -> Result<(), Error> {
        let (r, c) = self.active();
        self.board.flag(r, c)?;
        Ok(())
    }

    fn flagged(&self, r: u16, c: u16) -> Result<bool, Error> {
        Ok(self.board.tile(r, c)?.flagged)
    }

    fn mine(&self, r: u16, c: u16) -> Result<bool, Error> {
        Ok(self.board.tile(r, c)?.mine)
    }

    fn exposed(&self, r: u16, c: u16) -> Result<bool, Error> {
        Ok(self.board.tile(r, c)?.exposed)
    }

    fn num_adjacent_mines(&self, r: u16, c: u16) -> Result<u8, Error> {
        Ok(self.board.tile(r, c)?.adjacent_mines)
    }
}

impl<W: Write> Ui<W> {
    pub(crate) fn run(&mut self) -> Result<(), Error> {
        let events = Events::new();
        let rows = self.rows;
        let columns = self.columns;
        let mines = self.mines;

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        ctrlc::set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
        })
        .map_err(Error::SetHandler)?;

        let cell_width = self.cell_width;
        let cell_height = self.cell_height;

        let padding = 1;

        let grid_width = cell_width * columns + 2 * padding;
        let grid_height = cell_height * rows + 2 * padding;

        let row_constraints = std::iter::repeat(Constraint::Length(cell_height))
            .take(rows.into())
            .collect::<Vec<_>>();

        let col_constraints = std::iter::repeat(Constraint::Length(cell_width))
            .take(columns.into())
            .collect::<Vec<_>>();

        let mut app = App::new(Board::new(rows, columns, mines)?);
        let mut lost = false;

        while running.load(Ordering::SeqCst) {
            self.terminal
                .draw(|frame| {
                    let terminal_rect = frame.size();

                    let outer_block = Block::default()
                        .borders(Borders::ALL)
                        .title(Span::styled(
                            "Minesweeper",
                            Style::default()
                                .fg(Color::LightYellow)
                                .add_modifier(Modifier::BOLD),
                        ))
                        .border_type(BorderType::Rounded);
                    frame.render_widget(outer_block, terminal_rect);

                    let outer_rects = Layout::default()
                        .direction(Direction::Vertical)
                        .vertical_margin(1)
                        .horizontal_margin(1)
                        .constraints(vec![Constraint::Min(grid_height)])
                        .split(terminal_rect);

                    let mines_rect = outer_rects[0];

                    let available_flags = app.board.available_flags();
                    let info_text = Gauge::default()
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(Span::styled(
                                    FLAG,
                                    Style::default()
                                        .fg(Color::LightMagenta)
                                        .add_modifier(Modifier::BOLD),
                                ))
                                .style(Style::default().add_modifier(Modifier::SLOW_BLINK)),
                        )
                        .gauge_style(
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        )
                        .label(format!(
                            "{:>length$}",
                            available_flags,
                            length = f64::from(available_flags).log10().ceil() as usize + 1
                        ))
                        .ratio(f64::from(available_flags) / f64::from(mines));

                    let horizontal_pad_block_width = (terminal_rect.width - grid_width) / 2;
                    let mines_rects = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(vec![
                            Constraint::Min(horizontal_pad_block_width),
                            Constraint::Length(grid_width),
                            Constraint::Min(horizontal_pad_block_width),
                        ])
                        .split(mines_rect);

                    let vertical_pad_block_height = (mines_rect.height - grid_height) / 2;
                    let middle_mines_rects = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(vec![
                            Constraint::Min(vertical_pad_block_height),
                            Constraint::Length(grid_height),
                            Constraint::Min(vertical_pad_block_height),
                        ])
                        .split(mines_rects[1]);

                    let help_text_block = List::new(
                        align_strings_to_char(
                            &[
                                "movement: hjkl / 🠔 🠗 🠕 🠖",
                                "expose tile: spacebar",
                                "flag tile: f",
                                "quit: q",
                            ],
                            ':',
                        )
                        .into_iter()
                        .map(|line| format!("{:^width$}", line, width = usize::from(grid_width)))
                        .map(ListItem::new)
                        .collect::<Vec<_>>(),
                    )
                    .block(Block::default().borders(Borders::NONE));
                    frame.render_widget(help_text_block, middle_mines_rects[2]);

                    let info_text_split_rects = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(vec![
                            Constraint::Length(vertical_pad_block_height - 3),
                            Constraint::Length(3),
                        ])
                        .split(middle_mines_rects[0]);

                    let info_mines_rects = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(info_text_split_rects[1]);
                    frame.render_widget(info_text, info_mines_rects[0]);

                    let mines_text = Paragraph::new(mines.to_string())
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(Span::styled(
                                    BOMB,
                                    Style::default()
                                        .fg(Color::LightYellow)
                                        .add_modifier(Modifier::BOLD),
                                ))
                                .style(Style::default().add_modifier(Modifier::SLOW_BLINK)),
                        )
                        .alignment(Alignment::Center);
                    frame.render_widget(mines_text, info_mines_rects[1]);

                    let mines_block = Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded);

                    let final_mines_rect = middle_mines_rects[1];
                    frame.render_widget(mines_block, final_mines_rect);

                    let row_rects = Layout::default()
                        .direction(Direction::Vertical)
                        .vertical_margin(1)
                        .horizontal_margin(0)
                        .constraints(row_constraints.clone())
                        .split(final_mines_rect);

                    for (r, row_rect) in row_rects.into_iter().enumerate() {
                        let col_rects = Layout::default()
                            .direction(Direction::Horizontal)
                            .vertical_margin(0)
                            .horizontal_margin(1)
                            .constraints(col_constraints.clone())
                            .split(row_rect);

                        let r = u16::try_from(r).unwrap();

                        for (c, cell_rect) in col_rects.into_iter().enumerate() {
                            let c = u16::try_from(c).unwrap();

                            let is_cell_active = app.active() == (r, c);
                            let is_cell_exposed = app.exposed(r, c).unwrap();
                            let is_cell_mine = app.mine(r, c).unwrap();
                            let is_cell_flagged = app.flagged(r, c).unwrap();

                            let block = Block::default()
                                .borders(Borders::ALL)
                                .style(Style::default().bg(Color::Black).fg(if is_cell_active {
                                    Color::Cyan
                                } else if lost && is_cell_mine {
                                    Color::LightRed
                                } else {
                                    Color::White
                                }))
                                .border_type(BorderType::Rounded);

                            let cell_text = if is_cell_flagged {
                                FLAG.to_owned()
                            } else if is_cell_mine && is_cell_exposed {
                                BOMB.to_owned()
                            } else if is_cell_exposed {
                                let num_mines = app.num_adjacent_mines(r, c).unwrap();
                                if num_mines == 0 {
                                    " ".to_owned()
                                } else {
                                    format!("{}", num_mines)
                                }
                            } else {
                                " ".to_owned()
                            };

                            let single_row_text = format!(
                                "{:^length$}",
                                cell_text,
                                length = usize::from(cell_width - 2)
                            );

                            let pad_line = " ".repeat(usize::from(cell_width));
                            let num_pad_lines = usize::from(cell_height - 3);
                            let lines = std::iter::repeat(pad_line.clone())
                                .take(num_pad_lines / 2)
                                .chain(std::iter::once(single_row_text.clone()))
                                .chain(std::iter::repeat(pad_line).take(num_pad_lines / 2))
                                .collect::<Vec<_>>();

                            let cell = Paragraph::new(lines.join("\n"))
                                .block(block)
                                .style(
                                    Style::default()
                                        .fg(if is_cell_exposed && is_cell_mine {
                                            Color::LightYellow
                                        } else if is_cell_exposed {
                                            Color::White
                                        } else {
                                            Color::Black
                                        })
                                        .bg(if is_cell_exposed {
                                            Color::Black
                                        } else if is_cell_active {
                                            Color::Cyan
                                        } else {
                                            Color::White
                                        }),
                                )
                                .alignment(Alignment::Left);
                            frame.render_widget(cell, cell_rect);
                        }
                    }
                    if !lost && app.win() {
                        let area = centered_rect(20, 3, final_mines_rect);
                        frame.render_widget(Clear, area); //this clears out the background
                        frame.render_widget(
                            Paragraph::new("You won!")
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .border_type(BorderType::Double)
                                        .border_style(
                                            Style::default()
                                                .fg(Color::LightGreen)
                                                .add_modifier(Modifier::BOLD),
                                        )
                                        .style(Style::default().add_modifier(Modifier::BOLD)),
                                )
                                .alignment(Alignment::Center)
                                .style(Style::default()),
                            area,
                        );
                    }
                })
                .map_err(Error::DrawToTerminal)?;

            if let Event::Input(key) = events.next().map_err(Error::GetEvent)? {
                match key {
                    // movement using arrow keys or vim movement keys
                    Key::Up | Key::Char('k') => app.up(),
                    Key::Down | Key::Char('j') => app.down(),
                    Key::Left | Key::Char('h') => app.left(),
                    Key::Right | Key::Char('l') => app.right(),
                    Key::Char('a') => app.page_left(),
                    Key::Char('e') => app.page_right(),
                    Key::Home | Key::Ctrl('a') => app.home(),
                    Key::End | Key::Ctrl('e') => app.end(),
                    Key::PageUp | Key::Ctrl('u') => app.page_up(),
                    Key::PageDown | Key::Ctrl('d') => app.page_down(),
                    Key::Char('f') if !lost && !app.win() => app.flag()?,
                    Key::Char(' ')
                        if !lost && !app.win() && {
                            let (i, j) = app.active();
                            !app.flagged(i, j)?
                        } =>
                    {
                        lost = app.expose()?;
                        if lost {
                            app.expose_all()?;
                        }
                    }
                    Key::Char('q') => break,
                    _ => {}
                }
            }
        }

        Ok(())
    }
}
