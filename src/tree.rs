use bevy::{math::vec3, prelude::*};
use bevy_rapier3d::{prelude::*, rapier::prelude::JointAxis};
use rand::{thread_rng, Rng};

use crate::{
    collision_groups::{
        COLLISION_CHARACTER, COLLISION_NO_PHYSICS, COLLISION_PROJECTILES, COLLISION_TREES,
        COLLISION_WORLD,
    },
    health::{ApplyHealthEvent, DespawnOnHealth0, Health, HealthRoot},
    inventory::Item,
    item_pickups::{SpawnItemEvent, SpawnItemEvery},
};

#[derive(Event)]
pub struct TriggerSpawnTrees(pub f32);

#[derive(Event)]
pub struct SpawnTreeEvent {
    pub pos: Vec3,
    pub blueprint: TreeBlueprint,
    pub play_sound: bool,
}

// how to style tree
pub enum TreeBlueprint {
    Randomized,
    Specific {
        y_scale: f32,
        xz_scale: f32,
        tree_model: Handle<Scene>,
    },
}

#[derive(Component)]
pub struct TreeRootTag;

#[derive(Component)]
pub struct TreeTrunkTag;

// reference all tree 3d models
#[derive(Resource)]
pub struct TreeModels(Vec<Handle<Scene>>);

pub struct TreePlugin;

impl Plugin for TreePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnTreeEvent>()
            .add_event::<TriggerSpawnTrees>()
            .add_systems(Startup, setup_tree_resources)
            .add_systems(Update, (spawn_trees, shake_on_health, spawn_log_on_health));
    }
}

fn shake_on_health(
    mut events: EventReader<ApplyHealthEvent>,
    transforms: Query<&GlobalTransform>,
    mut trees_impulse: Query<&mut ExternalImpulse>,
) {
    for event in events.read() {
        if event.amount >= 0 || event.target_entity == event.caster_entity {
            continue;
        }
        let Ok(mut tree_impulse) = trees_impulse.get_mut(event.target_entity) else {
            continue;
        };
        // get dir
        let Ok(transform) = transforms.get(event.caster_entity) else {
            continue;
        };
        let Ok(transform_2) = transforms.get(event.target_entity) else {
            continue;
        };
        let caster_pos = transform.translation();
        let target_pos = transform_2.translation();
        let mut dir = (caster_pos - target_pos).normalize_or_zero();
        dir.y = -0.3;
        let power = 20.0;
        tree_impulse.impulse = -dir * power;
    }
}

fn spawn_log_on_health(
    mut events: EventReader<ApplyHealthEvent>,
    transforms: Query<&GlobalTransform>,
    mut log_spawn_events: EventWriter<SpawnItemEvent>,
) {
    for event in events.read() {
        let Ok(transform) = transforms.get(event.target_entity) else {
            continue;
        };
        log_spawn_events.send(SpawnItemEvent {
            item: Item::Log,
            pos: transform.translation() + Vec3::Y,
        });
    }
}

