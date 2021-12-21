use anyhow::{Context, Result};
use structopt::StructOpt;

mod error;
mod events;
mod sweep;
mod ui;

#[derive(Debug, structopt::StructOpt)]
struct Opt {
    /// The number of rows in the grid.
    #[structopt(short, long, default_value = "9")]
    rows: usize,

    /// The number of columns in the grid.
    #[structopt(short, long, default_value = "9")]
    columns: usize,

    /// The total number of mines in the grid. The maximum number of mines
    /// is the product of the number of rows and the number of columns.
    #[structopt(short = "-n", long, default_value = "10")]
    mines: usize,

    /// The width of each cell.
    #[structopt(short = "-w", long, default_value = "5")]
    cell_width: usize,

    /// The height of each cell.
    #[structopt(short = "-H", long, default_value = "3")]
    cell_height: usize,
}

fn main() -> Result<()> {
    let Opt {
        rows,
        columns,
        mines,
        cell_width,
        cell_height,
    } = Opt::from_args();

    ui::Ui::builder()
        .rows(rows)
        .columns(columns)
        .mines(mines.min(rows * columns))
        .cell_width(cell_width)
        .cell_height(cell_height)
        .build()
        .run()
        .context("sweep failed")
}
