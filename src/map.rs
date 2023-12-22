use std::f32::consts::FRAC_PI_2;

use bevy::{
    math::{vec2, vec3},
    pbr::{ExtendedMaterial, NotShadowCaster, OpaqueRendererMethod},
    prelude::*,
    render::texture::{
        ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
    },
};
use bevy_rapier3d::prelude::*;
use bracket_noise::prelude::*;
use rand::Rng;

use crate::{
    border_material::BorderMaterial,
    collision_groups::{COLLISION_BORDER, COLLISION_WORLD},
    ground_material::GroundMaterial,
    tree::{SpawnTreeEvent, TreeBlueprint, TriggerSpawnTrees},
};

pub const MAP_SIZE_HALF: f32 = 20.0;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Startup, setup_visual_border);
        app.add_systems(Update, setup_trees);
    }
}

fn setup_trees(
    mut ev_reader: EventReader<TriggerSpawnTrees>,
    mut tree_events: EventWriter<SpawnTreeEvent>,
) {
    let Some(TriggerSpawnTrees(noise_chance)) = ev_reader.read().next() else {
        return;
    };

    let map_size_i = MAP_SIZE_HALF as i32;

    let mut noise = FastNoise::seeded(0);
    noise.set_noise_type(NoiseType::Simplex);
    noise.set_frequency(100.0);

    let mut rng = rand::thread_rng();

    for z in (-map_size_i + 1)..(map_size_i - 1) {
        for x in (-map_size_i + 1)..(map_size_i - 1) {
            let noise = noise.get_noise(z as f32, x as f32);
            // 60% chance to discard randomly
            let random_discard = rng.gen_range(0.0..1.0) > *noise_chance;

            if noise > 0.2 && !random_discard {
                tree_events.send(SpawnTreeEvent {
                    pos: vec3(x as f32, 0.0, z as f32),
                    blueprint: TreeBlueprint::Randomized,
                    play_sound: false,
                });
            }
        }
    }
}

/// set up ground and walls
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, GroundMaterial>>>,
    asset_server: Res<AssetServer>,
) {
    let settings = move |s: &mut ImageLoaderSettings| {
        s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..default()
        });
    };
    let grass_img = asset_server.load_with_settings("textures/Grass_01.png", settings);
    let ground_img = asset_server.load_with_settings("textures/Dirt_01.png", settings);
    // ground
    commands.spawn((
        Collider::cuboid(MAP_SIZE_HALF * 4.0, 0.1, MAP_SIZE_HALF * 4.0),
        // EXPLANATION: see docs/physics.txt
        CollisionGroups::new(
            Group::from_bits(COLLISION_WORLD).unwrap(), // part of world(1)
            Group::all(),                               // interacts with all
        ),
        MaterialMeshBundle {
            // mesh: meshes.add(shape::Plane::from_size(MAP_SIZE_HALF * 4.4).into()),
            mesh: meshes.add(shape::Plane::from_size(MAP_SIZE_HALF * 2.0 + 15.0).into()),
            // material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
            material: materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color_texture: Some(grass_img),
                    ..default()
                },
                extension: GroundMaterial {
                    // scale: vec2(13.0, 0.3),
                    scale: 13.0,
                    color_texture: ground_img,
                    noise_scale: 0.3,
                    // filler: Default::default(),
                    // filler2: Default::default(),
                    // color_texture: todo!(),
                },
            }),
            transform: Transform::from_translation(vec3(0.0, -0.05, 0.0)),
            ..default()
        },
    ));

    let wall_thickness = 0.5;
    let wall_thickness_half = wall_thickness * 0.5;
    // wall right
    commands.spawn((
        Collider::cuboid(wall_thickness, 10.0, MAP_SIZE_HALF),
        RigidBody::Fixed,
        ColliderMassProperties::Mass(100.0),
        // EXPLANATION: see docs/physics.txt
        CollisionGroups::new(
            Group::from_bits(COLLISION_BORDER).unwrap(), // part of world(1)
            Group::all(),                                // interacts with all
        ),
        PbrBundle {
            transform: Transform::from_translation(vec3(
                MAP_SIZE_HALF + wall_thickness_half,
                0.0,
                0.0,
            )),
            ..default()
        },
    ));
    // wall left
    commands.spawn((
        Collider::cuboid(wall_thickness, 10.0, MAP_SIZE_HALF),
        RigidBody::Fixed,
        ColliderMassProperties::Mass(100.0),
        // EXPLANATION: see docs/physics.txt
        CollisionGroups::new(
            Group::from_bits(COLLISION_BORDER).unwrap(), // part of world(1)
            Group::all(),                                // interacts with all
        ),
        PbrBundle {
            transform: Transform::from_translation(vec3(
                -MAP_SIZE_HALF - wall_thickness_half,
                0.0,
                0.0,
            )),
            ..default()
        },
    ));
    // wall +z
    commands.spawn((
        Collider::cuboid(MAP_SIZE_HALF, 10.0, wall_thickness),
        RigidBody::Fixed,
        ColliderMassProperties::Mass(100.0),
        // EXPLANATION: see docs/physics.txt
        CollisionGroups::new(
            Group::from_bits(COLLISION_BORDER).unwrap(), // part of world(1)
            Group::all(),                                // interacts with all
        ),
        PbrBundle {
            transform: Transform::from_translation(vec3(
                0.0,
                0.0,
                MAP_SIZE_HALF + wall_thickness_half,
            )),
            ..default()
        },
    ));
    // wall -z
    commands.spawn((
        Collider::cuboid(MAP_SIZE_HALF, 10.0, wall_thickness),
        RigidBody::Fixed,
        ColliderMassProperties::Mass(100.0),
        // EXPLANATION: see docs/physics.txt
        CollisionGroups::new(
            Group::from_bits(COLLISION_BORDER).unwrap(), // part of world(1)
            Group::all(),                                // interacts with all
        ),
        PbrBundle {
            transform: Transform::from_translation(vec3(
                0.0,
                0.0,
                -MAP_SIZE_HALF - wall_thickness_half,
            )),
            ..default()
        },
    ));
}

