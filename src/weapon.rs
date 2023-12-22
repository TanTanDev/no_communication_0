use bevy::{audio::PlaybackMode, prelude::*};
use bevy_rapier3d::prelude::{Collider, QueryFilter, RapierContext};
use rand::Rng;

use crate::{
    health::{ApplyHealthEvent, Health},
    player::Body,
    projectile::{ProjectileAsset, SpawnProjectileEvent},
};

pub const AXE_SFX_COOLDOWN: f32 = 0.11;
pub const PROJ_SFX_COOLDOWN: f32 = 0.3;
pub const SLEDGEHAMMER_SFX_COOLDOWN: f32 = 0.6;

#[derive(Resource)]
pub struct AxeSfxCooldownTimer(pub f32);
#[derive(Resource)]
pub struct ProjSfxCooldownTimer(pub f32);

#[derive(Component, Reflect)]
pub struct WeaponStats {
    pub cooldown_mul: f32,
    pub damage_add: i32,
}

impl Default for WeaponStats {
    fn default() -> Self {
        Self {
            cooldown_mul: 1.0,
            damage_add: 0,
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub enum WeaponType {
    Axe,
    Bow(Handle<ProjectileAsset>),
    SledgeHammer,
}

// should maybe be fetched from asssets
impl WeaponType {
    pub fn sound_effect(&self) -> (String, f32) {
        let (sound_name, volume) = match self {
            WeaponType::Axe => ("axe", 0.5),
            WeaponType::Bow(_) => ("bow", 0.9),
            WeaponType::SledgeHammer => ("sledgehammer", 1.0),
        };
        let path = format!("sounds/{}-projectile.ogg", sound_name);
        (path, volume)
    }

    pub fn cooldown(&self) -> f32 {
        match self {
            WeaponType::Axe => 0.4,
            WeaponType::Bow(_) => 0.6,
            WeaponType::SledgeHammer => 1.4,
        }
    }
}

#[derive(Component, Reflect)]
pub struct WeaponCooldown {
    pub time_left: f32,
}

// execute CastWeaponEvent if spell isn't on cooldown
#[derive(Event)]
pub struct TryCastWeaponEvent {
    pub caster_entity: Entity,
    pub target_entity: Option<Entity>,
    pub dir: Vec3,
}

// any entity can at any point execute a "spell", regardless of cooldown using this
#[derive(Event)]
pub struct CastWeaponEvent {
    pub caster_entity: Entity,
    pub target_entity: Option<Entity>,
    weapon_type: WeaponType,
    dir: Vec3,
}

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WeaponCooldown>()
            .register_type::<WeaponType>()
            .register_type::<WeaponStats>()
            .add_event::<TryCastWeaponEvent>()
            .add_event::<CastWeaponEvent>()
            .add_systems(
                PostUpdate,
                (
                    update_cooldown,
                    promote_try_cast,
                    (cast_axes, cast_projectiles, cast_sledgehammer),
                )
                    .chain(),
            );
    }
}

pub fn update_cooldown(
    mut query: Query<Option<&mut WeaponCooldown>>,
    time: Res<Time>,
    mut sfx_cooldown: ResMut<ProjSfxCooldownTimer>,
) {
    sfx_cooldown.0 += time.delta_seconds();
    for mut cooldown in query.iter_mut().flatten() {
        cooldown.time_left -= time.delta_seconds();
    }
}

// spell attempts are performed, if it isn't on cooldown
pub fn promote_try_cast(
    mut try_events: EventReader<TryCastWeaponEvent>,
    mut events: EventWriter<CastWeaponEvent>,
    mut weapon_query: Query<(&mut WeaponCooldown, &WeaponType, &WeaponStats)>,
    player_query: Query<&Body>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sfx_cooldown: ResMut<ProjSfxCooldownTimer>,
) {
    for event in try_events.read() {
        let cast_by_monkey = player_query
            .get(event.caster_entity)
            .map(|body| *body == Body::Monkey)
            .unwrap_or(false);

        let Ok((mut cooldown, weapon_type, stats)) = weapon_query.get_mut(event.caster_entity)
        else {
            continue;
        };
        // on cooldown abort
        if cooldown.time_left > 0.0 {
            continue;
        }

        if sfx_cooldown.0 >= PROJ_SFX_COOLDOWN || cast_by_monkey {
            let (sound_path, volume) = weapon_type.sound_effect();
            commands.spawn(AudioBundle {
                source: asset_server.load(sound_path),
                settings: PlaybackSettings {
                    volume: bevy::audio::Volume::Relative(bevy::audio::VolumeLevel::new(volume)),
                    speed: 1.0 + rand::thread_rng().gen::<f32>(),
                    mode: PlaybackMode::Despawn,
                    ..Default::default()
                },
            });
            sfx_cooldown.0 = 0.0;
        }
        // yay cast spell
        cooldown.time_left = weapon_type.cooldown() * stats.cooldown_mul;
        events.send(CastWeaponEvent {
            caster_entity: event.caster_entity,
            target_entity: event.target_entity,
            weapon_type: weapon_type.clone(),
            dir: event.dir.try_normalize().unwrap_or(Vec3::Z),
        });
    }
}

// axe behaviour
pub fn cast_axes(
    mut events: EventReader<CastWeaponEvent>,
    mut query: Query<(&GlobalTransform, &WeaponStats)>,
    rapier_context: Res<RapierContext>,
    mut apply_health_events: EventWriter<ApplyHealthEvent>,
    mut gizmos: Gizmos,
    transforms: Query<&GlobalTransform, With<Health>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sfx_cooldown: ResMut<AxeSfxCooldownTimer>,
    time: Res<Time>,
) {
    for event in events.read() {
        let Ok((caster_transform_g, stats)) = query.get_mut(event.caster_entity) else {
            continue;
        };
        let WeaponType::Axe = &event.weapon_type else {
            continue;
        };

        let axe_range = 2.6;
        // 90 degree swing
        let axe_cone_dot = 0.3;

        let shape = Collider::ball(axe_range);
        let shape_pos = caster_transform_g.translation();
        let filter = QueryFilter::default();
        const AXE_DAMAGE: i32 = 1;
        let axe_damage = stats.damage_add + AXE_DAMAGE;
        const MAX_HIT: i32 = 2;
        let mut hits = 0;
        rapier_context.intersections_with_shape(
            shape_pos,
            Quat::IDENTITY,
            &shape,
            filter,
            |hit_entity| {
                let Ok(hit_transform) = transforms.get(hit_entity) else {
                    return true;
                };
                let to_target = caster_transform_g.translation() - hit_transform.translation();
                // let to_target = hit_transform.translation() - caster_transform_g.translation();
                let to_target_dir = to_target.normalize();
                let caster_dir = event.dir;
                let dot = -caster_dir.dot(to_target_dir);
                let is_outside_of_cone = dot < axe_cone_dot;
                if is_outside_of_cone {
                    return true;
                }

                // don't hurt self
                if hit_entity == event.caster_entity {
                    // continue intersection_with_shape
                    return true;
                }
                gizmos.sphere(
                    hit_transform.translation(),
                    Quat::IDENTITY,
                    0.9,
                    Color::YELLOW,
                );
                gizmos.line(
                    caster_transform_g.translation() + Vec3::Y * 2.0,
                    hit_transform.translation() + Vec3::Y * 2.0,
                    Color::YELLOW,
                );
                if sfx_cooldown.0 >= AXE_SFX_COOLDOWN {
                    commands.spawn(AudioBundle {
                        source: asset_server.load("sounds/chop.ogg"),
                        settings: PlaybackSettings {
                            volume: bevy::audio::Volume::Relative(bevy::audio::VolumeLevel::new(
                                0.6,
                            )),
                            speed: 1.0 + rand::thread_rng().gen::<f32>(),
                            ..Default::default()
                        },
                    });
                    sfx_cooldown.0 = 0.0;
                } else {
                    sfx_cooldown.0 += time.delta_seconds();
                }
                apply_health_events.send(ApplyHealthEvent {
                    amount: -axe_damage,
                    target_entity: hit_entity,
                    caster_entity: event.caster_entity,
                });
                hits += 1;
                if hits <= MAX_HIT - 1 {
                    true // continute search
                } else {
                    false // don't hit anything more
                }
            },
        );
    }
}

pub fn cast_projectiles(
    mut events: EventReader<CastWeaponEvent>,
    mut query: Query<(&GlobalTransform, &WeaponStats)>,
    mut projectile_events: EventWriter<SpawnProjectileEvent>,
) {
    for event in events.read() {
        let Ok((caster_transform_g, stats)) = query.get_mut(event.caster_entity) else {
            continue;
        };
        let WeaponType::Bow(projectile_asset) = &event.weapon_type else {
            continue;
        };

        projectile_events.send(SpawnProjectileEvent {
            pos: caster_transform_g.translation(),
            dir: event.dir,
            projectile_asset: projectile_asset.clone(),
            additional_damage: stats.damage_add,
            caster_entity: event.caster_entity,
            target_entity: event.target_entity,
        })
    }
}

// sledgehammer behaviour (pretty much a big axe)
pub fn cast_sledgehammer(
    mut events: EventReader<CastWeaponEvent>,
    mut query: Query<(&GlobalTransform, &WeaponStats)>,
    rapier_context: Res<RapierContext>,
    mut apply_health_events: EventWriter<ApplyHealthEvent>,
    mut gizmos: Gizmos,
    transforms: Query<&GlobalTransform, With<Health>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sfx_cooldown: ResMut<AxeSfxCooldownTimer>,
    time: Res<Time>,
) {
    for event in events.read() {
        let Ok((caster_transform_g, stats)) = query.get_mut(event.caster_entity) else {
            continue;
        };
        let WeaponType::SledgeHammer = &event.weapon_type else {
            continue;
        };

        let axe_range = 2.6;
        // 90 degree swing
        let axe_cone_dot = 0.3;

        let shape = Collider::ball(axe_range);
        let shape_pos = caster_transform_g.translation();
        let filter = QueryFilter::default();
        const SLEDGEHAMMER_DAMAGE: i32 = 6;
        let sledgehammer_damage = stats.damage_add + SLEDGEHAMMER_DAMAGE;
        const MAX_HIT: i32 = 2;
        let mut hits = 0;
        rapier_context.intersections_with_shape(
            shape_pos,
            Quat::IDENTITY,
            &shape,
            filter,
            |hit_entity| {
                let Ok(hit_transform) = transforms.get(hit_entity) else {
                    return true;
                };
                let to_target = caster_transform_g.translation() - hit_transform.translation();
                // let to_target = hit_transform.translation() - caster_transform_g.translation();
                let to_target_dir = to_target.normalize();
                let caster_dir = event.dir;
                let dot = -caster_dir.dot(to_target_dir);
                let is_outside_of_cone = dot < axe_cone_dot;
                if is_outside_of_cone {
                    return true;
                }

                // don't hurt self
                if hit_entity == event.caster_entity {
                    // continue intersection_with_shape
                    return true;
                }
                gizmos.sphere(
                    hit_transform.translation(),
                    Quat::IDENTITY,
                    0.9,
                    Color::YELLOW,
                );
                gizmos.line(
                    caster_transform_g.translation() + Vec3::Y * 2.0,
                    hit_transform.translation() + Vec3::Y * 2.0,
                    Color::YELLOW,
                );
                if sfx_cooldown.0 >= SLEDGEHAMMER_SFX_COOLDOWN {
                    commands.spawn(AudioBundle {
                        source: asset_server.load("sounds/chop.ogg"),
                        settings: PlaybackSettings {
                            volume: bevy::audio::Volume::Relative(bevy::audio::VolumeLevel::new(
                                0.6,
                            )),
                            speed: 1.0 + rand::thread_rng().gen::<f32>(),
                            ..Default::default()
                        },
                    });
                    sfx_cooldown.0 = 0.0;
                } else {
                    sfx_cooldown.0 += time.delta_seconds();
                }
                apply_health_events.send(ApplyHealthEvent {
                    amount: -sledgehammer_damage,
                    target_entity: hit_entity,
                    caster_entity: event.caster_entity,
                });
                hits += 1;
                if hits <= MAX_HIT - 1 {
                    true // continute search
                } else {
                    false // don't hit anything more
                }
            },
        );
    }
}
