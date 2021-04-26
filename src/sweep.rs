use crate::error::Error;
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};

type Coordinate = (u16, u16);

struct Tile {
    adjacent_tiles: HashSet<Coordinate>,
    mine: bool,
    exposed: bool,
    flagged: bool,
    adjacent_mines: HashSet<Coordinate>,
}

impl Tile {
    pub(crate) fn new(adjacent_tiles: impl IntoIterator<Item = Coordinate>, mine: bool) -> Self {
        Self {
            adjacent_tiles: adjacent_tiles.into_iter().collect(),
            mine,
            exposed: false,
            flagged: false,
            adjacent_mines: HashSet::new(),
        }
    }

    pub(crate) fn correctly_flagged(&self) -> bool {
        self.flagged && self.mine
    }

    pub(crate) fn exposed(&self) -> bool {
        !(self.mine || self.flagged)
    }
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
            Self::NegOne => value - 1,
            Self::Zero => value,
        }
    }
}

fn adjacent(
    (i, j): Coordinate,
    (nrows, ncolumns): (u16, u16),
) -> Result<HashSet<Coordinate>, Error> {
    let increments = [Increment::One, Increment::NegOne, Increment::Zero];

    Ok(increments
        .iter()
        .copied()
        .cartesian_product(increments.iter().copied())
        .filter_map(|(x, y)| {
            let x_offset = x.offset(i);
            let y_offset = y.offset(j);
            if x != Increment::Zero
                || y != Increment::Zero
                    && (0..nrows).contains(&x_offset)
                    && (0..ncolumns).contains(&y_offset)
            {
                Some((x_offset, y_offset))
            } else {
                None
            }
        })
        .collect())
}

pub(crate) struct Board {
    nrows: u16,
    ncolumns: u16,
    grid: HashMap<Coordinate, Tile>,
    nmines: u16,
    nflagged: u16,
}

impl Board {
    pub(crate) fn new(nrows: u16, ncolumns: u16, nmines: u16) -> Result<Self, Error> {
        let mut rng = rand::thread_rng();
        let samples =
            rand::seq::index::sample(&mut rng, usize::from(nrows * ncolumns), nmines as usize)
                .into_iter()
                .collect::<HashSet<_>>();

        let grid = (0..nrows)
            .cartesian_product(0..ncolumns)
            .enumerate()
            .map(|(i, point)| {
                Ok((
                    point,
                    Tile::new(adjacent(point, (nrows, ncolumns))?, samples.contains(&i)),
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(Self {
            nrows,
            ncolumns,
            grid,
            nmines,
            nflagged: 0,
        })
    }

    fn ntiles(&self) -> u16 {
        self.nrows * self.ncolumns
    }

    pub(crate) fn unexposed_tiles(&self) -> u16 {
        self.ntiles() - self.nmines
    }

    pub(crate) fn available_flags(&self) -> u16 {
        self.nmines - self.nflagged
    }

    pub(crate) fn total_exposed(&self) -> u16 {
        self.grid.values().map(|tile| u16::from(tile.exposed)).sum()
    }

    pub(crate) fn win(&self) -> bool {
        let correctly_flagged_mines = self
            .grid
            .values()
            .map(|tile| u16::from(tile.correctly_flagged()))
            .sum::<u16>();
        let exposed_or_correctly_flagged = self.total_exposed() + correctly_flagged_mines;
        assert!(exposed_or_correctly_flagged <= self.ntiles());
        self.ntiles() == exposed_or_correctly_flagged
    }

    pub(crate) fn expose(&mut self, i: u16, j: u16) -> Result<HashSet<Coordinate>, Error> {
        if self.tile(i, j)?.mine {
            self.grid
                .get_mut(&(i, j))
                .ok_or(Error::GetTile(i, j))?
                .exposed = true;
            return Ok([(i, j)].iter().copied().collect());
        }

        let mut seen = HashSet::new();
        let mut coordinates = [(i, j)].iter().copied().collect::<VecDeque<_>>();
        let mut exposed = HashSet::new();

        while let Some((x, y)) = coordinates.pop_front() {
            if seen.insert((x, y)) {
                let tile = self.tile(x, y)?;

                if tile.exposed() {
                    exposed.insert((x, y));
                }

                if tile.adjacent_mines.is_empty() {
                    coordinates.extend(tile.adjacent_tiles.iter());
                }
            }
        }

        Ok(exposed)
    }

    fn tile(&self, i: u16, j: u16) -> Result<&Tile, Error> {
        self.grid.get(&(i, j)).ok_or(Error::GetTile(i, j))
    }

    fn tile_mut(&mut self, i: u16, j: u16) -> Result<&mut Tile, Error> {
        self.grid.get_mut(&(i, j)).ok_or(Error::GetTile(i, j))
    }

    pub(crate) fn flag(&mut self, i: u16, j: u16) -> Result<bool, Error> {
        let nflagged = self.nflagged;
        let was_flagged = self.tile(i, j)?.flagged;
        let flagged = !was_flagged;
        let nmines = self.nmines;
        if was_flagged {
            if nflagged.checked_sub(1).is_some() {
                self.tile_mut(i, j)?.flagged = flagged;
                self.nflagged -= 1;
            }
        } else {
            if nflagged + 1 <= nmines && !self.grid[&(i, j)].exposed() {
                self.tile_mut(i, j)?.flagged = flagged;
                self.nflagged += 1;
            }
        }
        Ok(flagged)
    }
}
