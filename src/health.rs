use bevy::prelude::*;
use bevy_vector_shapes::{prelude::ShapePainter, shapes::LinePainter};

use crate::camera::MainCameraTag;

#[derive(Component, Debug)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

// add "amount" to target_entity health
#[derive(Event)]
pub struct ApplyHealthEvent {
    pub amount: i32,
    pub target_entity: Entity,
    pub caster_entity: Entity,
}

// if we have a hitbox as child of an entity with health.
// we can reference the health entity
#[derive(Component)]
pub struct HealthRoot {
    pub entity: Entity,
}

pub struct HealthPlugin;

#[derive(Component)]
pub struct ShowHealthBar;

#[derive(Component)]
pub struct DespawnOnHealth0;

#[derive(Component)]
pub struct DeathSound(pub Handle<AudioSource>);

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ApplyHealthEvent>().add_systems(
            Update,
            (apply_health_events, despawn_0_system, display_health),
        );
    }
}

fn despawn_0_system(query: Query<(&Health, Entity, Option<&DeathSound>)>, mut commands: Commands) {
    for (health, entity, death_sound) in query.iter() {
        if health.is_dead() {
            commands.entity(entity).despawn_recursive();
            if let Some(sound) = death_sound {
                commands.spawn(AudioBundle {
                    source: sound.0.clone(),
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
    }
}

fn apply_health_events(mut events: EventReader<ApplyHealthEvent>, mut query: Query<&mut Health>) {
    for event in events.read() {
        let Ok(mut health) = query.get_mut(event.target_entity) else {
            continue;
        };
        *health += event.amount;
    }
}

fn display_health(
    mut painter: ShapePainter,
    query: Query<(&Health, &GlobalTransform), With<ShowHealthBar>>,
    q_camera: Query<&Transform, With<MainCameraTag>>,
) {
    const HEALTHBAR_LENGTH: f32 = 1.5;

    let camera_tr = q_camera.single();

    for (health, transform) in &query {
        painter.color = Color::GRAY;
        let healthbar_pos = transform.translation() + transform.up() * 4.0;
        let healthbar_left = healthbar_pos - camera_tr.right() * HEALTHBAR_LENGTH / 2.0;
        painter.line(
            healthbar_left,
            healthbar_left + camera_tr.right() * HEALTHBAR_LENGTH,
        );

        let health_ratio = health.current as f32 / health.max as f32;

        painter.color = Color::RED;
        painter.line(
            healthbar_left,
            healthbar_left + camera_tr.right() * HEALTHBAR_LENGTH * health_ratio,
        );
    }
}

impl Health {
    pub fn new(health: i32) -> Self {
        Self {
            current: health,
            max: health,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0
    }

    pub fn percent(&self) -> f32 {
        let percent = self.current as f32 / self.max as f32;
        f32::clamp(percent, 0.0, 1.0)
    }
}

impl std::ops::SubAssign<i32> for Health {
    fn sub_assign(&mut self, rhs: i32) {
        self.current -= rhs;
    }
}

impl std::ops::Sub<i32> for Health {
    type Output = Health;

    fn sub(self, rhs: i32) -> Self::Output {
        Health {
            current: self.current - rhs,
            max: self.max,
        }
    }
}
impl std::ops::AddAssign<i32> for Health {
    fn add_assign(&mut self, rhs: i32) {
        self.current = (self.current + rhs).min(self.max);
    }
}

impl std::ops::Add<i32> for Health {
    type Output = Health;

    fn add(self, rhs: i32) -> Self::Output {
        Health {
            current: (self.current + rhs).min(self.max),
            max: self.max,
        }
    }
}
