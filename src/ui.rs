use crate::{
    error::Error,
    events::{Event, Events},
    sweep::Board,
};
use num_traits::ToPrimitive;
use std::{
    convert::TryFrom,
    fmt, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
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
pub(crate) struct Ui {
    rows: u16,
    columns: u16,
    mines: u32,
    cell_width: u16,
    cell_height: u16,
}

const BOMB: &str = "💣";
const FLAG: &str = "⛳";

struct App {
    board: Board,
    active_index: usize,
}

struct Cell<'app> {
    app: &'app App,
    index: usize,
}

impl<'app> Cell<'app> {
    fn new(app: &'app App, index: usize) -> Self {
        Self { app, index }
    }

    fn is_active(&self) -> bool {
        self.app.active() == self.index
    }

    fn is_exposed(&self) -> bool {
        self.app.board.tile(self.index).unwrap().exposed
    }

    fn is_flagged(&self) -> bool {
        self.app.board.tile(self.index).unwrap().flagged
    }

    fn is_mine(&self) -> bool {
        self.app.board.tile(self.index).unwrap().mine
    }

    fn block(&self, lost: bool) -> Block {
        Block::default()
            .borders(Borders::ALL)
            .style(
                Style::default()
                    .bg(Color::Black)
                    .fg(if self.is_active() {
                        Color::Cyan
                    } else if lost && self.is_mine() {
                        Color::LightRed
                    } else {
                        Color::White
                    })
                    .add_modifier(if self.is_active() {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )
            .border_type(BorderType::Rounded)
    }

    fn text_style(&self) -> Style {
        Style::default()
            .fg(if self.is_exposed() && self.is_mine() {
                Color::LightYellow
            } else if self.is_exposed() {
                Color::White
            } else {
                Color::Black
            })
            .bg(if self.is_exposed() {
                Color::Black
            } else if self.is_active() {
                Color::Cyan
            } else {
                Color::White
            })
    }
}

impl fmt::Display for Cell<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            if self.is_flagged() {
                FLAG.to_owned()
            } else if self.is_mine() && self.is_exposed() {
                BOMB.to_owned()
            } else if self.is_exposed() {
                let num_adjacent_mines = self.app.board.tile(self.index).unwrap().adjacent_mines;
                if num_adjacent_mines == 0 {
                    " ".to_owned()
                } else {
                    format!("{}", num_adjacent_mines)
                }
            } else {
                " ".to_owned()
            }
        )
    }
}

impl App {
    fn new(board: Board) -> Self {
        Self {
            board,
            active_index: 0,
        }
    }

    fn up(&mut self) {
        if let Some(active_index) = self.active_index.checked_sub(self.board.columns) {
            self.active_index = active_index;
        }
    }

    fn down(&mut self) {
        let nrows = self.board.rows;
        self.active_index +=
            self.board.columns * usize::from(self.active_index / nrows < nrows - 1);
    }

    fn left(&mut self) {
        self.active_index -= usize::from(
            (self.active_index % self.board.columns)
                .checked_sub(1)
                .is_some(),
        );
    }

    fn right(&mut self) {
        let ncolumns = self.board.columns;
        self.active_index += usize::from(self.active_index % ncolumns < ncolumns - 1);
    }

    fn cell(&self, index: usize) -> Cell {
        Cell::new(self, index)
    }

    fn active_cell(&self) -> Cell {
        self.cell(self.active())
    }

    fn active(&self) -> usize {
        self.active_index
    }

    fn expose_active_cell(&mut self) -> Result<bool, Error> {
        self.board.expose(self.active())
    }

    fn expose_all(&mut self) -> Result<(), Error> {
        self.board.expose_all()
    }

    fn won(&self) -> bool {
        self.board.won()
    }

    fn flag_active_cell(&mut self) -> Result<(), Error> {
        self.board.flag(self.active())?;
        Ok(())
    }
}

impl Ui {
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

        let mut app = App::new(Board::new(
            rows,
            columns,
            usize::try_from(mines).map_err(Error::ConvertU32ToUsize)?,
        )?);
        let mut lost = false;

        let stdout = io::stdout()
            .into_raw_mode()
            .map_err(Error::GetStdoutInRawMode)?;
        let mouse_terminal = MouseTerminal::from(stdout);
        let alt_screen = AlternateScreen::from(mouse_terminal);
        let backend = TermionBackend::new(alt_screen);
        let mut terminal = Terminal::new(backend).map_err(Error::CreateTerminal)?;

        while running.load(Ordering::SeqCst) {
            terminal
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
                            Block::default().borders(Borders::ALL).title(Span::styled(
                                FLAG,
                                Style::default()
                                    .fg(Color::LightMagenta)
                                    .add_modifier(Modifier::BOLD),
                            )),
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
                            length = (available_flags)
                                .to_f64()
                                .unwrap()
                                .log10()
                                .ceil()
                                .to_usize()
                                .unwrap()
                                + 1
                        ))
                        .ratio(available_flags.to_f64().unwrap() / f64::from(mines));

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
                                "movement: hjkl / ← ↓ ↑ →",
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
                            Constraint::Min(vertical_pad_block_height - 3),
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
                            Block::default().borders(Borders::ALL).title(Span::styled(
                                BOMB,
                                Style::default()
                                    .fg(Color::LightYellow)
                                    .add_modifier(Modifier::BOLD),
                            )),
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

                    let mut index = 0_usize;
                    for row_rect in row_rects.into_iter() {
                        let col_rects = Layout::default()
                            .direction(Direction::Horizontal)
                            .vertical_margin(0)
                            .horizontal_margin(1)
                            .constraints(col_constraints.clone())
                            .split(row_rect);

                        for cell_rect in col_rects.into_iter() {
                            let cell = app.cell(index);
                            let single_row_text = format!(
                                "{:^length$}",
                                cell.to_string(),
                                length = usize::from(cell_width - 2)
                            );
                            let pad_line = " ".repeat(usize::from(cell_width));

                            // 1 line for the text, 1 line each for the top and bottom of the cell == 3 lines
                            // that are not eligible for padding
                            let num_pad_lines = usize::from(cell_height - 3);

                            // text is:
                            //   pad with half the pad lines budget
                            //   the interesting text
                            //   pad with half the pad lines budget
                            //   join with newlines
                            let text = std::iter::repeat(pad_line.clone())
                                .take(num_pad_lines / 2)
                                .chain(std::iter::once(single_row_text.clone()))
                                .chain(std::iter::repeat(pad_line).take(num_pad_lines / 2))
                                .collect::<Vec<_>>()
                                .join("\n");

                            let cell_text = Paragraph::new(text)
                                .block(cell.block(lost))
                                .style(cell.text_style());
                            frame.render_widget(cell_text, cell_rect);
                            index += 1;
                        }
                    }

                    // if the user has lost or won, display a banner indicating so
                    if lost || app.won() {
                        let area = centered_rect(20, 3, final_mines_rect);
                        frame.render_widget(Clear, area); //this clears out the background
                        frame.render_widget(
                            Paragraph::new(format!("You {}!", if lost { "lose" } else { "won" }))
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .border_type(BorderType::Thick)
                                        .border_style(
                                            Style::default()
                                                .fg(if lost {
                                                    Color::Magenta
                                                } else {
                                                    Color::LightGreen
                                                })
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
                    Key::Char('f') if !lost && !app.won() => app.flag_active_cell()?,
                    Key::Char(' ') if !lost && !app.won() && !app.active_cell().is_flagged() => {
                        lost = app.expose_active_cell()?;
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
