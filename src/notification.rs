use bevy::prelude::*;

use crate::ui_util::UiAssets;

pub struct NotificationPlugin;

impl Plugin for NotificationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NotificationEvent>()
            .add_systems(Startup, ui_setup)
            .add_systems(Update, (spawn_notifications, despawn_notifications));
    }
}

#[derive(Event)]
pub struct NotificationEvent {
    pub text: String,
    /// Seconds to show for
    pub show_for: f32,
    pub color: Color,
}

#[derive(Component)]
struct NotificationUiTag;

#[derive(Component)]
struct Notification {
    time_left: f32,
}

fn ui_setup(mut commands: Commands) {
    commands.spawn((
        NotificationUiTag,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        },
    ));
}

fn spawn_notifications(
    mut commands: Commands,
    ui_assets: Res<UiAssets>,
    mut notification_event: EventReader<NotificationEvent>,
    node: Query<Entity, With<NotificationUiTag>>,
) {
    let node = node.single();
    for notification in notification_event.read() {
        commands
            .spawn((
                Notification {
                    time_left: notification.show_for,
                },
                TextBundle::from_section(
                    &notification.text,
                    TextStyle {
                        font: ui_assets.font.clone(),
                        font_size: 60.0,
                        color: notification.color,
                    },
                ),
            ))
            .set_parent(node);
    }
}

fn despawn_notifications(
    mut commands: Commands,
    time: Res<Time>,
    mut notifications: Query<(Entity, &mut Notification, &mut Text)>,
) {
    const FADE_AT: f32 = 0.6;
    for (entity, mut notification, mut text) in notifications.iter_mut() {
        notification.time_left -= time.delta_seconds();
        if notification.time_left <= 0.0 {
            commands.entity(entity).despawn_recursive();
        } else if notification.time_left <= FADE_AT {
            let t = notification.time_left / FADE_AT;
            // Ease out
            let fade = 1.0 - (1.0 - t).powi(3);
            for section in text.sections.iter_mut() {
                section.style.color = section.style.color.with_a(fade);
            }
        }
    }
}
