mod collision_groups {
    pub const COLLISION_CHARACTER: u32 = 1;
    pub const COLLISION_WORLD: u32 = 1 << 1;
    pub const COLLISION_NO_PHYSICS: u32 = 1 << 2;
    pub const COLLISION_ITEM_PICKUP: u32 = 1 << 3;
    pub const COLLISION_PROJECTILES: u32 = 1 << 4;
    // things that the player can point with the cursor
    pub const COLLISION_POINTER: u32 = 1 << 5;
    pub const COLLISION_TREES: u32 = 1 << 6;
    pub const COLLISION_BORDER: u32 = 1 << 7;
}

pub mod camera;
pub mod health;
pub mod inventory;
pub mod item_pickups;
pub mod map;
pub mod notification;
pub mod pickup;
pub mod player;
pub mod pointer;
pub mod projectile;
pub mod shop;
pub mod state;
pub mod tower;
pub mod tree;
pub mod ui_util;
pub mod utils;
pub mod waves;
pub mod weapon;

pub mod animation_linker;
pub mod asset_utils;
pub mod background;
pub mod border_material;
pub mod foliage;
pub mod ground_material;
pub mod knockback;
pub mod tree_spawner;
