// Block registry: id, display name, atlas tiles per face and physical flags.
use crate::atlas as tex;
use crate::items::Item;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Air,
    Grass,
    Dirt,
    Stone,
    Cobble,
    Sand,
    Log,
    Leaves,
    Planks,
    Water,
    Glass,
    Bedrock,
    CoalOre,
    IronOre,
    Snow,
    GoldOre,
    DiamondOre,
    CraftingTable,
    Furnace,
    StoneBricks,
}

pub struct BlockDef {
    pub name: &'static str,
    /// (top, bottom, side) atlas tiles.
    pub tiles: (u8, u8, u8),
    pub solid: bool,
    pub transparent: bool,
}

impl Block {
    pub fn def(self) -> BlockDef {
        use Block::*;
        let d = match self {
            Air => BlockDef { name: "Air", tiles: (0, 0, 0), solid: false, transparent: true },
            Grass => BlockDef { name: "Grass", tiles: (tex::GRASS_TOP, tex::DIRT, tex::GRASS_SIDE), solid: true, transparent: false },
            Dirt => BlockDef { name: "Dirt", tiles: (tex::DIRT, tex::DIRT, tex::DIRT), solid: true, transparent: false },
            Stone => BlockDef { name: "Stone", tiles: (tex::STONE, tex::STONE, tex::STONE), solid: true, transparent: false },
            Cobble => BlockDef { name: "Cobblestone", tiles: (tex::COBBLE, tex::COBBLE, tex::COBBLE), solid: true, transparent: false },
            Sand => BlockDef { name: "Sand", tiles: (tex::SAND, tex::SAND, tex::SAND), solid: true, transparent: false },
            Log => BlockDef { name: "Log", tiles: (tex::LOG_TOP, tex::LOG_TOP, tex::LOG_SIDE), solid: true, transparent: false },
            Leaves => BlockDef { name: "Leaves", tiles: (tex::LEAVES, tex::LEAVES, tex::LEAVES), solid: true, transparent: false },
            Planks => BlockDef { name: "Planks", tiles: (tex::PLANKS, tex::PLANKS, tex::PLANKS), solid: true, transparent: false },
            Water => BlockDef { name: "Water", tiles: (tex::WATER, tex::WATER, tex::WATER), solid: false, transparent: true },
            Glass => BlockDef { name: "Glass", tiles: (tex::GLASS, tex::GLASS, tex::GLASS), solid: true, transparent: true },
            Bedrock => BlockDef { name: "Bedrock", tiles: (tex::BEDROCK, tex::BEDROCK, tex::BEDROCK), solid: true, transparent: false },
            CoalOre => BlockDef { name: "Coal ore", tiles: (tex::COAL_ORE, tex::COAL_ORE, tex::COAL_ORE), solid: true, transparent: false },
            IronOre => BlockDef { name: "Iron ore", tiles: (tex::IRON_ORE, tex::IRON_ORE, tex::IRON_ORE), solid: true, transparent: false },
            Snow => BlockDef { name: "Snow", tiles: (tex::SNOW, tex::SNOW, tex::SNOW), solid: true, transparent: false },
            GoldOre => BlockDef { name: "Gold ore", tiles: (tex::GOLD_ORE, tex::GOLD_ORE, tex::GOLD_ORE), solid: true, transparent: false },
            DiamondOre => BlockDef { name: "Diamond ore", tiles: (tex::DIAMOND_ORE, tex::DIAMOND_ORE, tex::DIAMOND_ORE), solid: true, transparent: false },
            CraftingTable => BlockDef { name: "Crafting table", tiles: (tex::TABLE_TOP, tex::PLANKS, tex::TABLE_SIDE), solid: true, transparent: false },
            Furnace => BlockDef { name: "Furnace", tiles: (tex::FURNACE_TOP, tex::FURNACE_TOP, tex::FURNACE_SIDE), solid: true, transparent: false },
            StoneBricks => BlockDef { name: "Stone bricks", tiles: (tex::STONE_BRICKS, tex::STONE_BRICKS, tex::STONE_BRICKS), solid: true, transparent: false },
        };
        d
    }

    pub fn from_id(id: u8) -> Block {
        use Block::*;
        match id {
            0 => Air,
            1 => Grass,
            2 => Dirt,
            3 => Stone,
            4 => Cobble,
            5 => Sand,
            6 => Log,
            7 => Leaves,
            8 => Planks,
            9 => Water,
            10 => Glass,
            11 => Bedrock,
            12 => CoalOre,
            13 => IronOre,
            14 => Snow,
            15 => GoldOre,
            16 => DiamondOre,
            17 => CraftingTable,
            18 => Furnace,
            19 => StoneBricks,
            _ => Air,
        }
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn is_solid(self) -> bool {
        self.def().solid
    }

    pub fn is_transparent(self) -> bool {
        self.def().transparent
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// Seconds a bare hand needs to break the block in survival mode.
    pub fn hardness(self) -> f32 {
        use Block::*;
        match self {
            Air | Water => 0.0,
            Bedrock => f32::INFINITY,
            Leaves => 0.2,
            Glass => 0.4,
            Grass | Dirt | Sand | Snow => 0.5,
            Log => 1.4,
            Planks | CraftingTable => 1.0,
            Stone | Cobble | StoneBricks => 2.0,
            CoalOre => 2.6,
            IronOre | GoldOre | Furnace => 3.0,
            DiamondOre => 3.4,
        }
    }

    /// What the block leaves behind when broken by hand.
    pub fn drop(self) -> Option<Item> {
        use Block::*;
        match self {
            Air | Water | Bedrock => None,
            Grass => Some(Item::Block(Dirt)),
            Stone => Some(Item::Block(Cobble)),
            CoalOre => Some(Item::Coal),
            IronOre => Some(Item::RawIron),
            GoldOre => Some(Item::RawGold),
            DiamondOre => Some(Item::Diamond),
            Leaves => {
                // Leaves occasionally hide an apple.
                if macroquad::rand::gen_range(0, 100) < 12 {
                    Some(Item::Apple)
                } else {
                    None
                }
            }
            other => Some(Item::Block(other)),
        }
    }

    /// Blocks that open a crafting window when used.
    pub fn is_station(self) -> bool {
        matches!(self, Block::CraftingTable | Block::Furnace)
    }
}

/// Everything the player may place, in creative palette order.
pub const ALL_BLOCKS: [Block; 18] = [
    Block::Grass,
    Block::Dirt,
    Block::Stone,
    Block::Cobble,
    Block::StoneBricks,
    Block::Sand,
    Block::Log,
    Block::Leaves,
    Block::Planks,
    Block::Glass,
    Block::Snow,
    Block::CoalOre,
    Block::IronOre,
    Block::GoldOre,
    Block::DiamondOre,
    Block::CraftingTable,
    Block::Furnace,
    Block::Bedrock,
];

pub const HOTBAR: [Block; 9] = [
    Block::Grass,
    Block::Dirt,
    Block::Stone,
    Block::Cobble,
    Block::Sand,
    Block::Log,
    Block::Leaves,
    Block::Planks,
    Block::Glass,
];
