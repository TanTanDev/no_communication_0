use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::{player::PlayerControllerTag, ui_util::UiAssets};

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Item>()
            .register_type::<Inventory>()
            .add_systems(Startup, setup_inventory_ui)
            .add_systems(Update, update_inventory_ui);
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Reflect, Deserialize)]
pub enum Item {
    Log,
    Banana,
    Apple,
}

#[derive(Component, Default, Reflect)]
pub struct Inventory {
    items: HashMap<Item, u32>,
}

impl Inventory {
    pub fn add_item(&mut self, item: Item, count: u32) {
        *self.items.entry(item).or_insert(0) += count;
    }

    /// Spends `count` material, returning whether it was successful or not.
    pub fn spend_item(&mut self, item: Item, count: u32) -> bool {
        let mut is_zero = false;
        let res = self.items.get_mut(&item).is_some_and(|c| {
            let Some(new_count) = c.checked_sub(count) else {
                return false;
            };
            *c = new_count;
            is_zero = new_count == 0;
            true
        });

        if is_zero {
            self.items.remove(&item);
        }

        res
    }

    pub fn spend_items(&mut self, items: impl Iterator<Item = (Item, u32)> + Clone) -> bool {
        if items
            .clone()
            .all(|(item, c)| self.items.get(&item).map_or(false, |count| *count >= c))
        {
            for (item, count) in items {
                let c = self
                    .items
                    .get_mut(&item)
                    .expect("We already checked that this is in the map.");
                *c -= count;
                if *c == 0 {
                    self.items.remove(&item);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn get_item_count(&self, item: Item) -> u32 {
        self.items.get(&item).copied().unwrap_or(0)
    }
}

#[derive(Component)]
struct ItemText(Item);

fn setup_inventory_ui(mut commands: Commands, ui_assets: Res<UiAssets>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                height: Val::Percent(1.0),
                width: Val::Percent(1.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for material in Item::iter() {
                parent.spawn((
                    ItemText(material),
                    TextBundle::from_section(
                        format!("{}: 0", material),
                        TextStyle {
                            font: ui_assets.font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_style(Style {
                        display: Display::None,
                        ..default()
                    }),
                ));
            }
        });
}

fn update_inventory_ui(
    player: Query<&Inventory, (With<PlayerControllerTag>, Changed<Inventory>)>,
    mut material_text: Query<(&mut Text, &mut Style, &ItemText)>,
) {
    let Ok(inventory) = player.get_single() else {
        // Inventory hasn't changed, return.
        return;
    };

    for (mut text, mut style, material) in material_text.iter_mut() {
        let count = inventory.get_item_count(material.0);
        if count > 0 {
            style.display = Display::Flex;
            text.sections[0].value = format!("{}: {}", material.0, count);
        } else {
            style.display = Display::None;
        }
    }
}
