use bevy::{prelude::*, render::render_resource::FilterMode};

pub struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(set_img_sampler_filter);
    }
}

fn set_img_sampler_filter(
    mut ev_asset: EventReader<AssetEvent<Image>>,
    mut assets: ResMut<Assets<Image>>,
) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(mut texture) = assets.get_mut(handle) {
                    // set sampler filtering to add some AA (quite fuzzy though)
                    texture.sampler_descriptor.mag_filter = FilterMode::Linear;
                    texture.sampler_descriptor.min_filter = FilterMode::Linear;
                }
            }
            _ => {}
        }
    }
}
