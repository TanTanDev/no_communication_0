pub struct KnockbackPlugin;

#[derive(Component)]
pub struct KnockbackRetriever;

impl Plugin for KnockbackPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ApplyHealthEvent>().add_systems(
            Update,
            (apply_health_events, despawn_0_system, display_health),
        );
    }
}
