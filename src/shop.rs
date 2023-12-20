use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    health::ApplyHealthEvent,
    inventory::{Inventory, Item},
    player::PlayerControllerTag,
    tower::SpawnTowerEvent,
    tree::{SpawnTreeEvent, TreeBlueprint},
    tree_spawner::SpawnTreeSpawnerEvent,
    ui_util::{ButtonColor, JustClicked, UiAssets},
    weapon::WeaponStats,
};

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnShopItemEvent>()
            .add_event::<BuyEvent>()
            .add_systems(Startup, setup_shop_ui)
            .add_systems(
                Update,
                (spawn_shop_items, handle_shop_item_click, buy_items),
            );
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum ShopItemEffect {
    PlantTree,
    IncreaseDamage(i32),
    MultiplyCooldown(f32),
    Heal(i32),
    BuildTower,
    BuildTreeSpawner,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShopItemData {
    pub cost: Vec<(Item, u32)>,
    pub effects: Vec<ShopItemEffect>,
    #[serde(default)]
    pub permanent: bool,
}

impl ShopItemData {
    pub fn name(&self) -> String {
        self.effects
            .iter()
            .map(|e| match e {
                ShopItemEffect::PlantTree => String::from("Plant tree"),
                ShopItemEffect::IncreaseDamage(d) => format!("Increase damage (+{d})"),
                ShopItemEffect::MultiplyCooldown(d) => format!("Decrease cooldown (x{d})"),
                ShopItemEffect::Heal(h) => format!("Heal (+{h})"),
                ShopItemEffect::BuildTower => String::from("Build defense tower"),
                ShopItemEffect::BuildTreeSpawner => String::from("Build tree spawner"),
            })
            .map(|s| format!("> {s}\n"))
            .collect()
    }

    pub fn color(&self) -> Color {
        match self.effects[0] {
            ShopItemEffect::BuildTower => Color::GOLD,
            ShopItemEffect::Heal(_) => Color::RED,
            ShopItemEffect::IncreaseDamage(_) => Color::PURPLE,
            ShopItemEffect::MultiplyCooldown(_) => Color::PURPLE,
            ShopItemEffect::PlantTree => Color::BEIGE,
            ShopItemEffect::BuildTreeSpawner => Color::TEAL,
        }
        .with_a(0.5)
    }
}

#[derive(Component)]
struct ShopUiTag;

#[derive(Event)]
pub struct SpawnShopItemEvent {
    pub item: ShopItemData,
}

// The data should not be mutated, since then ui would also need to be updated.
#[derive(Component)]
struct ShopItem(ShopItemData);

#[derive(Event)]
pub struct BuyEvent {
    pub buyer: Entity,
    pub item: Entity,
}

fn setup_shop_ui(mut commands: Commands) {
    commands.spawn((
        ShopUiTag,
        NodeBundle {
            style: Style {
                grid_auto_rows: vec![GridTrack::max_content()],
                grid_template_columns: vec![GridTrack::max_content()],
                column_gap: Val::Px(5.0),
                row_gap: Val::Px(5.0),
                position_type: PositionType::Absolute,
                height: Val::Percent(1.0),
                width: Val::Percent(1.0),
                right: Val::Percent(0.0),
                justify_content: JustifyContent::End,
                justify_items: JustifyItems::End,
                padding: UiRect::all(Val::Px(10.0)),
                display: Display::Grid,
                ..default()
            },
            ..default()
        },
    ));
}

fn spawn_shop_items(
    mut commands: Commands,
    mut shop_items: EventReader<SpawnShopItemEvent>,
    shop_node: Query<Entity, With<ShopUiTag>>,
    ui_assets: Res<UiAssets>,
) {
    let shop_node = shop_node.single();

    for ev in shop_items.read() {
        commands
            .spawn((
                ShopItem(ev.item.clone()),
                ButtonColor(ev.item.color()),
                ButtonBundle {
                    style: Style {
                        min_width: Val::Px(50.0),
                        min_height: Val::Px(50.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        padding: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(ev.item.color()),
                    border_color: Color::BLACK.into(),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    &ev.item.name(),
                    TextStyle {
                        font: ui_assets.font.clone(),
                        font_size: 21.0,
                        color: Color::BLACK,
                    },
                ));

                parent.spawn(TextBundle::from_sections(ev.item.cost.iter().map(
                    |(item, amount)| {
                        TextSection::new(
                            format!("{amount}x {item}"),
                            TextStyle {
                                font: ui_assets.font.clone(),
                                font_size: 14.0,
                                color: Color::BLACK,
                            },
                        )
                    },
                )));
            })
            .set_parent(shop_node);
    }
}

fn handle_shop_item_click(
    mut buy_event: EventWriter<BuyEvent>,
    shop_buttons: Query<Entity, (With<ShopItem>, With<JustClicked>)>,
    player: Query<Entity, With<PlayerControllerTag>>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };

    buy_event.send_batch(shop_buttons.iter().map(|e| BuyEvent {
        buyer: player,
        item: e,
    }));
}

fn buy_items(
    mut commands: Commands,
    mut buy_event: EventReader<BuyEvent>,
    shop_item: Query<&ShopItem>,
    mut spawn_tree_event: EventWriter<SpawnTreeEvent>,
    mut spawn_tower_event: EventWriter<SpawnTowerEvent>,
    mut weapon: Query<&mut WeaponStats>,
    mut inventory: Query<&mut Inventory>,
    mut apply_health_event: EventWriter<ApplyHealthEvent>,
    mut tree_spawner: EventWriter<SpawnTreeSpawnerEvent>,
    transform: Query<&GlobalTransform>,
) {
    let mut apply_effect = |effect: &ShopItemEffect, buyer: Entity| match effect {
        ShopItemEffect::PlantTree => {
            if let Ok(transform) = transform.get(buyer) {
                let mut pos = transform.translation();
                pos.y = 0.0;
                spawn_tree_event.send(SpawnTreeEvent {
                    pos,
                    blueprint: TreeBlueprint::Randomized,
                    play_sound: true,
                });
            }
        }
        ShopItemEffect::IncreaseDamage(amount) => {
            if let Ok(mut weapon) = weapon.get_mut(buyer) {
                weapon.damage_add += amount;
            }
        }
        ShopItemEffect::MultiplyCooldown(amount) => {
            if let Ok(mut weapon) = weapon.get_mut(buyer) {
                weapon.cooldown_mul *= amount;
            }
        }
        ShopItemEffect::Heal(amount) => apply_health_event.send(ApplyHealthEvent {
            amount: *amount,
            target_entity: buyer,
            caster_entity: buyer,
        }),
        ShopItemEffect::BuildTower => {
            if let Ok(transform) = transform.get(buyer) {
                let mut pos = transform.translation();
                pos.y = 0.0;
                spawn_tower_event.send(SpawnTowerEvent { pos });
            }
        }
        ShopItemEffect::BuildTreeSpawner => {
            if let Ok(transform) = transform.get(buyer) {
                let mut pos = transform.translation();
                pos.y = 0.0;
                tree_spawner.send(SpawnTreeSpawnerEvent { pos });
            }
        }
    };

    for event in buy_event.read() {
        if let (Some(e), Ok(shop_item)) =
            (commands.get_entity(event.item), shop_item.get(event.item))
        {
            if inventory
                .get_mut(event.buyer)
                .map_or(false, |mut inventory| {
                    inventory.spend_items(shop_item.0.cost.iter().copied())
                })
            {
                if !shop_item.0.permanent {
                    e.despawn_recursive();
                }

                shop_item
                    .0
                    .effects
                    .iter()
                    .for_each(|e| apply_effect(e, event.buyer));
            }
        }
    }
}
