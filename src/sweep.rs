use crate::error::Error;
use bit_set::BitSet;
use std::collections::VecDeque;

pub(crate) type Coordinate = (usize, usize);

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
    fn offset(&self, value: usize) -> usize {
        match *self {
            Self::One => value + 1,
            Self::NegOne => value.saturating_sub(1),
            Self::Zero => value,
        }
    }
}

fn adjacent((row, column): Coordinate, rows: usize, columns: usize) -> impl Iterator<Item = usize> {
    const INCREMENTS: [Increment; 3] = [Increment::One, Increment::NegOne, Increment::Zero];

    INCREMENTS
        .iter()
        .copied()
        .flat_map(|row_incr| std::iter::repeat(row_incr).zip(INCREMENTS))
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
    seen: BitSet<usize>,
}

fn index_from_coord((r, c): Coordinate, columns: usize) -> usize {
    r * columns + c
}

fn coord_from_index(index: usize, columns: usize) -> Coordinate {
    (index / columns, index % columns)
}

impl Board {
    pub(crate) fn new(rows: usize, columns: usize, mines: usize) -> Result<Self, Error> {
        let mut rng = rand::thread_rng();
        let samples = rand::seq::index::sample(&mut rng, rows * columns, mines)
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
            rows,
            columns,
            tiles,
            mines,
            flagged_cells: Default::default(),
            correctly_flagged_mines: Default::default(),
            seen: Default::default(),
        })
    }

    pub(crate) fn available_flags(&self) -> usize {
        assert!(self.flagged_cells <= self.mines);
        self.mines - self.flagged_cells
    }

    pub(crate) fn won(&self) -> bool {
        let exposed_or_correctly_flagged = self.seen.len() + self.correctly_flagged_mines;
        let ntiles = self.rows * self.columns;
        assert!(exposed_or_correctly_flagged <= ntiles);
        ntiles == exposed_or_correctly_flagged
    }

    fn index_from_coord(&self, (r, c): Coordinate) -> usize {
        index_from_coord((r, c), self.columns)
    }

    pub(crate) fn expose(&mut self, (r, c): Coordinate) -> Result<bool, Error> {
        if self.tile(r, c)?.mine {
            self.tile_mut(r, c)?.exposed = true;
            return Ok(true);
        }

        let mut coordinates = [(r, c)].iter().copied().collect::<VecDeque<_>>();

        let columns = self.columns;

        while let Some((r, c)) = coordinates.pop_front() {
            if self.seen.insert(self.index_from_coord((r, c))) {
                let tile = self.tile_mut(r, c)?;

                tile.exposed = !(tile.mine || tile.flagged);

                if tile.adjacent_mines == 0 {
                    coordinates.extend(
                        tile.adjacent_tiles
                            .iter()
                            .map(move |index| coord_from_index(index, columns)),
                    );
                }
            };
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

    pub(crate) fn tile(&self, i: usize, j: usize) -> Result<&Tile, Error> {
        self.tiles
            .get(self.index_from_coord((i, j)))
            .ok_or(Error::GetTile((i, j)))
    }

    pub(crate) fn tile_mut(&mut self, i: usize, j: usize) -> Result<&mut Tile, Error> {
        let index = self.index_from_coord((i, j));
        self.tiles.get_mut(index).ok_or(Error::GetTile((i, j)))
    }

    pub(crate) fn flag(&mut self, i: usize, j: usize) -> Result<bool, Error> {
        let nflagged = self.flagged_cells;
        let tile = self.tile(i, j)?;
        let was_flagged = tile.flagged;
        let flagged = !was_flagged;
        let nmines = self.mines;
        self.correctly_flagged_mines += usize::from(flagged && tile.mine);
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
