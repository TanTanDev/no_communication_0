use std::cmp::Ordering;

use bevy::{math::vec3, prelude::*, utils::HashMap};
use bevy_rapier3d::prelude::*;
use rand::{thread_rng, Rng};

use crate::{
    animation_linker::{AnimationEntityLink, AnimationEntityLinkTrap},
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
    tree_spawner::TreeSpawner,
    utils::movement_axis,
    weapon::{TryCastWeaponEvent, WeaponCooldown, WeaponStats, WeaponType},
};

pub const PLAYER_HEALTH: i32 = 20;
pub const ROBOT_HEALTH: i32 = 10;
pub const BOSS_HEALTH: i32 = 100;
pub const FAST_ROBOT_HEALTH: i32 = 6;
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
    FastRobot,
    Boss,
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
            .add_systems(Update, animate_farmer)
            .add_systems(Update, (input, update_farmer_animation).chain())
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
    tree_spawners: Query<(Entity, &GlobalTransform), With<TreeSpawner>>,
    transforms: Query<&GlobalTransform>,
    entity_query: Query<Entity, With<Health>>,
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

        if let Some(target) = controller.target {
            if entity_query.get(target).is_ok() {
                continue;
            } else {
                controller.target = None;
            }
        }
        let closest_tree = trees.iter().map(dist_map).min_by(float_cmp);
        let closest_spawner = tree_spawners.iter().map(dist_map).min_by(float_cmp);
        // 5 % chance to attack spawner
        let target = match thread_rng().gen_range(0.0..1.0) < 0.1 {
            true => match closest_spawner {
                Some(c) => Some(c.1),
                None => closest_tree.map(|t| t.1),
            },
            false => match closest_tree {
                Some(c) => Some(c.1),
                None => closest_spawner.map(|t| t.1),
            },
        };
        if let Some(target) = target {
            controller.target = Some(target);
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

fn animate_farmer(
    // input: Res<Input<KeyCode>>,
    mut query: Query<(&mut PlayerInput, &mut FarmerAnimator), With<PlayerControllerTag>>,
) {
    for (player_input, mut animator) in query.iter_mut() {
        if player_input.movement.length() > 0.0 {
            animator.play(FarmerAnimation::Run);
        } else {
            animator.play(FarmerAnimation::Idle);
        }
        if player_input.attack.is_some() {
            animator.play(FarmerAnimation::Attack);
        }
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
    mut query: Query<(
        &PlayerInput,
        &mut Transform,
        &Player,
        &mut Velocity,
        Option<&MonkeyTag>,
    )>,
    time: Res<Time>,
    pointer: Res<PointerPos>,
) {
    for (input, mut transform, player, mut velocity, monkey_tag) in query.iter_mut() {
        let normalized_input = input.movement.normalize_or_zero();
        let desired_velocity = normalized_input * player.movement_speed;
        let true_velocity = velocity.linvel;

        velocity.linvel = Vec3::lerp(true_velocity, desired_velocity, time.delta_seconds() * 10.0);
        let mut desired_quat =
            Quat::from_rotation_y(f32::atan2(normalized_input.x, normalized_input.z));

        // rotate to where we are heading
        if monkey_tag.is_some() {
            if let Some(pointer_on) = pointer.pointer_on {
                let target = pointer_on.wpos;
                let target = Vec3::new(target.x, 0.0, target.z) - transform.translation;
                desired_quat = Quat::from_rotation_y(f32::atan2(target.x, target.z));
            }
        } else if normalized_input.length() > 0.1 {
            transform.rotation = Quat::lerp(
                transform.rotation,
                desired_quat,
                time.delta_seconds() * player.rotation_speed,
            );
        }
        transform.rotation = Quat::lerp(
            transform.rotation,
            desired_quat,
            time.delta_seconds() * player.rotation_speed,
        );
    }
}

#[derive(Resource)]
struct CharacterModels(HashMap<Body, Handle<Scene>>);

#[derive(Resource)]
pub struct FarmerAnimations {
    idle: Handle<AnimationClip>,
    run: Handle<AnimationClip>,
    attack: Handle<AnimationClip>,
    idle_model: Handle<Scene>,
    run_model: Handle<Scene>,
    attack_model: Handle<Scene>,
}
#[derive(Component)]
pub struct FarmerAnimator {
    idle: (Entity, Handle<AnimationClip>),
    run: (Entity, Handle<AnimationClip>),
    attack: (Entity, Handle<AnimationClip>),
    next_anim: Option<(Entity, Handle<AnimationClip>)>,
}

impl FarmerAnimator {
    pub fn play(&mut self, anim: FarmerAnimation) {
        match anim {
            FarmerAnimation::Idle => self.next_anim = Some(self.idle.clone()),
            FarmerAnimation::Run => self.next_anim = Some(self.run.clone()),
            FarmerAnimation::Attack => self.next_anim = Some(self.attack.clone()),
        };
    }

    pub fn model_entities(&self) -> [Entity; 3] {
        [self.idle.0, self.run.0, self.attack.0]
    }
}

pub enum FarmerAnimation {
    Idle,
    Run,
    Attack,
}

fn input(input: Res<Input<KeyCode>>, mut farmer_animator: Query<&mut FarmerAnimator>) {
    let Ok(mut farmer_animator) = farmer_animator.get_single_mut() else {
        return;
    };
    if input.just_pressed(KeyCode::R) {
        farmer_animator.play(FarmerAnimation::Idle);
    }
    if input.just_pressed(KeyCode::T) {
        farmer_animator.play(FarmerAnimation::Run);
    }
    if input.just_pressed(KeyCode::Y) {
        farmer_animator.play(FarmerAnimation::Attack);
    }
}

fn update_farmer_animation(
    mut farmer_animator: Query<&mut FarmerAnimator>,
    mut root_players: Query<(&AnimationEntityLink, &mut Visibility)>,
    mut animation_players: Query<&mut AnimationPlayer>,
) {
    let Ok(mut farmer_animator) = farmer_animator.get_single_mut() else {
        return;
    };
    let Some(next_anim) = farmer_animator.next_anim.take() else {
        return;
    };
    let Ok((animation_link, mut visibility)) = root_players.get_mut(next_anim.0) else {
        return;
    };
    *visibility = Visibility::Inherited;
    animation_players
        .get_mut(animation_link.0)
        .unwrap()
        .play(next_anim.1.clone())
        .repeat();

    // hide others
    for entity in farmer_animator.model_entities().iter() {
        // skip the one we are showing
        if entity == &next_anim.0 {
            continue;
        }
        let Ok((_animation_link, mut visibility)) = root_players.get_mut(*entity) else {
            continue;
        };
        *visibility = Visibility::Hidden;
    }
}

fn load_character_models(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(FarmerAnimations {
        idle_model: asset_server.load("models/characters/farmer_idle.gltf#Scene0"),
        run_model: asset_server.load("models/characters/farmer_run.gltf#Scene0"),
        attack_model: asset_server.load("models/characters/farmer_attack.gltf#Scene0"),
        idle: asset_server.load("models/characters/farmer_idle.gltf#Animation0"),
        run: asset_server.load("models/characters/farmer_run.gltf#Animation0"),
        attack: asset_server.load("models/characters/farmer_attack.gltf#Animation0"),
    });
    commands.insert_resource(CharacterModels(HashMap::from_iter([
        (
            Body::Monkey,
            asset_server.load("models/characters/farmer_idle.gltf#Scene0"),
        ),
        (
            Body::Robot,
            asset_server.load("models/characters/robot.gltf#Scene0"),
        ),
        (
            Body::FastRobot,
            asset_server.load("models/characters/fast_robot.gltf#Scene0"),
        ),
        (
            Body::Boss,
            asset_server.load("models/characters/boss.glb#Scene0"),
        ),
    ])));
}

fn spawn_players(
    mut commands: Commands,
    mut events: EventReader<SpawnPlayerEvent>,
    character_models: Res<CharacterModels>,
    farmer_animations: Res<FarmerAnimations>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        let speed = match event.body {
            Body::Monkey => 20.0,
            Body::Robot => 10.0,
            Body::FastRobot => 14.0,
            Body::Boss => 7.5,
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
            Body::Robot | Body::FastRobot | Body::Boss => {
                // EXPLANATION: see docs/physics.txt
                CollisionGroups::new(
                    Group::from_bits(COLLISION_CHARACTER).unwrap(),
                    Group::from_bits(COLLISION_CHARACTER | COLLISION_WORLD | COLLISION_PROJECTILES)
                        .unwrap(),
                )
            }
        };
        let health = match event.body {
            Body::Monkey => Health::new(PLAYER_HEALTH),
            Body::Robot => Health::new(ROBOT_HEALTH),
            Body::FastRobot => Health::new(FAST_ROBOT_HEALTH),
            Body::Boss => Health::new(BOSS_HEALTH),
        };
        let weapon_stats = match event.body {
            Body::Monkey => WeaponStats::default(),
            Body::Robot => WeaponStats {
                cooldown_mul: 1.0,
                damage_add: 1,
            },
            Body::FastRobot => WeaponStats {
                cooldown_mul: 0.8,
                damage_add: 0,
            },
            Body::Boss => WeaponStats {
                cooldown_mul: 1.0,
                damage_add: 1,
            },
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
                    health,
                ),
                (
                    ShowHealthBar,
                    weapon_stats,
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

        match event.body {
            Body::Monkey => {
                let y_offset = 0.0;
                let idle = commands
                    .spawn((
                        AnimationEntityLinkTrap,
                        SceneBundle {
                            scene: farmer_animations.idle_model.clone(),
                            transform: Transform::from_translation(vec3(0.0, y_offset, 0.0)),
                            ..default()
                        },
                    ))
                    .set_parent(player_root)
                    .id();
                let run = commands
                    .spawn((
                        AnimationEntityLinkTrap,
                        SceneBundle {
                            scene: farmer_animations.run_model.clone(),
                            transform: Transform::from_translation(vec3(0.0, y_offset, 0.0)),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                    ))
                    .set_parent(player_root)
                    .id();
                let attack = commands
                    .spawn((
                        AnimationEntityLinkTrap,
                        SceneBundle {
                            scene: farmer_animations.attack_model.clone(),
                            transform: Transform::from_translation(vec3(0.0, y_offset, 0.0)),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                    ))
                    .set_parent(player_root)
                    .id();

                commands.entity(player_root).insert(FarmerAnimator {
                    idle: (idle, farmer_animations.idle.clone()),
                    run: (run, farmer_animations.run.clone()),
                    attack: (attack, farmer_animations.attack.clone()),
                    next_anim: None,
                });
            }
            Body::Robot | Body::FastRobot | Body::Boss => {
                let scene = character_models.0[&event.body].clone();
                let graphics = commands
                    .spawn(SceneBundle {
                        scene,
                        transform: Transform::from_translation(vec3(0.0, 0.5, 0.0)),
                        ..default()
                    })
                    .id();
                commands.entity(graphics).set_parent(player_root);
            }
        }

        if event.is_main {
            commands.entity(player_root).insert((
                PlayerControllerTag,
                MonkeyTag,
                PickupSound,
                Name::new("player"),
            ));
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
