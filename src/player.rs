use std::cmp::Ordering;

use bevy::{math::vec3, prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::*;

use crate::{
    camera::MainCameraTag,
    collision_groups::{
        COLLISION_BORDER, COLLISION_CHARACTER, COLLISION_ITEM_PICKUP, COLLISION_POINTER,
        COLLISION_PROJECTILES, COLLISION_WORLD,
    },
    health::{DeathSound, Health, ShowHealthBar},
    inventory::Inventory,
    item_pickups::PickupSound,
    pickup::PickupMagnet,
    pointer::PointerPos,
    tree::TreeTrunkTag,
    utils::movement_axis,
    weapon::{TryCastWeaponEvent, WeaponCooldown, WeaponStats, WeaponType},
};

pub const PLAYER_HEALTH: i32 = 10;
pub const PLAYER_PICKUP_RADIUS: f32 = 3.0;

#[derive(Component)]
pub struct Player {
    pub movement_speed: f32,
    // how fast player visually rotates
    pub rotation_speed: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum Body {
    Monkey,
    Robot,
}

#[derive(Event)]
pub struct SpawnPlayerEvent {
    pub pos: Vec3,
    pub is_main: bool,
    pub body: Body,
    pub weapon_type: WeaponType,
}

#[derive(Component)]
pub struct RobotController {
    target: Option<Entity>,
    attack_monkey_range: f32,
    /// Keeps track of where we were at certain intervals, to determine if we're stuck or not.
    last_position_check: Option<(f64, Vec3)>,
}

#[derive(Component)]
pub struct PlayerControllerTag;

/// 🐒 🙈🙉🙊 🐵 🦍🍌
#[derive(Component)]
pub struct MonkeyTag;

/// 🪓🪓🤖 ⚡ ⚙
#[derive(Component)]
pub struct RobotTag;

// input controller + ai can set these values to controll the wanted actions
// see playercontrollerTag and dumpplayercontroller
#[derive(Component, Default)]
pub struct PlayerInput {
    pub movement: Vec3,
    pub jump: bool,
    pub attack: Option<(Vec3, Option<Entity>)>,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnPlayerEvent>()
            .add_systems(Startup, load_character_models)
            .add_systems(Update, spawn_players)
            .add_systems(
                Update,
                (
                    (movement_input, attack_input, robot_ai),
                    (apply_movement, apply_attack),
                )
                    .chain(),
            );
    }
}

fn robot_ai(
    mut robots: Query<(
        &mut PlayerInput,
        &mut RobotController,
        &Player,
        &GlobalTransform,
    )>,
    monkeys: Query<(Entity, &GlobalTransform), With<MonkeyTag>>,
    trees: Query<(Entity, &GlobalTransform), With<TreeTrunkTag>>,
    transforms: Query<&GlobalTransform>,
    time: Res<Time>,
) {
    for (mut player_input, mut controller, player, transform) in robots.iter_mut() {
        let dist_map = |(e, t): (Entity, &GlobalTransform)| {
            (
                t.translation().distance_squared(transform.translation()),
                e,
                *t,
            )
        };
        let float_cmp =
            |a: &(f32, _, _), b: &(f32, _, _)| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Greater);

        player_input.attack = None;
        if let Some((t, p)) = controller.last_position_check {
            let check_interval = 0.1;
            let min_move_distance = check_interval as f32 * player.movement_speed / 5.0;
            if (time.elapsed_seconds_f64() - t) >= check_interval {
                if p.distance_squared(transform.translation()) <= min_move_distance.powi(2)
                    && player_input.movement.length_squared() > 0.0
                {
                    player_input.attack = Some((player_input.movement, None));
                }
                controller.last_position_check =
                    Some((time.elapsed_seconds_f64(), transform.translation()));
            }
        } else {
            controller.last_position_check =
                Some((time.elapsed_seconds_f64(), transform.translation()));
        }

        let mut attack_target = |target: &GlobalTransform| {
            let attack_distance: f32 = 2.0;
            let mut diff = target.translation() - transform.translation();
            if transform
                .translation()
                .distance_squared(target.translation())
                < attack_distance.powi(2)
            {
                player_input.attack = Some((diff, None));
            } else {
                diff.y = 0.0;
                player_input.movement = diff;
            }
        };

        // If we have a monkey as a target, follow and attack that
        if let Some((_, target)) = controller.target.and_then(|e| monkeys.get(e).ok()) {
            attack_target(target);
        }
        // Otherwise check if we are close enough to the closest monkey, if so target it
        else if let Some((_, monkey_entity, _)) = monkeys
            .iter()
            .map(dist_map)
            .filter(|(t, _, _)| *t < controller.attack_monkey_range.powi(2))
            .min_by(float_cmp)
        {
            controller.target = Some(monkey_entity);
        }
        // If we don't have any monkeys to target attack choose the non-monkey target if we have one
        else if let Some(target) = controller.target.and_then(|e| transforms.get(e).ok()) {
            attack_target(target);
        }
        // If we don't have a target, find the closest tree to target.
        else if let Some((_, tree_entity, _)) = trees.iter().map(dist_map).min_by(float_cmp) {
            controller.target = Some(tree_entity);
        } else {
            controller.target = None;
        }
    }
}

