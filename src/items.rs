// Items: everything the player can carry. Blocks double as items.
use crate::atlas as tex;
use crate::blocks::Block;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Block(Block),
    Coal,
    RawIron,
    RawGold,
    Diamond,
    IronIngot,
    GoldIngot,
    Stick,
    Apple,
}

const MATERIAL_BASE: u16 = 300;

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::Block(b) => b.name(),
            Item::Coal => "Coal",
            Item::RawIron => "Raw iron",
            Item::RawGold => "Raw gold",
            Item::Diamond => "Diamond",
            Item::IronIngot => "Iron ingot",
            Item::GoldIngot => "Gold ingot",
            Item::Stick => "Stick",
            Item::Apple => "Apple",
        }
    }

    /// Atlas tile used to draw the item icon.
    pub fn tile(self) -> u8 {
        match self {
            Item::Block(b) => b.def().tiles.2,
            Item::Coal => tex::ITEM_COAL,
            Item::RawIron => tex::ITEM_RAW_IRON,
            Item::RawGold => tex::ITEM_RAW_GOLD,
            Item::Diamond => tex::ITEM_DIAMOND,
            Item::IronIngot => tex::ITEM_IRON_INGOT,
            Item::GoldIngot => tex::ITEM_GOLD_INGOT,
            Item::Stick => tex::ITEM_STICK,
            Item::Apple => tex::ITEM_APPLE,
        }
    }

    pub fn block(self) -> Option<Block> {
        match self {
            Item::Block(b) => Some(b),
            _ => None,
        }
    }

    pub fn max_stack(self) -> u8 {
        match self {
            Item::Apple => 16,
            _ => 64,
        }
    }

    /// Stable id used in save files.
    pub fn id(self) -> u16 {
        match self {
            Item::Block(b) => b.id() as u16,
            Item::Coal => MATERIAL_BASE,
            Item::RawIron => MATERIAL_BASE + 1,
            Item::RawGold => MATERIAL_BASE + 2,
            Item::Diamond => MATERIAL_BASE + 3,
            Item::IronIngot => MATERIAL_BASE + 4,
            Item::GoldIngot => MATERIAL_BASE + 5,
            Item::Stick => MATERIAL_BASE + 6,
            Item::Apple => MATERIAL_BASE + 7,
        }
    }

    pub fn from_id(id: u16) -> Option<Item> {
        if id == 0 {
            return None;
        }
        if (MATERIAL_BASE..MATERIAL_BASE + 8).contains(&id) {
            return Some(match id - MATERIAL_BASE {
                0 => Item::Coal,
                1 => Item::RawIron,
                2 => Item::RawGold,
                3 => Item::Diamond,
                4 => Item::IronIngot,
                5 => Item::GoldIngot,
                6 => Item::Stick,
                _ => Item::Apple,
            });
        }
        if id > u8::MAX as u16 {
            return None;
        }
        let block = Block::from_id(id as u8);
        if block == Block::Air {
            None
        } else {
            Some(Item::Block(block))
        }
    }
}

/// One inventory slot: an item type plus a stack size.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    pub item: Option<Item>,
    pub count: u8,
}

impl Stack {
    pub const EMPTY: Stack = Stack {
        item: None,
        count: 0,
    };

    pub fn new(item: Item, count: u8) -> Self {
        Stack {
            item: Some(item),
            count: count.min(item.max_stack()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.item.is_none() || self.count == 0
    }

    pub fn same(&self, other: &Stack) -> bool {
        match (self.item, other.item) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    pub fn space(&self) -> u8 {
        match self.item {
            Some(item) => item.max_stack().saturating_sub(self.count),
            None => 64,
        }
    }

    pub fn take(&mut self, n: u8) -> Stack {
        let n = n.min(self.count);
        let item = match self.item {
            Some(item) => item,
            None => return Stack::EMPTY,
        };
        self.count -= n;
        if self.count == 0 {
            self.item = None;
        }
        Stack::new(item, n)
    }
}

impl Default for Stack {
    fn default() -> Self {
        Stack::EMPTY
    }
}
