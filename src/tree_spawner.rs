use std::f32::consts::TAU;

use bevy::{math::vec3, prelude::*};
use bevy_rapier3d::{
    dynamics::RigidBody,
    geometry::ColliderMassProperties,
    prelude::{Collider, CollisionGroups, Group},
};
use bevy_vector_shapes::{painter::ShapePainter, shapes::DiscPainter};
use rand::Rng;

use crate::{
    animation_linker::AnimationEntityLink,
    collision_groups::{COLLISION_CHARACTER, COLLISION_PROJECTILES, COLLISION_WORLD},
    health::Health,
    tree::{SpawnTreeEvent, TreeBlueprint},
};

const TREE_SPAWNER_RANGE: f32 = 10.0;
const TREE_SPAWNER_TIME: f32 = 5.0;
const TREE_SPAWNER_HEALTH: i32 = 13;

pub struct TreeSpawnerPlugin;
impl Plugin for TreeSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnTreeSpawnerEvent>()
            .add_systems(Startup, setup_tower_model)
            .add_systems(Update, (tower_spawn, tower_shoot).chain())
            .add_systems(Update, (start_animation, visualize_range));
    }
}

#[derive(Resource)]
pub struct TreeSpawnerModel((Handle<Scene>, Handle<AnimationClip>));

fn setup_tower_model(mut cmds: Commands, asset_server: Res<AssetServer>) {
    cmds.insert_resource(TreeSpawnerModel((
        asset_server.load("models/buildings/tree_spawner.gltf#Scene0"),
        asset_server.load("models/buildings/tree_spawner.gltf#Animation0"),
    )));
}

#[derive(Component)]
pub struct TreeSpawner {
    timer: Timer,
}

#[derive(Event)]
pub struct SpawnTreeSpawnerEvent {
    pub pos: Vec3,
}

fn start_animation(
    query: Query<&AnimationEntityLink, (Added<AnimationEntityLink>, With<TreeSpawner>)>,
    mut players: Query<&mut AnimationPlayer>,
    tree_model: Res<TreeSpawnerModel>,
) {
    for link in query.iter() {
        let Ok(mut player) = players.get_mut(link.0) else {
            continue;
        };
        player.play(tree_model.0 .1.clone()).repeat();
    }
}

fn tower_spawn(
    mut cmds: Commands,
    tower_model: Res<TreeSpawnerModel>,
    mut ev_spawn_tower: EventReader<SpawnTreeSpawnerEvent>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_spawn_tower.read() {
        cmds.spawn(AudioBundle {
            source: asset_server.load("sounds/build.ogg"),
            settings: PlaybackSettings::DESPAWN,
        });
        cmds.spawn((
            Name::new("Tower"),
            TreeSpawner {
                timer: Timer::from_seconds(TREE_SPAWNER_TIME, TimerMode::Repeating),
            },
            Health::new(TREE_SPAWNER_HEALTH),
            SceneBundle {
                scene: tower_model.0 .0.clone_weak(),
                transform: Transform::from_translation(vec3(ev.pos.x + 1.0, 0.0, ev.pos.z)),
                ..default()
            },
            RigidBody::Fixed,
            Collider::capsule(Vec3::ZERO, Vec3::Y, 0.5),
            ColliderMassProperties::Mass(1.0),
            // EXPLANATION: see docs/physics.txt
            CollisionGroups::new(
                Group::from_bits(COLLISION_CHARACTER).unwrap(),
                Group::from_bits(COLLISION_CHARACTER | COLLISION_WORLD | COLLISION_PROJECTILES)
                    .unwrap(),
            ),
        ));
        // .with_children(|cmds| {
        //     cmds.spawn((
        //         SpatialBundle::from_transform(Transform::from_xyz(0.0, -2.5, 0.0)),
        //         Collider::cuboid(1.0, 2.5, 1.0),
        //         CollisionGroups::new(
        //             Group::from_bits(COLLISION_WORLD).unwrap(),
        //             Group::from_bits(COLLISION_CHARACTER).unwrap(),
        //         ),
        //     ));
        // });
    }
}

fn visualize_range(mut painter: ShapePainter, query: Query<(&TreeSpawner, &Transform)>) {
    for (_, transform) in query.iter() {
        painter.color = Color::YELLOW;
        painter.thickness = 0.05;
        painter.hollow = true;
        painter.set_rotation(Quat::from_rotation_x(TAU / 4.0));
        painter.set_translation(vec3(transform.translation.x, 0.0, transform.translation.z));
        painter.circle(TREE_SPAWNER_RANGE);
    }
}

fn tower_shoot(
    mut query: Query<(&mut TreeSpawner, &Transform)>,
    time: Res<Time>,
    mut spawn: EventWriter<SpawnTreeEvent>,
) {
    for (mut tower, transform) in query.iter_mut() {
        if !tower.timer.tick(time.delta()).just_finished() {
            continue;
        }
        let mut rng = rand::thread_rng();
        let dist = rng.gen_range(1.0..TREE_SPAWNER_RANGE);
        let rot = Quat::from_rotation_y(rng.gen_range(0.0..TAU));
        let pos = transform.translation + rot * (Vec3::Z * dist);
        spawn.send(SpawnTreeEvent {
            pos,
            blueprint: TreeBlueprint::Randomized,
            play_sound: true,
        });
    }
}
