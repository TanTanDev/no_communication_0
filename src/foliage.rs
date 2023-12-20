use bevy::{math::vec3, prelude::*};
use bevy_rapier3d::prelude::*;
use bracket_noise::prelude::{FastNoise, NoiseType};
use rand::Rng;

use crate::map::MAP_SIZE_HALF;

#[derive(Event)]
pub struct SpawnFoliageEvent {
    pub pos: Vec3,
}

#[derive(Component)]
pub struct TreeRootTag;

#[derive(Component)]
pub struct TreeTrunkTag;

// reference all tree 3d models
#[derive(Resource)]
pub struct TreeModels(Vec<Handle<Scene>>);

pub struct FoliagePlugin;

impl Plugin for FoliagePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnFoliageEvent>()
            .add_systems(Startup, setup_tree_resources)
            .add_systems(Startup, setup_foliage)
            .add_systems(Update, (spawn_foliage,));
    }
}

fn setup_foliage(mut foliage_events: EventWriter<SpawnFoliageEvent>) {
    let map_size_i = MAP_SIZE_HALF as i32;

    let mut noise = FastNoise::seeded(1);
    noise.set_noise_type(NoiseType::Simplex);
    noise.set_frequency(100.0);

    let mut rng = rand::thread_rng();

    for z in (-map_size_i + 1)..(map_size_i - 1) {
        for x in (-map_size_i + 1)..(map_size_i - 1) {
            let noise = noise.get_noise(z as f32, x as f32);
            // 70% chance to discard randomly
            let random_discard = rng.gen_range(0.0..1.0) < 0.7;

            if noise > 0.4 && !random_discard {
                foliage_events.send(SpawnFoliageEvent {
                    pos: vec3(x as f32, 0.0, z as f32),
                });
            }
        }
    }
}

fn spawn_foliage(
    mut events: EventReader<SpawnFoliageEvent>,
    mut commands: Commands,
    tree_models: Res<TreeModels>,
) {
    for event in events.read() {
        let mut rng = rand::thread_rng();
        let model_handle = tree_models.0[rng.gen_range(0..tree_models.0.len())].clone();
        let scale = rng.gen_range(2.5..=3.5);

        commands.spawn((
            Name::new("foliage"),
            TreeRootTag,
            RigidBody::Fixed,
            SceneBundle {
                scene: model_handle,
                transform: Transform::from_translation(event.pos).with_scale(Vec3::splat(scale)),
                ..default()
            },
        ));
    }
}

fn setup_tree_resources(mut commands: Commands, asset_server: Res<AssetServer>) {
    let models = vec![
        "foliage_0",
        "foliage_1",
        "foliage_2",
        "foliage_3",
        "foliage_4",
        "foliage_5",
        "foliage_6",
    ]
    .iter()
    .map(|name| asset_server.load(format!("models/foliage/{}.gltf#Scene0", name)))
    .collect::<Vec<_>>();
    commands.insert_resource(TreeModels(models));
}
