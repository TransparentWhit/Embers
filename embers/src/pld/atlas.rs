use crate::reg::RegistryInitExt;
use bevy::prelude::*;

pub struct BlockTextureAtlasIndex {}

fn create_atlas(
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //materials.add(StandardMaterial::from(images.add()));
}

pub(super) fn plugin(app: &mut App) {
    app.init_registry::<BlockTextureAtlasIndex>();
}
