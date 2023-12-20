use crate::health::ApplyHealthEvent;
use bevy::prelude::*;
use bevy_rapier3d::dynamics::Velocity;

pub struct KnockbackPlugin;

#[derive(Component)]
pub struct KnockbackRetriever;

impl Plugin for KnockbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_knockback_on_health_event);
    }
}

fn apply_knockback_on_health_event(
    mut events: EventReader<ApplyHealthEvent>,
    mut query: Query<(&mut Velocity, &Transform)>,
) {
    for event in events.read() {
        let Ok((_bd, transform)) = query.get_mut(event.caster_entity) else {
            continue;
        };
        let caster_pos = transform.translation;
        let Ok((mut bd, transform)) = query.get_mut(event.target_entity) else {
            continue;
        };
        let target_pos = transform.translation;
        let to = (caster_pos - target_pos).normalize();
        bd.linvel -= to * 20.0;
        bd.linvel.y = 7.0;
    }
}
