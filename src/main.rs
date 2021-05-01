use crate::error::Error;
use anyhow::{Context, Result};
use structopt::StructOpt;

mod error;
mod events;
mod sweep;
mod ui;

fn parse_nonzero_u16(src: &str) -> Result<u16, Error> {
    let value = src
        .parse::<u16>()
        .map_err(|e| Error::ParseNum(e, src.to_owned()))?;
    if value == 0 {
        return Err(error::Error::GetNonZeroValue);
    }
    Ok(value)
}

fn parse_nonzero_u32(src: &str) -> Result<u32, Error> {
    let value = src
        .parse::<u32>()
        .map_err(|e| Error::ParseNum(e, src.to_owned()))?;
    if value == 0 {
        return Err(error::Error::GetNonZeroValue);
    }
    Ok(value)
}

#[derive(Debug, structopt::StructOpt)]
struct Opt {
    /// The number of rows in the grid.
    #[structopt(
        short,
        long,
        default_value = "9",
        parse(try_from_str = parse_nonzero_u16)
    )]
    rows: u16,

    /// The number of columns in the grid.
    #[structopt(
        short,
        long,
        default_value = "9",
        parse(try_from_str = parse_nonzero_u16)
    )]
    columns: u16,

    /// The total number of mines in the grid. The maximum number of mines
    /// is the product of the number of rows and the number of columns.
    #[structopt(
        short = "-n",
        long,
        default_value = "10",
        parse(try_from_str = parse_nonzero_u32)
    )]
    mines: u32,

    /// The width of each cell.
    #[structopt(
        short = "-w",
        long,
        default_value = "5",
        parse(try_from_str = parse_nonzero_u16)
    )]
    cell_width: u16,

    /// The height of each cell.
    #[structopt(
        short = "-H",
        long,
        default_value = "3",
        parse(try_from_str = parse_nonzero_u16)
    )]
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

    ui::Ui::builder()
        .rows(rows)
        .columns(columns)
        .mines(mines.min(u32::from(rows) * u32::from(columns)))
        .cell_width(cell_width)
        .cell_height(cell_height)
        .build()
        .run()
        .context("sweep failed")
}
