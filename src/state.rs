use bevy::{core::FrameCount, math::vec3, prelude::*};
use rand::Rng;

use crate::{
    map::MAP_SIZE_HALF,
    notification::NotificationEvent,
    player::{Body, PlayerControllerTag, SpawnPlayerEvent},
    shop::SpawnShopItemEvent,
    tree::TreeTrunkTag,
    waves::{WaveDescriptors, WaveDescriptorsAsset},
    weapon::WeaponType,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Resource)]
pub enum AppState {
    Init,
    Wave(usize),
    Lost,
    Win,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Last,
            handle_next_wave
                .run_if(check_for_no_robots)
                .run_if(|v: Res<AppState>| matches!(&*v, AppState::Wave(_)))
                .run_if(not(reached_max_wave))
                .run_if(|f: Res<FrameCount>| f.0 > 3),
        );
        app.add_systems(
            Last,
            handle_win
                .run_if(check_for_no_robots)
                .run_if(reached_max_wave)
                .run_if(|f: Res<FrameCount>| f.0 > 3)
                .before(handle_next_wave),
        );
        app.add_systems(
            Last,
            handle_loss
                .run_if(check_for_loss)
                .run_if(|v: Res<AppState>| !(resource_equals::<AppState>(AppState::Lost))(v))
                .run_if(|f: Res<FrameCount>| f.0 > 3),
        );
    }
}

fn reached_max_wave(
    state: Res<AppState>,
    wave_descriptors: Res<WaveDescriptors>,
    wave_descriptor_assets: Res<Assets<WaveDescriptorsAsset>>,
) -> bool {
    let Some(wave) = wave_descriptor_assets.get(&wave_descriptors.0) else {
        return false;
    };
    let max_wave = wave.0.len();
    matches!(&*state, AppState::Wave(w) if *w == max_wave-1)
}

fn check_for_no_robots(players: Query<&Body>) -> bool {
    players
        .into_iter()
        .filter(|b| {
            matches!(b, Body::Robot) || matches!(b, Body::FastRobot) || matches!(b, Body::Boss)
        })
        .count()
        == 0
}

pub fn handle_next_wave(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut app_state: ResMut<AppState>,
    mut spawn_player_event: EventWriter<SpawnPlayerEvent>,
    mut notification_event: EventWriter<NotificationEvent>,
    mut spawn_shop_item_event: EventWriter<SpawnShopItemEvent>,
    wave_descriptors: Res<WaveDescriptors>,
    wave_descriptor_assets: Res<Assets<WaveDescriptorsAsset>>,
) {
    let AppState::Wave(wave) = app_state.as_mut() else {
        panic!("how did we get here?");
    };
    // tree_trigger_writer.send(TriggerSpawnTrees(0.1 - *wave as f32 / 30.0));
    let mut rng = rand::thread_rng();

    commands.spawn(AudioBundle {
        source: asset_server.load("sounds/next-level.ogg"),
        ..default()
    });

    *wave += 1;

    let wave_descriptors = &wave_descriptor_assets.get(&wave_descriptors.0).unwrap().0;
    let is_last_wave = wave_descriptors.len() - 1 == *wave;
    let wave_descriptor = wave_descriptors[*wave].clone();

    for i in 1..(1 + wave_descriptor.nb_enemies) {
        let weapon_type = WeaponType::Axe;
        let mut x = MAP_SIZE_HALF + rng.gen_range(6.0..26.0);
        let mut z = MAP_SIZE_HALF + rng.gen_range(6.0..26.0);
        x *= match rng.gen::<bool>() {
            true => 1.0,
            false => -1.0,
        };
        z *= match rng.gen::<bool>() {
            true => 1.0,
            false => -1.0,
        };
        let mut body = Body::Robot;
        let p = i as f32 / wave_descriptor.nb_enemies as f32;
        if p > 0.7 {
            body = Body::FastRobot;
        }
        if is_last_wave && i == wave_descriptor.nb_enemies {
            body = Body::Boss;
        }
        spawn_player_event.send(SpawnPlayerEvent {
            pos: vec3(x, 4.0, z),
            is_main: false,
            body,
            weapon_type,
        });
    }

    for new_item in wave_descriptor.new_shop_items {
        spawn_shop_item_event.send(SpawnShopItemEvent { item: new_item });
    }

    notification_event.send(NotificationEvent {
        text: format!("Wave {}!", *wave),
        show_for: 3.0,
        color: Color::BLUE,
    });
}

pub fn handle_win(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut notification_event: EventWriter<NotificationEvent>,
    mut app_state: ResMut<AppState>,
) {
    let AppState::Wave(wave) = &mut *app_state else {
        return;
    };
    *wave += 1;

    commands.spawn(AudioBundle {
        source: asset_server.load("sounds/win.ogg"),
        ..default()
    });

    notification_event.send(NotificationEvent {
        text: "You Win!".into(),
        show_for: 60.0,
        color: Color::GREEN,
    });

    *app_state = AppState::Win;
}

fn check_for_loss(
    trees: Query<Entity, With<TreeTrunkTag>>,
    player: Query<Entity, With<PlayerControllerTag>>,
) -> bool {
    //apply lose sound effect
    trees.is_empty() || player.is_empty()
}

pub fn handle_loss(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut notification_event: EventWriter<NotificationEvent>,
) {
    commands.spawn(AudioBundle {
        source: asset_server.load("sounds/lost.ogg"),
        ..default()
    });

    notification_event.send(NotificationEvent {
        text: "You Lost!".into(),
        show_for: 5.0,
        color: Color::RED,
    });

    commands.insert_resource(AppState::Lost);
}
