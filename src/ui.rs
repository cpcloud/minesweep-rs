use crate::{
    error::Error,
    events::{Event, Events},
};
use std::{
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
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Terminal,
};

#[derive(typed_builder::TypedBuilder)]
pub(crate) struct Ui<W>
where
    W: Write,
{
    rows: u16,
    columns: u16,
    mines: u16,
    terminal: Terminal<TermionBackend<W>>,
}

impl<W: Write> Ui<W> {
    pub(crate) fn run(&mut self) -> Result<(), Error> {
        let events = Events::new();
        let rows = self.rows;
        let columns = self.columns;

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        ctrlc::set_handler(move || {
            running_clone.store(false, Ordering::SeqCst);
        })
        .map_err(Error::SetHandler)?;

        let horizontal_length = 5;
        let vertical_length = 3;
        let padding = 1;
        let grid_width = horizontal_length * columns + 2 * padding;
        let grid_height = vertical_length * rows + 2 * padding;

        let row_constraints = std::iter::repeat(Constraint::Length(vertical_length))
            .take(rows.into())
            .collect::<Vec<_>>();

        let col_constraints = std::iter::repeat(Constraint::Length(horizontal_length))
            .take(columns.into())
            .collect::<Vec<_>>();

        while running.load(Ordering::SeqCst) {
            self.terminal
                .draw(|f| {
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .title("sweep-rs")
                        .border_type(BorderType::Thick);

                    let terminal_rect = f.size();

                    let size = Rect::new(
                        terminal_rect.width / 2 - grid_width / 2,
                        terminal_rect.height / 2 - grid_height / 2,
                        grid_width,
                        grid_height,
                    );

                    f.render_widget(block, size);

                    let row_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .vertical_margin(1)
                        .horizontal_margin(0)
                        .constraints(row_constraints.clone())
                        .split(size);

                    for row_chunk in row_chunks {
                        let col_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .vertical_margin(0)
                            .horizontal_margin(1)
                            .constraints(col_constraints.clone())
                            .split(row_chunk);

                        for col_chunk in col_chunks {
                            let block = Block::default()
                                .borders(Borders::ALL)
                                .style(Style::default().bg(Color::Black).fg(Color::White))
                                .border_type(BorderType::Rounded);
                            f.render_widget(block, col_chunk);
                        }
                    }
                })
                .map_err(Error::DrawToTerminal)?;

            if let Event::Input(key) = events.next().map_err(Error::GetEvent)? {
                if key == Key::Char('q') {
                    break;
                }
            }
        }

        Ok(())
    }
}
