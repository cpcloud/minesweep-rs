use crate::error::Error;
use bit_set::BitSet;
use std::{collections::VecDeque, convert::TryFrom};

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
        .flat_map(|row_incr| std::iter::repeat(row_incr).zip(INCREMENTS.iter().copied()))
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
    // number of rows on the board
    pub(crate) rows: usize,
    // number of columns on the board
    pub(crate) columns: usize,
    // the total number of mines
    mines: usize,
    flagged_cells: usize,
    // the total number of correctly flagged mines, allows checking a win in O(1)
    correctly_flagged_mines: usize,
    // the exposed tiles
    seen: BitSet<u32>,
}

fn index_from_coord((r, c): Coordinate, columns: u16) -> usize {
    usize::from(r * columns + c)
}

impl Board {
    pub(crate) fn new(rows: u16, columns: u16, mines: usize) -> Result<Self, Error> {
        let mut rng = rand::thread_rng();
        let samples =
            rand::seq::index::sample(&mut rng, usize::from(rows) * usize::from(columns), mines)
                .into_iter()
                .collect::<BitSet>();

        let tiles = (0..rows)
            .flat_map(|row| std::iter::repeat(row).zip(0..columns))
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
            rows: usize::from(rows),
            columns: usize::from(columns),
            tiles,
            mines,
            flagged_cells: Default::default(),
            correctly_flagged_mines: Default::default(),
            seen: Default::default(),
        })
    }

    pub(crate) fn available_flags(&self) -> usize {
        self.mines - self.flagged_cells
    }

    pub(crate) fn won(&self) -> bool {
        let exposed_or_correctly_flagged = self.seen.len() + self.correctly_flagged_mines;
        let ntiles = self.rows * self.columns;
        assert!(exposed_or_correctly_flagged <= ntiles);
        ntiles == exposed_or_correctly_flagged
    }

    pub(crate) fn coord_from_index(&self, index: usize) -> Coordinate {
        let columns = self.columns;
        (
            u16::try_from(index / columns).unwrap(),
            u16::try_from(index % columns).unwrap(),
        )
    }

    pub(crate) fn expose(&mut self, index: usize) -> Result<bool, Error> {
        if self.tile(index)?.mine {
            self.tile_mut(index)?.exposed = true;
            return Ok(true);
        }

        let mut coordinates = [index].iter().copied().collect::<VecDeque<_>>();

        while let Some(index) = coordinates.pop_front() {
            if self.seen.insert(index) {
                let tile = self.tile_mut(index)?;

                tile.exposed = !(tile.mine || tile.flagged);

                if tile.adjacent_mines == 0 {
                    coordinates.extend(&tile.adjacent_tiles);
                }
            };
        }

        Ok(false)
    }

    pub(crate) fn expose_all(&mut self) -> Result<(), Error> {
        (0..self.tiles.len()).try_for_each(|index| {
            self.expose(index)?;
            Ok(())
        })
    }

    pub(crate) fn tile(&self, index: usize) -> Result<&Tile, Error> {
        self.tiles
            .get(index)
            .ok_or_else(|| Error::GetTile(self.coord_from_index(index)))
    }

    pub(crate) fn tile_mut(&mut self, index: usize) -> Result<&mut Tile, Error> {
        let coord = self.coord_from_index(index);
        self.tiles.get_mut(index).ok_or(Error::GetTile(coord))
    }

    pub(crate) fn flag(&mut self, index: usize) -> Result<bool, Error> {
        let nflagged = self.flagged_cells;
        let tile = self.tile(index)?;
        let was_flagged = tile.flagged;
        let flagged = !was_flagged;
        let nmines = self.mines;
        self.correctly_flagged_mines += usize::from(flagged && tile.mine);
        if was_flagged {
            self.flagged_cells = self.flagged_cells.saturating_sub(1);
            self.tile_mut(index)?.flagged = flagged;
        } else if nflagged < nmines && !self.tile(index)?.exposed {
            self.tile_mut(index)?.flagged = flagged;
            self.flagged_cells += 1;
        }
        Ok(flagged)
    }
}
