use std::f32::consts::TAU;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_rapier3d::prelude::{CollisionGroups, Group, QueryFilter, RapierContext};
use bevy_vector_shapes::{prelude::ShapePainter, shapes::RectPainter};

use crate::{
    camera::MainCameraTag,
    collision_groups::{COLLISION_CHARACTER, COLLISION_POINTER, COLLISION_PROJECTILES},
};

pub struct PointerPlugin;

impl Plugin for PointerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_pointer_pos, test_pointer, display_pointer))
            .init_resource::<PointerPos>();
    }
}

fn test_pointer(
    mut commands: Commands,
    pointer: Res<PointerPos>,
    mut sphere: Local<Option<Entity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(p) = pointer.pointer_on {
        let t = Transform::from_translation(p.wpos);
        if let Some(sphere) = *sphere {
            commands.entity(sphere).insert(t);
        } else {
            *sphere = Some(
                commands
                    .spawn(PbrBundle {
                        material: materials.add(StandardMaterial::default()),
                        mesh: meshes.add(Mesh::from(shape::UVSphere {
                            radius: 0.2,
                            sectors: 5,
                            stacks: 5,
                        })),
                        transform: t,
                        ..default()
                    })
                    .id(),
            );
        }
    } else if let Some(e) = *sphere {
        commands.entity(e).despawn();
        *sphere = None;
    }
}

#[derive(Clone, Copy)]
pub struct PointerTarget {
    pub entity: Entity,
    pub wpos: Vec3,
}

#[derive(Resource, Default)]
pub struct PointerPos {
    pub pointer_on: Option<PointerTarget>,
}

pub fn update_pointer_pos(
    mut pointer: ResMut<PointerPos>,
    rapier: Res<RapierContext>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&GlobalTransform, &Camera), With<MainCameraTag>>,
    q_transform: Query<&GlobalTransform>,
    q_parent: Query<&Parent>,
) {
    let window = window.single();
    let (camera_t, camera) = camera.single();
    pointer.pointer_on = window.cursor_position().and_then(|cursor| {
        let ray = camera.viewport_to_world(camera_t, cursor)?;

        let mut filter = QueryFilter::default();
        // EXPLANATION: see docs/physics.txt
        filter.groups = Some(CollisionGroups::new(
            Group::from_bits(COLLISION_POINTER | COLLISION_PROJECTILES).unwrap(),
            Group::from_bits(COLLISION_POINTER | COLLISION_CHARACTER).unwrap(),
        ));
        let (collider_entity, _) =
            rapier.cast_ray(ray.origin, ray.direction, f32::MAX, true, filter)?;

        let entity = q_parent
            .iter_ancestors(collider_entity)
            .last()
            .unwrap_or(collider_entity);
        let wpos = q_transform.get(entity).unwrap().translation();

        Some(PointerTarget { entity, wpos })
    });
}

fn display_pointer(time: Res<Time>, mut painter: ShapePainter, pointer: Res<PointerPos>) {
    let Some(target) = pointer.pointer_on else {
        return;
    };
    painter.color = Color::RED;
    painter.set_rotation(Quat::default());
    painter.rotate_x(TAU / 4.0);
    painter.rotate_z(time.elapsed_seconds());
    painter.hollow = true;
    painter.set_translation(target.wpos);
    painter.rect(Vec2::splat(1.0));
}
