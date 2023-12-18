use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::{erased_serde::__private::serde::Deserialize, TypePath},
};
use bevy_rapier3d::prelude::{CollisionGroups, Group, QueryFilter, RapierContext};

use crate::{
    asset_utils::CustomAssetLoaderError,
    collision_groups::{COLLISION_CHARACTER, COLLISION_PROJECTILES},
    health::{ApplyHealthEvent, Health, HealthRoot},
};

#[derive(Debug, Deserialize, TypePath, Asset)]
pub struct ProjectileAsset {
    pub speed: f32,
    pub gravity: f32,
    pub spread: f32,
    pub damage: i32,
    // hits until despawn
    pub max_hits: i32,
    pub model: String,
}

#[derive(Event)]
pub struct SpawnProjectileEvent {
    pub caster_entity: Entity,
    pub target_entity: Option<Entity>,
    pub pos: Vec3,
    pub dir: Vec3,
    pub projectile_asset: Handle<ProjectileAsset>,
    pub additional_damage: i32,
}

pub struct ProjectilePlugin;

#[derive(Default)]
pub struct ProjectileAssetLoader;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnProjectileEvent>()
            .init_asset::<ProjectileAsset>()
            .add_systems(Update, (spawn_projectile, (projectile_aim, update).chain()))
            .init_asset_loader::<ProjectileAssetLoader>();
    }
}

// projectiles move through code
#[derive(Component)]
pub struct Projectile {
    // how many things this projectile have hit
    pub hits: i32,
    pub caster_entity: Entity,
    pub target_entity: Option<Entity>,
    pub vel: Vec3,
    pub asset_handle: Handle<ProjectileAsset>,
    pub additional_damage: i32,
}

pub fn projectile_aim(
    mut q_projectile: Query<(&mut Transform, &mut Projectile)>,
    q_target_transform: Query<&GlobalTransform>,
) {
    for (mut projectile_tr, mut projectile) in &mut q_projectile {
        let Some(target_entity) = projectile.target_entity else {
            continue;
        };
        let Ok(target) = q_target_transform.get(target_entity) else {
            continue;
        };

        let to_target_dir = (target.translation() - projectile_tr.translation).normalize();

        projectile_tr.rotation = Quat::from_rotation_arc(-Vec3::Z, to_target_dir);
        projectile.vel = to_target_dir * projectile.vel.length();
    }
}

pub fn update(
    mut query: Query<(Entity, &mut Transform, &mut Projectile)>,
    projectile_assets: Res<Assets<ProjectileAsset>>,
    time: Res<Time>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    hit_query: Query<(Option<&Health>, Option<&HealthRoot>)>,
    mut apply_health_events: EventWriter<ApplyHealthEvent>,
) {
    for (projectile_entity, mut transform, mut projectile) in query.iter_mut() {
        let Some(projectile_asset) = projectile_assets.get(&projectile.asset_handle) else {
            error!("no projectile asset on projectile");
            return;
        };
        let prev_pos = transform.translation;

        projectile.vel -= projectile_asset.gravity * time.delta_seconds();
        transform.translation += projectile.vel * time.delta_seconds();

        // transform.rotation = projectile.vel

        let current_pos = transform.translation;
        let max_toi = prev_pos.distance(current_pos);
        let mut filter = QueryFilter::default();
        // EXPLANATION: see docs/physics.txt
        filter.groups = Some(CollisionGroups::new(
            Group::from_bits(COLLISION_PROJECTILES).unwrap(), // this projectile is part of projectile layer (4)
            Group::from_bits(COLLISION_CHARACTER).unwrap(),   // collide with ALL
        ));

        rapier_context.intersections_with_ray(
            prev_pos,
            projectile.vel.normalize(),
            max_toi,
            true,
            filter,
            |hit_entity, _intersection| {
                let Ok((health, health_root)) = hit_query.get(hit_entity) else {
                    return true; // continue ray
                };

                let health_entity = match (health, health_root) {
                    (None, Some(health_root)) => health_root.entity, // fetched health entity
                    (Some(_health), None) => hit_entity, // original collider has health component
                    _ => return true, // continue search, hit something with no health
                };

                // don't hurt self
                if health_entity == projectile.caster_entity {
                    return true; // continue ray
                }

                apply_health_events.send(ApplyHealthEvent {
                    amount: -projectile_asset.damage - projectile.additional_damage,
                    target_entity: health_entity,
                    caster_entity: projectile.caster_entity,
                });
                projectile.hits += 1;
                if projectile.hits >= projectile_asset.max_hits {
                    commands.entity(projectile_entity).despawn_recursive();
                    return false; // stop ray
                }
                return true; // continue ray
            },
        );
    }
}

pub fn spawn_projectile(
    mut events: EventReader<SpawnProjectileEvent>,
    projectile_assets: Res<Assets<ProjectileAsset>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        let Some(projectile) = projectile_assets.get(&event.projectile_asset) else {
            error!("no such projectile: {:?}", event.projectile_asset);
            continue;
        };
        commands.spawn((
            SceneBundle {
                scene: asset_server.load(&projectile.model),
                transform: Transform::from_translation(event.pos).looking_to(event.dir, Vec3::Y),
                ..default()
            },
            Projectile {
                vel: event.dir * projectile.speed,
                asset_handle: event.projectile_asset.clone(),
                additional_damage: event.additional_damage,
                caster_entity: event.caster_entity,
                target_entity: event.target_entity,
                hits: 0,
            },
        ));
    }
}

impl AssetLoader for ProjectileAssetLoader {
    type Asset = ProjectileAsset;
    type Settings = ();
    type Error = CustomAssetLoaderError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let asset = ron::de::from_bytes::<ProjectileAsset>(&bytes)?;
            Ok(asset)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["projectile.ron"]
    }
}
