use std::ops::Range;

use bevy::{ecs::query::Has, math::vec3, prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::*;
use rand::{thread_rng, Rng};

use crate::{
    collision_groups::{COLLISION_CHARACTER, COLLISION_ITEM_PICKUP, COLLISION_WORLD},
    inventory::{Inventory, Item},
    pickup::{OnPickedUpEvent, PickupTag},
};

const ITEM_LIFETIME: f32 = 20.0;

#[derive(Component)]
pub struct SpawnItemEvery {
    pub range: Range<f32>,
    pub item: Item,
    pub next: f64,
}

#[derive(Event)]
pub struct SpawnItemEvent {
    pub item: Item,
    pub pos: Vec3,
}
#[derive(Resource)]
pub struct ItemModels(HashMap<Item, Vec<Handle<Scene>>>);

#[derive(Component)]
pub struct ItemPickup(Item);

#[derive(Component)]
pub struct DespawnAfter(f32);

#[derive(Component)]
pub struct PickupSound;

pub struct ItemPickupPlugin;

impl Plugin for ItemPickupPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<OnPickedUpEvent>()
            .add_event::<SpawnItemEvent>()
            .add_systems(Startup, setup_item_pickup_resources)
            .add_systems(
                Update,
                (despawn_after, spawn_item_every, spawn_items, perform_pickup),
            );
    }
}

fn despawn_after(
    mut commands: Commands,
    mut despawn: Query<(Entity, &mut DespawnAfter)>,
    time: Res<Time>,
) {
    for (entity, mut despawn) in despawn.iter_mut() {
        despawn.0 -= time.delta_seconds();
        if despawn.0 <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_item_every(
    mut spawn_item: EventWriter<SpawnItemEvent>,
    time: Res<Time>,
    mut spawn_item_every: Query<(&mut SpawnItemEvery, &GlobalTransform)>,
) {
    spawn_item.send_batch(
        spawn_item_every
            .iter_mut()
            .filter_map(|(mut spawn, transform)| {
                if time.elapsed_seconds_f64() >= spawn.next {
                    spawn.next = time.elapsed_seconds_f64()
                        + thread_rng().gen_range(spawn.range.clone()) as f64;
                    Some(SpawnItemEvent {
                        item: spawn.item,
                        pos: transform.translation(),
                    })
                } else {
                    None
                }
            }),
    );
}

fn perform_pickup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut pickup_events: EventReader<OnPickedUpEvent>,
    item_pickups: Query<&ItemPickup>,
    mut receivers: Query<(&mut Inventory, Has<PickupSound>)>,
) {
    for event in pickup_events.read() {
        let Ok(item) = item_pickups.get(event.pickup_entity) else {
            continue;
        };
        let Ok((mut receiver, sound)) = receivers.get_mut(event.receiver_entity) else {
            continue;
        };

        receiver.add_item(item.0, 1);
        if sound {
            commands.spawn(AudioBundle {
                source: asset_server.load("sounds/item_pickup.ogg"),
                settings: PlaybackSettings::DESPAWN,
            });
        }
    }
}

fn spawn_items(
    mut events: EventReader<SpawnItemEvent>,
    mut commands: Commands,
    item_models: Res<ItemModels>,
) {
    let mut rng = rand::thread_rng();
    for event in events.read() {
        let model_handle = item_models.0[&event.item][0].clone();

        let collider_height = 0.4;
        let collider_radius = 0.1;
        let torque = 0.1;
        commands.spawn((
            ItemPickup(event.item),
            PickupTag,
            SceneBundle {
                scene: model_handle,
                transform: Transform::from_translation(event.pos),
                ..default()
            },
            RigidBody::Dynamic,
            Collider::capsule_x(collider_height * 0.5, collider_radius),
            ColliderMassProperties::Mass(1.0),
            Damping {
                linear_damping: 1.2,
                angular_damping: 1.2,
            },
            GravityScale(1.0),
            ActiveEvents::COLLISION_EVENTS,
            // initial velocity up
            ExternalImpulse {
                impulse: vec3(0.0, -2.0, 0.0),
                torque_impulse: vec3(
                    rng.gen_range(-torque..torque),
                    rng.gen_range(-torque..torque),
                    rng.gen_range(-torque..torque),
                ),
            },
            // EXPLANATION: see docs/physics.txt
            CollisionGroups::new(
                Group::from_bits(COLLISION_CHARACTER | COLLISION_WORLD | COLLISION_ITEM_PICKUP)
                    .unwrap(),
                Group::from_bits(COLLISION_CHARACTER | COLLISION_WORLD | COLLISION_ITEM_PICKUP)
                    .unwrap(),
            ),
            DespawnAfter(ITEM_LIFETIME),
        ));
    }
}

fn setup_item_pickup_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(ItemModels(HashMap::from_iter([
        (
            Item::Log,
            vec![asset_server.load("models/items/log_model.gltf#Scene0")],
        ),
        (
            Item::Banana,
            vec![asset_server.load("models/items/banana_model.gltf#Scene0")],
        ),
        (
            Item::Apple,
            vec![asset_server.load("models/items/apple_model.gltf#Scene0")],
        ),
    ])));
}