pub fn spawn_trees(
    mut events: EventReader<SpawnTreeEvent>,
    mut commands: Commands,
    tree_models: Res<TreeModels>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        if event.play_sound {
            commands.spawn(AudioBundle {
                source: asset_server.load("sounds/plant_tree.ogg"),
                settings: PlaybackSettings::DESPAWN,
            });
        }
        let (model_handle, y_scale, xz_scale) = match &event.blueprint {
            TreeBlueprint::Randomized => {
                let mut rng = rand::thread_rng();
                let model = tree_models.0[rng.gen_range(0..tree_models.0.len())].clone();
                let y_scale = rng.gen_range(0.4..=0.9);
                let xz_scale = y_scale * rng.gen_range(0.5..=0.9);
                (model, y_scale, xz_scale)
            }
            TreeBlueprint::Specific {
                y_scale,
                xz_scale,
                tree_model,
            } => (tree_model.clone(), *y_scale, *xz_scale),
        };

        let joint = SphericalJointBuilder::new()
            .local_anchor1(vec3(0.0, 0.4, 0.0))
            .local_anchor2(vec3(0.0, 0.2, 0.0))
            .limits(JointAxis::X, [0.0, 0.0])
            .limits(JointAxis::Y, [0.0, 0.0])
            .limits(JointAxis::Z, [0.0, 0.0])
            .limits(JointAxis::AngX, [-0.2, 0.2])
            .limits(JointAxis::AngY, [0.0, 0.0])
            .limits(JointAxis::AngZ, [-0.2, 0.2]);

        let root = commands
            .spawn((
                Name::new("Tree"),
                TreeRootTag,
                RigidBody::Fixed,
                TransformBundle::from_transform(Transform::from_translation(event.pos)),
                VisibilityBundle::default(),
            ))
            .id();

        let collider_height = 2.0;
        let collider_radius = 0.2;
        let child = commands
            .spawn((
                TreeTrunkTag,
                DespawnOnHealth0,
                Health::new(6),
                SpawnItemEvery {
                    range: 5.0..20.0,
                    item: if rand::thread_rng().gen_bool(0.1) {
                        Item::Apple
                    } else {
                        Item::Banana
                    },
                    next: time.elapsed_seconds_f64() + thread_rng().gen_range(5.0..120.0),
                },
                SceneBundle {
                    scene: model_handle,
                    transform: Transform::from_translation(vec3(0.0, collider_radius + 0.2, 0.0))
                        .with_scale(vec3(xz_scale, y_scale, xz_scale)),
                    ..default()
                },
                RigidBody::Dynamic,
                Collider::capsule(Vec3::ZERO, vec3(0.0, collider_height, 0.0), collider_radius),
                ColliderMassProperties::Mass(1.0),
                GravityScale(-3.0),
                ExternalImpulse {
                    impulse: Vec3::ZERO,
                    torque_impulse: Vec3::ZERO,
                },
                Damping {
                    linear_damping: 1.0,
                    angular_damping: 1.0,
                },
                ImpulseJoint::new(root, joint),
                // EXPLANATION: see docs/physics.txt
                CollisionGroups::new(
                    Group::from_bits(COLLISION_TREES | COLLISION_WORLD).unwrap(), // group 0: character
                    Group::from_bits(COLLISION_PROJECTILES | COLLISION_WORLD | COLLISION_CHARACTER)
                        .unwrap(), // collides with characters(0) + world(1) + projectiles(4)
                ),
            ))
            .id();
        commands.entity(child).set_parent(root);

        // make hit box larger for projectiles
        commands.entity(child).with_children(|parent| {
            parent.spawn((
                HealthRoot { entity: child },
                Collider::capsule(
                    Vec3::ZERO,
                    vec3(0.0, collider_height, 0.0),
                    collider_radius * 6.0,
                ),
                // EXPLANATION: see docs/physics.txt
                CollisionGroups::new(
                    Group::from_bits(COLLISION_NO_PHYSICS).unwrap(), // part of no_physics(2)
                    Group::from_bits(COLLISION_PROJECTILES).unwrap(), // collides with projectiles(4) only
                ),
                ColliderMassProperties::Mass(0.0), // without this it breaks the anti gravity
            ));
        });

        // anti gravity mass to make trees stand up
        commands.entity(child).with_children(|parent| {
            parent.spawn((
                ColliderMassProperties::Mass(1.0),
                GravityScale(-3.0),
                TransformBundle::from_transform(Transform::from_translation(vec3(
                    0.0,
                    collider_height + 5.0,
                    0.0,
                ))),
            ));
        });
    }
}

fn setup_tree_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    let models = vec![
        "Pine_1", "Pine_2", "Pine_3", "Pine_4", "tree_1", "tree_2", "tree_3", "tree_4", "tree_5",
        "tree_6", "Birch_1", "Birch_2", "Birch_3", "Birch_4", "Birch_5", "Birch_6",
    ]
    .iter()
    .map(|name| asset_server.load(format!("models/trees/{}.gltf#Scene0", name)))
    .collect::<Vec<_>>();
    commands.insert_resource(TreeModels(models));
}