pub fn attack_input(
    mouse: Res<Input<MouseButton>>,
    mut query: Query<(Entity, &mut PlayerInput, &GlobalTransform), With<PlayerControllerTag>>,
    pointer: Res<PointerPos>,
) {
    let Ok((player_entity, mut player_input, transform)) = query.get_single_mut() else {
        return;
    };
    player_input.attack = None;
    if mouse.pressed(MouseButton::Left) {
        // don't attack self
        if Some(player_entity) == pointer.pointer_on.map(|p| p.entity) {
            return;
        }
        player_input.attack = pointer
            .pointer_on
            .map(|p| (p.wpos - transform.translation(), Some(p.entity)));
    }
}

fn movement_input(
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut PlayerInput, With<PlayerControllerTag>>,
    cameras: Query<&Transform, With<MainCameraTag>>,
) {
    let camera_transform = cameras.single();

    let forward = camera_transform.right();
    let rotation = Quat::from_axis_angle(Vec3::Y, forward.y);

    for mut player_input in query.iter_mut() {
        let x = movement_axis(&input, KeyCode::D, KeyCode::A);
        let z = movement_axis(&input, KeyCode::S, KeyCode::W);
        let dir = vec3(x, 0.0, z).normalize_or_zero();
        let dir = rotation * dir;
        player_input.movement = dir;
    }
}

fn apply_attack(
    query: Query<(&PlayerInput, Entity)>,
    mut attack_events: EventWriter<TryCastWeaponEvent>,
) {
    for (input, entity) in query.iter() {
        if let Some((dir, target)) = input.attack {
            attack_events.send(TryCastWeaponEvent {
                caster_entity: entity,
                target_entity: target,
                dir,
            });
        }
    }
}

fn apply_movement(
    mut query: Query<(&PlayerInput, &mut Transform, &Player, &mut Velocity)>,
    time: Res<Time>,
) {
    for (input, mut transform, player, mut velocity) in query.iter_mut() {
        let normalized_input = input.movement.normalize_or_zero();
        let desired_velocity = normalized_input * player.movement_speed;
        let true_velocity = velocity.linvel;

        velocity.linvel = Vec3::lerp(true_velocity, desired_velocity, time.delta_seconds() * 10.0);
        let desired_quat =
            Quat::from_rotation_y(f32::atan2(normalized_input.x, normalized_input.z));

        // rotate to where we are heading
        if normalized_input.length() > 0.1 {
            transform.rotation = Quat::lerp(
                transform.rotation,
                desired_quat,
                time.delta_seconds() * player.rotation_speed,
            );
        }
    }
}

#[derive(Resource)]
struct CharacterModels(HashMap<Body, Handle<Scene>>);

fn load_character_models(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(CharacterModels(HashMap::from_iter([
        (
            Body::Monkey,
            asset_server.load("models/characters/monkey.gltf#Scene0"),
        ),
        (
            Body::Robot,
            asset_server.load("models/characters/robot.gltf#Scene0"),
        ),
    ])));
}

