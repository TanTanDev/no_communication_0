use bevy::prelude::*;

pub struct UiUtilPlugin;

impl Plugin for UiUtilPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiAssets>().add_systems(
            PostUpdate,
            (remove_just_clicked, update_button_color).chain(),
        );
    }
}

#[derive(Resource)]
pub struct UiAssets {
    pub font: Handle<Font>,
}

impl FromWorld for UiAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server: &AssetServer = world.resource();

        Self {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        }
    }
}

// Marker component for a button that was just clicked.
#[derive(Component)]
pub struct JustClicked;

#[derive(Component, Debug)]
struct LastInteraction(Interaction);

pub const DEFAULT_BUTTON_COLOR: Color = Color::rgba(1.0, 0.9, 1.0, 0.5);
pub const BUTTON_HOVER_COLOR: Color = Color::rgba(0.7, 0.6, 0.7, 0.5);
pub const BUTTON_PRESS_COLOR: Color = Color::rgba(0.5, 0.45, 0.5, 0.5);

#[derive(Component)]
pub struct ButtonColor(pub Color);

// We want this to be applied directly, and not at end of frame like command would,
// so we use an exclusive system.
fn remove_just_clicked(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, (With<Button>, With<Button>)>()
        .iter(world)
        .collect();

    for entity in entities {
        world.entity_mut(entity).remove::<JustClicked>();
    }
}

fn update_button_color(
    mut commands: Commands,
    mut buttons: Query<
        (
            Entity,
            &mut BackgroundColor,
            &Interaction,
            Option<&LastInteraction>,
            Option<&ButtonColor>,
        ),
        (With<Button>, Changed<Interaction>),
    >,
) {
    for (entity, mut color, interaction, last_interaction, button_color) in buttons.iter_mut() {
        match interaction {
            Interaction::Pressed => color.0 = BUTTON_PRESS_COLOR,
            Interaction::Hovered => {
                if matches!(
                    last_interaction,
                    Some(LastInteraction(Interaction::Pressed))
                ) {
                    commands.entity(entity).insert(JustClicked);
                }
                color.0 = BUTTON_HOVER_COLOR
            }
            Interaction::None => {
                color.0 = button_color.map(|c| c.0).unwrap_or(DEFAULT_BUTTON_COLOR)
            }
        }
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.insert(LastInteraction(*interaction));
        }
    }
}
