use bevy::{input::mouse::MouseMotion, math::vec3, prelude::*};
use dolly::prelude::*;

use crate::{player::PlayerControllerTag, utils::movement_axis};

#[derive(Component)]
pub struct MainCameraTag;

// attach to make it free flying
#[derive(Component)]
pub struct FreeFlyCamera;

// attach to make it free flying
#[derive(Component)]
pub struct FollowPlayerCamera;

#[derive(Component)]
pub struct DollyCamera {
    pub rig: CameraRig,
    pub speed: f32,
    pub rotation_speed: f32,
}

#[derive(Resource, Reflect)]
pub struct FollowCameraSettings {
    pub offset: Vec3,
    pub yaw: f32,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FollowCameraSettings>()
            .add_systems(Update, ((free_fly_input, follow_player), update).chain());
    }
}
impl DollyCamera {
    pub fn new(pos: Vec3, rotation: Quat, speed: f32) -> Self {
        let mut yaw = YawPitch::new();
        yaw.set_rotation_quat(rotation.into());
        Self {
            rig: CameraRig::builder()
                .with(Position::new(pos.into()))
                .with(yaw)
                .with(Smooth::new_position_rotation(1.0, 1.0))
                .build(),
            speed,
            rotation_speed: 3.0,
        }
    }
}

impl Default for FollowCameraSettings {
    fn default() -> Self {
        Self {
            offset: vec3(0.0, 15.0, 12.0),
            yaw: -50f32,
        }
    }
}

pub fn follow_player(
    players: Query<&GlobalTransform, With<PlayerControllerTag>>,
    mut cameras: Query<&mut DollyCamera, With<FollowPlayerCamera>>,
    camera_settings: Res<FollowCameraSettings>,
) {
    let mut dolly_cam = cameras.single_mut();
    let Ok(player) = players.get_single() else {
        return;
    };

    let pos_driver = dolly_cam.rig.driver_mut::<Position>();
    pos_driver.position = player.translation() + camera_settings.offset;

    let yaw_pitch = dolly_cam.rig.driver_mut::<YawPitch>();
    yaw_pitch.pitch_degrees = camera_settings.yaw;
    yaw_pitch.yaw_degrees = 0.0;
}

pub fn free_fly_input(
    keyboard: Res<Input<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<&mut DollyCamera, With<FreeFlyCamera>>,
    time: Res<Time>,
) {
    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        mouse_delta += event.delta;
    }
    if mouse_delta.is_nan() {
        mouse_delta = Vec2::ZERO;
    }
    mouse_delta *= time.delta_seconds();
    for mut cam in query.iter_mut() {
        let forward = movement_axis(&keyboard, KeyCode::S, KeyCode::W);
        let side = movement_axis(&keyboard, KeyCode::D, KeyCode::A);
        let y = movement_axis(&keyboard, KeyCode::Space, KeyCode::ShiftLeft);

        let mut translation = vec3(side, 0.0, forward).normalize_or_zero();

        // rotate translation so it's looking where camera is
        let rotation = cam.rig.final_transform.rotation;
        translation = rotation * translation;
        translation.y += y;

        let speed = cam.speed;
        let rotation_speed = cam.rotation_speed;

        cam.rig
            .driver_mut::<Position>()
            .translate(translation * time.delta_seconds() * speed);
        cam.rig.driver_mut::<YawPitch>().rotate_yaw_pitch(
            -mouse_delta.x * rotation_speed,
            -mouse_delta.y * rotation_speed,
        );
    }
}

pub fn update(mut query: Query<(&mut Transform, &mut DollyCamera)>, time: Res<Time>) {
    for (mut transform, mut dolly_cam) in query.iter_mut() {
        dolly_cam.rig.update(time.delta_seconds());
        transform.translation = dolly_cam.rig.final_transform.position;
        transform.rotation = dolly_cam.rig.final_transform.rotation;
    }
}
