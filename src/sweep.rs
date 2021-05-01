use crate::error::Error;
use bit_set::BitSet;
use itertools::Itertools;
use std::{
    collections::{HashSet, VecDeque},
    convert::TryFrom,
};

pub(crate) type Coordinate = (u16, u16);

#[derive(Debug)]
pub(crate) struct Tile {
    adjacent_tiles: BitSet,
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

fn adjacent((row, column): Coordinate, rows: u16, columns: u16) -> impl Iterator<Item = usize> {
    const INCREMENTS: [Increment; 3] = [Increment::One, Increment::NegOne, Increment::Zero];

    INCREMENTS
        .iter()
        .copied()
        .cartesian_product(INCREMENTS.iter().copied())
        .filter_map(move |(row_incr, column_incr)| {
            let row_offset = row_incr.offset(row);
            let column_offset = column_incr.offset(column);

            match (row_incr, column_incr) {
                (Increment::Zero, Increment::Zero) => None,
                (_, _) if row_offset < rows && column_offset < columns => {
                    Some(index_from_coord((row_offset, column_offset), columns))
                }
                _ => None,
            }
        })
}

pub(crate) struct Board {
    tiles: Vec<Tile>,
    pub(crate) rows: u16,
    pub(crate) columns: u16,
    mines: u16,
    flagged_cells: u16,
}

fn index_from_coord((r, c): Coordinate, columns: u16) -> usize {
    usize::from(r * columns + c)
}

fn coord_from_index(index: usize, columns: u16) -> Coordinate {
    let columns = usize::from(columns);
    (
        u16::try_from(index / columns).unwrap(),
        u16::try_from(index % columns).unwrap(),
    )
}

impl Board {
    pub(crate) fn new(rows: u16, columns: u16, mines: u16) -> Result<Self, Error> {
        let mut rng = rand::thread_rng();
        let samples =
            rand::seq::index::sample(&mut rng, usize::from(rows * columns), usize::from(mines))
                .into_iter()
                .collect::<BitSet>();

        let tiles = (0..rows)
            .cartesian_product(0..columns)
            .enumerate()
            .map(|(i, point)| {
                // compute the tiles adjacent to the one being constructed
                let adjacent_tiles = adjacent(point, rows, columns).collect::<BitSet>();

                // sum the number of adjacent tiles that are in the randomly generated mines set
                let adjacent_mines = adjacent_tiles
                    .iter()
                    .fold(0, |total, index| total + u8::from(samples.contains(index)));
                assert!(adjacent_mines <= 8);

                Tile {
                    adjacent_tiles,
                    mine: samples.contains(i),
                    exposed: false,
                    flagged: false,
                    adjacent_mines,
                }
            })
            .collect::<Vec<_>>();

        Ok(Self {
            rows,
            columns,
            tiles,
            mines,
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

    pub(crate) fn expose(&mut self, (r, c): Coordinate) -> Result<bool, Error> {
        if self.tile(r, c)?.mine {
            self.tile_mut(r, c)?.exposed = true;
            return Ok(true);
        }

        let mut seen = HashSet::new();
        let mut coordinates = [(r, c)].iter().copied().collect::<VecDeque<_>>();

        let columns = self.columns;

        while let Some((r, c)) = coordinates.pop_front() {
            if seen.insert((r, c)) {
                let tile = self.tile_mut(r, c)?;

                tile.exposed = !(tile.mine || tile.flagged);

                if tile.adjacent_mines == 0 {
                    coordinates.extend(
                        tile.adjacent_tiles
                            .iter()
                            .map(move |index| coord_from_index(index, columns)),
                    );
                }
            }
        }

        Ok(false)
    }

    pub(crate) fn expose_all(&mut self) -> Result<(), Error> {
        let columns = self.columns;
        (0..self.tiles.len())
            .map(move |i| coord_from_index(i, columns))
            .try_for_each(|coord| {
                self.expose(coord)?;
                Ok(())
            })
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
