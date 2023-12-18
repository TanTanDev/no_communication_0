use bevy::prelude::*;

pub fn movement_axis(keyboard: &Res<Input<KeyCode>>, left: KeyCode, right: KeyCode) -> f32 {
    match (keyboard.pressed(left), keyboard.pressed(right)) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0f32,
    }
}
