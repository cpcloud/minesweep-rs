use anyhow::{anyhow, Context, Result};
use std::io;
use structopt::StructOpt;
use termion::{input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};

mod error;
mod events;
mod sweep;
mod ui;

#[derive(Debug, structopt::StructOpt)]
struct Opt {
    /// The number of rows in the grid.
    #[structopt(short, long, default_value = "9")]
    rows: u16,

    /// The number of columns in the grid.
    #[structopt(short, long, default_value = "9")]
    columns: u16,

    /// The total number of mines in the grid.
    #[structopt(short, long, default_value = "10")]
    mines: u16,

    /// The width of each cell.
    #[structopt(short = "-w", long, default_value = "5")]
    cell_width: u16,

    /// The height of each cell.
    #[structopt(short = "-H", long, default_value = "3")]
    cell_height: u16,
}

fn main() -> Result<()> {
    let Opt {
        rows,
        columns,
        mines,
        cell_width,
        cell_height,
    } = Opt::from_args();

    if mines > rows * columns {
        return Err(anyhow!(
            "number of mines ({}) is greater than the number of cells: {}",
            mines,
            rows * columns
        ));
    }

    let stdout = io::stdout()
        .into_raw_mode()
        .context("failed to get stdout in raw mode")?;
    let mouse_terminal = MouseTerminal::from(stdout);
    let alt_screen = AlternateScreen::from(mouse_terminal);
    let backend = TermionBackend::new(alt_screen);
    let terminal = Terminal::new(backend).context("failed to construct terminal backend")?;

    ui::Ui::builder()
        .rows(rows)
        .columns(columns)
        .mines(mines)
        .cell_width(cell_width)
        .cell_height(cell_height)
        .terminal(terminal)
        .build()
        .run()
        .context("sweep failed")
}
