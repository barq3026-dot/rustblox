// Crafting rules: shaped recipes checked against the craft grid.
use crate::blocks::Block;
use crate::items::{Item, Stack};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Station {
    /// Bare hands: the 2x2 grid in the inventory.
    Hand,
    /// 3x3 grid of a crafting table.
    Table,
    /// 3x3 grid of a furnace (adds smelting recipes).
    Furnace,
}

pub struct Recipe {
    /// Rows of pattern characters; ' ' means an empty cell.
    pub pattern: &'static [&'static str],
    pub key: &'static [(char, Item)],
    pub out: (Item, u8),
    pub station: Station,
}

impl Recipe {
    pub fn width(&self) -> usize {
        self.pattern
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.pattern.len()
    }

    /// Items needed to build this recipe once, with how many of each.
    pub fn ingredients(&self) -> Vec<(Item, u32)> {
        let mut out: Vec<(Item, u32)> = Vec::new();
        for (ch, item) in self.key {
            let n = self
                .pattern
                .iter()
                .map(|line| line.chars().filter(|c| c == ch).count() as u32)
                .sum::<u32>();
            if n == 0 {
                continue;
            }
            match out.iter_mut().find(|(i, _)| i == item) {
                Some((_, count)) => *count += n,
                None => out.push((*item, n)),
            }
        }
        out
    }
}

const P: Item = Item::Block(Block::Planks);
const C: Item = Item::Block(Block::Cobble);

pub static RECIPES: &[Recipe] = &[
    Recipe {
        pattern: &["L"],
        key: &[('L', Item::Block(Block::Log))],
        out: (P, 4),
        station: Station::Hand,
    },
    Recipe {
        pattern: &["P", "P"],
        key: &[('P', P)],
        out: (Item::Stick, 4),
        station: Station::Hand,
    },
    Recipe {
        pattern: &["PP", "PP"],
        key: &[('P', P)],
        out: (Item::Block(Block::CraftingTable), 1),
        station: Station::Hand,
    },
    Recipe {
        pattern: &["CC", "CC"],
        key: &[('C', C)],
        out: (Item::Block(Block::StoneBricks), 4),
        station: Station::Hand,
    },
    Recipe {
        pattern: &["CCC", "C C", "CCC"],
        key: &[('C', C)],
        out: (Item::Block(Block::Furnace), 1),
        station: Station::Table,
    },
    // Smelting: ore plus coal, only at a furnace.
    Recipe {
        pattern: &["RC"],
        key: &[('R', Item::RawIron), ('C', Item::Coal)],
        out: (Item::IronIngot, 1),
        station: Station::Furnace,
    },
    Recipe {
        pattern: &["RC"],
        key: &[('R', Item::RawGold), ('C', Item::Coal)],
        out: (Item::GoldIngot, 1),
        station: Station::Furnace,
    },
    Recipe {
        pattern: &["SC"],
        key: &[('S', Item::Block(Block::Sand)), ('C', Item::Coal)],
        out: (Item::Block(Block::Glass), 1),
        station: Station::Furnace,
    },
];

pub fn allowed(required: Station, have: Station) -> bool {
    match required {
        Station::Hand => true,
        Station::Table => have == Station::Table || have == Station::Furnace,
        Station::Furnace => have == Station::Furnace,
    }
}

fn cell(grid: &[Stack; 9], size: usize, x: usize, y: usize) -> Option<Item> {
    if x >= size || y >= size {
        return None;
    }
    grid[y * size + x].item
}

fn pattern_width(recipe: &Recipe) -> usize {
    recipe.width()
}

fn matches_at(
    recipe: &Recipe,
    grid: &[Stack; 9],
    size: usize,
    ox: usize,
    oy: usize,
    flip: bool,
) -> bool {
    let h = recipe.pattern.len();
    let w = pattern_width(recipe);
    for y in 0..h {
        for x in 0..w {
            let px = if flip { w - 1 - x } else { x };
            let ch = recipe.pattern[y].chars().nth(px).unwrap_or(' ');
            let want = if ch == ' ' {
                None
            } else {
                recipe.key.iter().find(|(c, _)| *c == ch).map(|(_, i)| *i)
            };
            if want != cell(grid, size, ox + x, oy + y) {
                return false;
            }
        }
    }
    true
}

/// Tries every recipe against the non-empty part of the grid.
pub fn result(grid: &[Stack; 9], size: usize, station: Station) -> Option<Stack> {
    let mut min_x = size;
    let mut min_y = size;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut any = false;
    for y in 0..size {
        for x in 0..size {
            if !grid[y * size + x].is_empty() {
                any = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if !any {
        return None;
    }
    let w = max_x - min_x + 1;
    let h = max_y - min_y + 1;

    for recipe in RECIPES {
        if !allowed(recipe.station, station) {
            continue;
        }
        if recipe.height() != h || pattern_width(recipe) != w {
            continue;
        }
        if matches_at(recipe, grid, size, min_x, min_y, false)
            || matches_at(recipe, grid, size, min_x, min_y, true)
        {
            return Some(Stack::new(recipe.out.0, recipe.out.1));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(cells: &[(usize, Item)]) -> [Stack; 9] {
        let mut g = [Stack::EMPTY; 9];
        for (i, item) in cells {
            g[*i] = Stack::new(*item, 1);
        }
        g
    }

    #[test]
    fn log_makes_planks() {
        let g = grid(&[(0, Item::Block(Block::Log))]);
        let out = result(&g, 2, Station::Hand).unwrap();
        assert_eq!(out.item, Some(P));
        assert_eq!(out.count, 4);
    }

    #[test]
    fn planks_make_table() {
        let g = grid(&[(0, P), (1, P), (2, P), (3, P)]);
        let out = result(&g, 2, Station::Hand).unwrap();
        assert_eq!(out.item, Some(Item::Block(Block::CraftingTable)));
    }

    #[test]
    fn table_pattern_needs_space() {
        let g = grid(&[(0, P), (1, P), (2, P), (3, P)]);
        assert!(result(&g, 2, Station::Hand).is_some());
    }

    #[test]
    fn smelting_only_in_furnace() {
        let g = grid(&[(0, Item::RawIron), (1, Item::Coal)]);
        assert!(result(&g, 3, Station::Table).is_none());
        let out = result(&g, 3, Station::Furnace).unwrap();
        assert_eq!(out.item, Some(Item::IronIngot));
    }

    #[test]
    fn nothing_matches_garbage() {
        let g = grid(&[(4, Item::Diamond)]);
        assert!(result(&g, 3, Station::Hand).is_none());
    }

    #[test]
    fn ingredients_count_repeats() {
        let table = RECIPES
            .iter()
            .find(|r| r.out.0 == Item::Block(Block::CraftingTable))
            .unwrap();
        assert_eq!(table.ingredients(), vec![(P, 4)]);

        let furnace = RECIPES
            .iter()
            .find(|r| r.out.0 == Item::Block(Block::Furnace))
            .unwrap();
        assert_eq!(furnace.ingredients(), vec![(C, 8)]);

        let glass = RECIPES
            .iter()
            .find(|r| r.out.0 == Item::Block(Block::Glass))
            .unwrap();
        assert_eq!(
            glass.ingredients(),
            vec![(Item::Block(Block::Sand), 1), (Item::Coal, 1)]
        );
    }
}
