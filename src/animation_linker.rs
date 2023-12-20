use bevy::prelude::*;

pub struct AnimationEntityLinkPlugin;

impl Plugin for AnimationEntityLinkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, link_animations);
    }
}

#[derive(Component)]
pub struct AnimationEntityLink(pub Entity);

///! add this component to stop root parent search, and return this "trap lol"
#[derive(Component)]
pub struct AnimationEntityLinkTrap;

/// OOOOMG lol, sorry :D
///! finds the root top parent entity, and return the first child
fn get_top_parent(
    mut curr_entity: Entity,
    parent_query: &Query<(&Parent, Option<&AnimationEntityLinkTrap>)>,
) -> Entity {
    //Loop up all the way to the top parent
    loop {
        if let Ok((parent, trap)) = parent_query.get(curr_entity) {
            match trap {
                Some(_) => break,
                None => curr_entity = parent.get(),
            }
        } else {
            break;
        }
    }
    curr_entity
}

pub fn link_animations(
    player_query: Query<Entity, Added<AnimationPlayer>>,
    parent_query: Query<(&Parent, Option<&AnimationEntityLinkTrap>)>,
    animations_entity_link_query: Query<&AnimationEntityLink>,
    mut commands: Commands,
) {
    // Get all the Animation players which can be deep and hidden in the heirachy
    for entity in player_query.iter() {
        let top_entity = get_top_parent(entity, &parent_query);

        // If the top parent has an animation config ref then link the player to the config
        if animations_entity_link_query.get(top_entity).is_ok() {
            warn!("Problem with multiple animationsplayers for the same top parent");
        } else {
            commands
                .entity(top_entity)
                .insert(AnimationEntityLink(entity.clone()));
        }
    }
}
