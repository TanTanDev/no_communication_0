use bevy::{math::vec3, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy_vector_shapes::ShapePlugin;
use no_communication_0::{
    border_material::BorderMaterialPlugin,
    camera::{CameraPlugin, DollyCamera, FollowPlayerCamera, MainCameraTag},
    health::HealthPlugin,
    inventory::{InventoryPlugin, Item},
    item_pickups::ItemPickupPlugin,
    map::{MapPlugin, MAP_SIZE_HALF},
    notification::{NotificationEvent, NotificationPlugin},
    pickup::PickupPlugin,
    player::{Body, PlayerPlugin, SpawnPlayerEvent},
    pointer::PointerPlugin,
    projectile::ProjectilePlugin,
    shop::{ShopItemData, ShopItemEffect, ShopPlugin, SpawnShopItemEvent},
    state::{AppState, StatePlugin},
    tower::TowerPlugin,
    tree::{TreePlugin, TriggerSpawnTrees},
    ui_util::UiUtilPlugin,
    waves::WavePlugin,
    weapon::{AxeSfxCooldownTimer, ProjSfxCooldownTimer, WeaponPlugin, WeaponType},
};
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
            ShapePlugin::default(),
        ))
        // Our plugins
        .add_plugins((
            (
                BorderMaterialPlugin,
                UiUtilPlugin,
                CameraPlugin,
                PlayerPlugin,
                WeaponPlugin,
                PickupPlugin,
                HealthPlugin,
                TreePlugin,
                ItemPickupPlugin,
                ProjectilePlugin,
                InventoryPlugin,
                ShopPlugin,
                PointerPlugin,
                MapPlugin,
                NotificationPlugin,
            ),
            (TowerPlugin, WavePlugin, StatePlugin),
        ))
        // debug + large amount of rapier objects LAGS a lot, reduce MAP_SIZE_HALF in that case
        // .add_plugins(RapierDebugRenderPlugin::default())
        // edit camera settings in ui
        // .add_plugins(ResourceInspectorPlugin::<FollowCameraSettings>::default())
        // Enable for inspector
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut rapier_config: ResMut<RapierConfiguration>,
    mut spawn_player_event: EventWriter<SpawnPlayerEvent>,
    mut spawn_shop_item_event: EventWriter<SpawnShopItemEvent>,
    mut notification_event: EventWriter<NotificationEvent>,
    mut tree_trigger_writer: EventWriter<TriggerSpawnTrees>,
    asset_server: Res<AssetServer>,
) {
    tree_trigger_writer.send(TriggerSpawnTrees(0.1));

    rapier_config.gravity = Vec3::NEG_Y * 100.0;

    let mut rng = rand::thread_rng();
    spawn_player_event.send(SpawnPlayerEvent {
        pos: vec3(
            rng.gen_range(-MAP_SIZE_HALF..MAP_SIZE_HALF),
            1.0,
            rng.gen_range(-MAP_SIZE_HALF..MAP_SIZE_HALF),
        ),
        is_main: true,
        body: Body::Monkey,
        weapon_type: WeaponType::Bow(asset_server.load("projectiles/bow.projectile.ron")),
    });
    let mut x = MAP_SIZE_HALF + rng.gen_range(10.0..20.0);
    let mut z = MAP_SIZE_HALF + rng.gen_range(10.0..20.0);
    x *= match rng.gen::<bool>() {
        true => 1.0,
        false => -1.0,
    };
    z *= match rng.gen::<bool>() {
        true => 1.0,
        false => -1.0,
    };
    spawn_player_event.send(SpawnPlayerEvent {
        pos: vec3(x, 4.0, z),
        is_main: false,
        body: Body::Robot,
        weapon_type: WeaponType::Axe,
    });

    {
        spawn_shop_item_event.send(SpawnShopItemEvent {
            item: ShopItemData {
                cost: vec![(Item::Log, 3)],
                effects: vec![(ShopItemEffect::PlantTree)],
                permanent: true,
            },
        });
        spawn_shop_item_event.send(SpawnShopItemEvent {
            item: ShopItemData {
                cost: vec![(Item::Apple, 2)],
                effects: vec![(ShopItemEffect::Heal(10))],
                permanent: true,
            },
        });
    }

    // light
    commands.insert_resource(AmbientLight {
        brightness: 1.0,
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 30000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(1.0, 8.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    let transform = Transform::from_xyz(-2.0, 18.5, 25.0).looking_at(Vec3::ZERO, Vec3::Y);
    let pos = transform.translation;
    let rotation = transform.rotation;

    // appstate
    commands.insert_resource(AppState::Wave(0));
    commands.insert_resource(AxeSfxCooldownTimer(0.0));
    commands.insert_resource(ProjSfxCooldownTimer(0.0));

    // camera
    commands.spawn((
        MainCameraTag,
        FollowPlayerCamera,
        DollyCamera::new(pos, rotation, 10.0),
        Camera3dBundle {
            transform,
            ..default()
        },
    ));

    notification_event.send(NotificationEvent {
        text: "Protect The Trees!".into(),
        show_for: 3.0,
        color: Color::WHITE,
    });
    notification_event.send(NotificationEvent {
        text: "Wave 1!".into(),
        show_for: 3.0,
        color: Color::BLUE,
    });
}
