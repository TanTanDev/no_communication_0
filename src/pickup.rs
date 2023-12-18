use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{inventory::Item, item_pickups::SpawnItemEvent};

pub const PICKUP_FLY_SPEED: f32 = 10.0;
pub const TIME_TO_FLY: f32 = 0.4;

#[derive(Component)]
pub struct PickupMagnet {
    pub root_entity: Entity,
}

#[derive(Component)]
pub struct PickupTag;

#[derive(Component)]
pub struct FlyToEntity {
    pub entity: Entity,
    pub initial_pos: Vec3,
    pub progress: f32,
}

// the entity reached the "player" event
// listen to perform things like adding stuff to inventory
// BEFORE CoreState::Last, because it despawns then
#[derive(Event)]
pub struct OnPickedUpEvent {
    pub pickup_entity: Entity,   // the pickup entity
    pub receiver_entity: Entity, // who recieves pickup yo
}

pub struct PickupPlugin;

impl Plugin for PickupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, (detect_pickup, fly_to_target))
            .add_systems(Last, destroy_pickups);
    }
}

fn fly_to_target(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut FlyToEntity)>,
    transforms: Query<&GlobalTransform>,
    time: Res<Time>,
    mut pickup_event: EventWriter<OnPickedUpEvent>,
    mut spawn_item_event: EventWriter<SpawnItemEvent>, // in case the target entity dies
) {
    for (pickup_entity, mut transform, mut fly_to_entity) in query.iter_mut() {
        let Ok(target_transform) = transforms.get(fly_to_entity.entity) else {
            commands.entity(pickup_entity).despawn_recursive();
            spawn_item_event.send(SpawnItemEvent {
                item: Item::Log,
                pos: transform.translation,
            });

            continue;
        };
        fly_to_entity.progress += time.delta_seconds();
        let percent = (fly_to_entity.progress / TIME_TO_FLY).clamp(0.0, 1.0);

        let mut lerped = fly_to_entity
            .initial_pos
            .lerp(target_transform.translation(), percent);

        // 0.0  0.5   1.0
        // 0.0  0.5   0.0 REMAPPED
        let jump = percent - (-0.5 + percent * 2.0).max(0.0);
        lerped.y += jump * 3.0;
        transform.translation = lerped;

        if percent >= 1.0 {
            pickup_event.send(OnPickedUpEvent {
                pickup_entity,
                receiver_entity: fly_to_entity.entity,
            });
        }
    }
}

fn destroy_pickups(mut pickup_event: EventReader<OnPickedUpEvent>, mut commands: Commands) {
    for event in pickup_event.read() {
        if let Some(entity_commands) = commands.get_entity(event.pickup_entity) {
            entity_commands.despawn_recursive();
        }
    }
}

fn detect_pickup(
    mut events: EventReader<CollisionEvent>,
    pickup_magnets: Query<&PickupMagnet>,
    pickups: Query<(Entity, &GlobalTransform), With<PickupTag>>,
    mut commands: Commands,
) {
    for event in events.read() {
        let CollisionEvent::Started(e1, e2, _event_flags) = event else {
            continue;
        };

        // order of entity 1 and entity 2 can be swapped
        // sneaky method of testing both paths
        // i cri...
        let (magnet, (pickup_entity, pickup_transform)) = match (
            pickup_magnets.get(*e1),
            pickups.get(*e2),
            pickup_magnets.get(*e2),
            pickups.get(*e1),
        ) {
            (Ok(m), Ok(p), Err(_), Err(_)) => (m, p),
            (Err(_), Err(_), Ok(m), Ok(p)) => (m, p),
            _ => continue,
        };

        commands
            .entity(pickup_entity)
            .insert(FlyToEntity {
                entity: magnet.root_entity,
                initial_pos: pickup_transform.translation(),
                progress: 0.0,
            })
            .remove::<RigidBody>()
            .remove::<Collider>()
            .remove::<PickupTag>();
    }
}