fn spawn_players(
    mut commands: Commands,
    mut events: EventReader<SpawnPlayerEvent>,
    character_models: Res<CharacterModels>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        let speed = match event.body {
            Body::Monkey => 20.0,
            Body::Robot => 10.0,
        };
        let collision_groups = match event.body {
            Body::Monkey => {
                // EXPLANATION: see docs/physics.txt
                CollisionGroups::new(
                    Group::from_bits(COLLISION_CHARACTER).unwrap(),
                    Group::from_bits(
                        COLLISION_CHARACTER
                            | COLLISION_WORLD
                            | COLLISION_PROJECTILES
                            | COLLISION_BORDER,
                    )
                    .unwrap(),
                )
            }
            Body::Robot => {
                // EXPLANATION: see docs/physics.txt
                CollisionGroups::new(
                    Group::from_bits(COLLISION_CHARACTER).unwrap(),
                    Group::from_bits(COLLISION_CHARACTER | COLLISION_WORLD | COLLISION_PROJECTILES)
                        .unwrap(),
                )
            }
        };
        let player_root = commands
            .spawn((
                event.body,
                (
                    RigidBody::Dynamic,
                    Collider::capsule(Vec3::ZERO, Vec3::Y, 0.5),
                    TransformBundle::from(Transform::from_translation(event.pos)),
                    Velocity::default(),
                    ColliderMassProperties::Mass(1.0),
                    ExternalForce {
                        force: Vec3::ZERO,
                        torque: Vec3::ZERO,
                    },
                    GravityScale(1.0),
                    LockedAxes::ROTATION_LOCKED_X
                        | LockedAxes::ROTATION_LOCKED_Z
                        | LockedAxes::ROTATION_LOCKED_Y,
                    Sleeping::disabled(),
                    Ccd::enabled(),
                    // other
                    Player {
                        movement_speed: speed,
                        rotation_speed: 15.0,
                    },
                    PlayerInput::default(),
                    event.weapon_type.clone(),
                    WeaponCooldown { time_left: 0.0 },
                    Health::new(PLAYER_HEALTH),
                ),
                (
                    ShowHealthBar,
                    WeaponStats::default(),
                    ExternalImpulse::default(),
                    VisibilityBundle::default(),
                    collision_groups,
                    Inventory::default(),
                ),
            ))
            .id();

        let pickup_collider = commands
            .spawn((
                PickupMagnet {
                    root_entity: player_root,
                },
                Sensor,
                ActiveEvents::COLLISION_EVENTS,
                Collider::ball(PLAYER_PICKUP_RADIUS),
                CollisionGroups::new(
                    Group::all(),
                    Group::from_bits(COLLISION_ITEM_PICKUP).unwrap(), // collides with item_pickups(3) only
                ),
                ColliderMassProperties::Mass(0.0), // without this it breaks the anti gravity
            ))
            .id();

        commands.entity(pickup_collider).set_parent(player_root);

        let scene = character_models.0[&event.body].clone();
        let graphics = commands
            .spawn(SceneBundle {
                scene,
                transform: Transform::from_translation(vec3(0.0, 0.5, 0.0)),
                ..default()
            })
            .id();
        commands.entity(graphics).set_parent(player_root);
        if event.is_main {
            commands.entity(player_root).insert((
                PlayerControllerTag,
                MonkeyTag,
                PickupSound,
                Name::new("player"),
            ));
            info!("spawning plaer",);
        } else {
            commands
                .entity(player_root)
                .insert((
                    Name::new("enemy"),
                    RobotTag,
                    RobotController {
                        target: None,
                        attack_monkey_range: 5.0,
                        last_position_check: None,
                    },
                    DeathSound(asset_server.load("sounds/robot-death.ogg")),
                ))
                .with_children(|cmds| {
                    cmds.spawn((
                        SpatialBundle::INHERITED_IDENTITY,
                        Collider::cylinder(0.5, 2.0),
                        CollisionGroups::new(
                            Group::from_bits(COLLISION_POINTER).unwrap(),
                            Group::from_bits(COLLISION_POINTER).unwrap(),
                        ),
                    ));
                });
        }
    }
}
