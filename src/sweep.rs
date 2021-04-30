use crate::error::Error;
use itertools::Itertools;
use std::{
    collections::{HashSet, VecDeque},
    convert::TryFrom,
};

pub(crate) type Coordinate = (u16, u16);

#[derive(Debug)]
pub(crate) struct Tile {
    adjacent_tiles: HashSet<Coordinate>,
    pub(crate) mine: bool,
    pub(crate) exposed: bool,
    pub(crate) flagged: bool,
    pub(crate) adjacent_mines: u8,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Increment {
    One,
    NegOne,
    Zero,
}

impl Increment {
    fn offset(&self, value: u16) -> u16 {
        match *self {
            Self::One => value + 1,
            Self::NegOne => value.saturating_sub(1),
            Self::Zero => value,
        }
    }
}

const INCREMENTS: [Increment; 3] = [Increment::One, Increment::NegOne, Increment::Zero];

fn adjacent(
    (i, j): Coordinate,
    (nrows, ncolumns): (u16, u16),
) -> Result<impl Iterator<Item = Coordinate>, Error> {
    Ok(INCREMENTS
        .iter()
        .copied()
        .cartesian_product(INCREMENTS.iter().copied())
        .filter_map(move |(x, y)| {
            let x_offset = x.offset(i);
            let y_offset = y.offset(j);
            if (x != Increment::Zero || y != Increment::Zero)
                && x_offset < nrows
                && y_offset < ncolumns
            {
                Some((x_offset, y_offset))
            } else {
                None
            }
        }))
}

pub(crate) struct Board {
    tiles: Vec<Tile>,
    pub(crate) rows: u16,
    pub(crate) columns: u16,
    mines: u16,
    flagged_cells: u16,
}

fn index_from_coord((r, c): Coordinate, ncols: u16) -> usize {
    usize::from(r * ncols + c)
}

fn coord_from_index(index: usize, ncols: u16) -> Coordinate {
    (
        u16::try_from(index / usize::from(ncols)).unwrap(),
        u16::try_from(index % usize::from(ncols)).unwrap(),
    )
}

impl Board {
    pub(crate) fn new(nrows: u16, ncolumns: u16, nmines: u16) -> Result<Self, Error> {
        let mut rng = rand::thread_rng();
        let samples =
            rand::seq::index::sample(&mut rng, usize::from(nrows * ncolumns), usize::from(nmines))
                .into_iter()
                .collect::<HashSet<_>>();

        let grid = (0..nrows)
            .cartesian_product(0..ncolumns)
            .enumerate()
            .map(|(i, point)| {
                let adjacent_tiles = adjacent(point, (nrows, ncolumns))?.collect::<HashSet<_>>();

                // sum the number of adjacent tiles that are in the randomly generated mines set
                let adjacent_mines = adjacent_tiles.iter().fold(0, |total, &coord| {
                    total + u8::from(samples.contains(&index_from_coord(coord, ncolumns)))
                });
                assert!(adjacent_mines <= 8);

                Ok(Tile {
                    adjacent_tiles,
                    mine: samples.contains(&i),
                    exposed: false,
                    flagged: false,
                    adjacent_mines,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            rows: nrows,
            columns: ncolumns,
            tiles: grid,
            mines: nmines,
            flagged_cells: 0,
        })
    }

    pub(crate) fn available_flags(&self) -> u16 {
        self.mines - self.flagged_cells
    }

    pub(crate) fn won(&self) -> bool {
        let correctly_flagged_mines = self
            .tiles
            .iter()
            .map(|tile| u16::from(tile.flagged && tile.mine))
            .sum::<u16>();
        let total_exposed = self
            .tiles
            .iter()
            .map(|tile| u16::from(tile.exposed))
            .sum::<u16>();
        let exposed_or_correctly_flagged = total_exposed + correctly_flagged_mines;
        let ntiles = self.rows * self.columns;
        assert!(exposed_or_correctly_flagged <= ntiles);
        ntiles == exposed_or_correctly_flagged
    }

    pub(crate) fn expose(&mut self, (i, j): Coordinate) -> Result<bool, Error> {
        if self.tile(i, j)?.mine {
            self.tile_mut(i, j)?.exposed = true;
            return Ok(true);
        }

        let mut seen = HashSet::new();
        let mut coordinates = [(i, j)].iter().copied().collect::<VecDeque<_>>();

        while let Some((x, y)) = coordinates.pop_front() {
            if seen.insert((x, y)) {
                let tile = self.tile_mut(x, y)?;

                tile.exposed = !(tile.mine || tile.flagged);

                if tile.adjacent_mines == 0 {
                    coordinates.extend(tile.adjacent_tiles.iter());
                }
            }
        }

        Ok(false)
    }

    pub(crate) fn expose_all(&mut self) -> Result<(), Error> {
        let len = self.tiles.len();
        let columns = self.columns;
        for coord in (0..len).map(move |i| coord_from_index(i, columns)) {
            self.expose(coord)?;
        }
        Ok(())
    }

    pub(crate) fn tile(&self, i: u16, j: u16) -> Result<&Tile, Error> {
        self.tiles
            .get(index_from_coord((i, j), self.columns))
            .ok_or(Error::GetTile(i, j))
    }

    pub(crate) fn tile_mut(&mut self, i: u16, j: u16) -> Result<&mut Tile, Error> {
        self.tiles
            .get_mut(index_from_coord((i, j), self.columns))
            .ok_or(Error::GetTile(i, j))
    }

    pub(crate) fn flag(&mut self, i: u16, j: u16) -> Result<bool, Error> {
        let nflagged = self.flagged_cells;
        let was_flagged = self.tile(i, j)?.flagged;
        let flagged = !was_flagged;
        let nmines = self.mines;
        if was_flagged {
            self.flagged_cells = self.flagged_cells.saturating_sub(1);
            self.tile_mut(i, j)?.flagged = flagged;
        } else if nflagged < nmines && !self.tile(i, j)?.exposed {
            self.tile_mut(i, j)?.flagged = flagged;
            self.flagged_cells += 1;
        }
        Ok(flagged)
    }
}
