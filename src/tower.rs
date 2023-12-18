use std::f32::consts::TAU;

use bevy::{
    math::{vec3, Vec3Swizzles},
    prelude::*,
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group};
use bevy_vector_shapes::{
    prelude::ShapePainter,
    shapes::{DiscPainter, LinePainter},
};

use crate::{
    collision_groups::{COLLISION_CHARACTER, COLLISION_WORLD},
    player::RobotTag,
    weapon::{TryCastWeaponEvent, WeaponCooldown, WeaponStats, WeaponType},
};

const TOWER_RANGE: f32 = 8.0;

pub struct TowerPlugin;
impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnTowerEvent>()
            .add_systems(Startup, setup_tower_model)
            .add_systems(Update, (tower_spawn, tower_target, tower_shoot).chain());
    }
}

#[derive(Resource)]
pub struct TowerModel(Handle<Scene>);

fn setup_tower_model(mut cmds: Commands, asset_server: Res<AssetServer>) {
    cmds.insert_resource(TowerModel(
        asset_server.load("models/buildings/tower.glb#Scene0"),
    ));
}

#[derive(Component)]
pub struct TowerTag;

#[derive(Component)]
pub struct TowerTarget(Entity);

#[derive(Event)]
pub struct SpawnTowerEvent {
    pub pos: Vec3,
}

fn tower_spawn(
    mut cmds: Commands,
    tower_model: Res<TowerModel>,
    mut ev_spawn_tower: EventReader<SpawnTowerEvent>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_spawn_tower.read() {
        cmds.spawn(AudioBundle {
            source: asset_server.load("sounds/build.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
        cmds.spawn((
            Name::new("Tower"),
            TowerTag,
            TowerTarget(Entity::PLACEHOLDER),
            WeaponType::Bow(asset_server.load("projectiles/tower.projectile.ron")),
            WeaponCooldown { time_left: 2.0 },
            WeaponStats::default(),
            SceneBundle {
                scene: tower_model.0.clone_weak(),
                transform: Transform::from_translation(vec3(ev.pos.x, 5.0, ev.pos.z)),
                ..default()
            },
        ))
        .with_children(|cmds| {
            cmds.spawn((
                SpatialBundle::from_transform(Transform::from_xyz(0.0, -2.5, 0.0)),
                Collider::cuboid(1.0, 2.5, 1.0),
                CollisionGroups::new(
                    Group::from_bits(COLLISION_WORLD).unwrap(),
                    Group::from_bits(COLLISION_CHARACTER).unwrap(),
                ),
            ));
        });
    }
}

fn tower_target(
    mut painter: ShapePainter,
    mut q_tower: Query<(&mut TowerTarget, &Transform)>,
    q_enemies: Query<(Entity, &Transform), With<RobotTag>>,
) {
    for (mut target, tower_tr) in &mut q_tower {
        // get current targeted enemy distance
        let mut curr_target_distance = q_enemies
            .get(target.0)
            .map(|(_, tr)| (tr.translation.xz() - tower_tr.translation.xz()).length())
            .unwrap_or(10000.0);

        // switch to any closer enemy
        for (enemy_entity, enemy_tr) in &q_enemies {
            let distance = (enemy_tr.translation.xz() - tower_tr.translation.xz()).length();
            if distance < curr_target_distance {
                target.0 = enemy_entity;
                curr_target_distance = distance;
            }
        }

        if curr_target_distance > TOWER_RANGE {
            target.0 = Entity::PLACEHOLDER;
        }

        painter.color = Color::GREEN;
        painter.thickness = 0.03;
        painter.hollow = true;
        painter.set_rotation(Quat::from_rotation_x(TAU / 4.0));
        painter.set_translation(vec3(tower_tr.translation.x, 0.0, tower_tr.translation.z));
        painter.circle(TOWER_RANGE);

        // highlight targeted enemy
        if let Ok((_, target_pos)) = q_enemies.get(target.0) {
            painter.color = Color::RED;
            painter.thickness = 0.01;
            painter.hollow = true;
            painter.set_rotation(Quat::from_rotation_x(TAU / 4.0));
            painter.set_translation(target_pos.translation);
            painter.circle(1.0);

            painter.set_translation(Vec3::ZERO);
            painter.set_rotation(Quat::default());
            painter.line(tower_tr.translation, target_pos.translation);
        }
    }
}

fn tower_shoot(
    q_tower: Query<(Entity, &TowerTarget, &Transform)>,
    q_enemies: Query<&Transform>,
    mut ev_try_cast: EventWriter<TryCastWeaponEvent>,
) {
    for (tower_e, tower_target, tower_tr) in &q_tower {
        if let Ok(target_tr) = q_enemies.get(tower_target.0) {
            let dir = (target_tr.translation - tower_tr.translation).normalize();
            ev_try_cast.send(TryCastWeaponEvent {
                caster_entity: tower_e,
                target_entity: Some(tower_target.0),
                dir,
            });
        }
    }
}
