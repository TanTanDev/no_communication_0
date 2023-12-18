use crate::{asset_utils::CustomAssetLoaderError, shop::ShopItemData};
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use serde::Deserialize;

pub struct WavePlugin;
impl Plugin for WavePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<WaveDescriptorsAsset>()
            .init_asset_loader::<WavesAssetLoader>()
            .add_systems(Startup, setup_wave_descriptors);
    }
}

#[derive(Resource)]
pub struct WaveDescriptors(pub Handle<WaveDescriptorsAsset>);

fn setup_wave_descriptors(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(WaveDescriptors(asset_server.load("waves.wave.ron")));
}

#[derive(Default)]
pub struct WavesAssetLoader;

#[derive(Debug, Deserialize, Asset, TypePath)]
pub struct WaveDescriptorsAsset(pub Vec<WaveDescriptor>);

impl AssetLoader for WavesAssetLoader {
    type Asset = WaveDescriptorsAsset;
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
            let asset = ron::de::from_bytes::<WaveDescriptorsAsset>(&bytes)?;
            Ok(asset)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wave.ron"]
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct WaveDescriptor {
    pub nb_enemies: usize,
    pub new_shop_items: Vec<ShopItemData>,
}
