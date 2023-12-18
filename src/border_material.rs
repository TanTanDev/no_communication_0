//! Demonstrates using a custom extension to the `StandardMaterial` to modify the results of the builtin pbr shader.

use bevy::{
    asset::Asset,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

pub struct BorderMaterialPlugin;

impl Plugin for BorderMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, BorderMaterial>,
        >::default());
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct BorderMaterial {
    // We need to ensure that the bindings of the base material and the extension do not conflict,
    // so we start from binding slot 100, leaving slots 0-99 for the base material.
    #[uniform(100)]
    pub quantize_steps: u32,
    #[texture(101)]
    #[sampler(102)]
    pub color_texture: Handle<Image>,
}

impl MaterialExtension for BorderMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/border_material.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/border_material.wgsl".into()
    }
}