#[derive(Resource)]
pub struct BorderHandle(pub Handle<Image>);

fn setup_visual_border(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, BorderMaterial>>>,
    asset_server: Res<AssetServer>,
) {
    let settings = move |s: &mut ImageLoaderSettings| {
        s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..default()
        });
    };
    let border_img = asset_server.load_with_settings("textures/border.png", settings);
    // let border_img = asset_server.load("textures/border.png");
    commands.insert_resource(BorderHandle(border_img.clone()));

    let wall_height = 4.0;

    let mesh = meshes.add(shape::Quad::new(vec2(MAP_SIZE_HALF * 2.0, wall_height)).into());
    let material = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            opaque_render_method: OpaqueRendererMethod::Auto,
            alpha_mode: AlphaMode::Blend,
            ..default()
        },
        extension: BorderMaterial {
            quantize_steps: 3,
            color_texture: border_img.clone(),
        },
    });

    // wall right
    commands.spawn((
        NotShadowCaster,
        MaterialMeshBundle {
            mesh: mesh.clone(),
            transform: Transform::from_translation(vec3(MAP_SIZE_HALF, wall_height * 0.5, 0.0))
                .with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
            material: material.clone(),
            ..default()
        },
    ));
    // wall right
    commands.spawn((
        NotShadowCaster,
        MaterialMeshBundle {
            mesh: mesh.clone(),
            transform: Transform::from_translation(vec3(-MAP_SIZE_HALF, wall_height * 0.5, 0.0))
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
            material: material.clone(),
            ..default()
        },
    ));
    // wall up
    commands.spawn((
        NotShadowCaster,
        MaterialMeshBundle {
            mesh: mesh.clone(),
            transform: Transform::from_translation(vec3(0.0, wall_height * 0.5, -MAP_SIZE_HALF)),
            material: material.clone(),
            ..default()
        },
    ));
    // wall bottom
    commands.spawn((
        NotShadowCaster,
        MaterialMeshBundle {
            mesh: mesh.clone(),
            transform: Transform::from_translation(vec3(0.0, wall_height * 0.5, MAP_SIZE_HALF)),
            material: material.clone(),
            // .with_rotation(Quat::from_rotation_y(PI)),
            ..default()
        },
    ));
}
