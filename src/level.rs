use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use heron::*;

use crate::{WIN_WIDTH, WIN_HEIGHT};

pub struct LevelSettings {
    left: f32,
    right: f32,
    height: f32,
}

pub struct LevelPlugin;
impl Plugin for LevelPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup);
    }
}

fn setup(
    mut commands: Commands,
) {
    let x = WIN_WIDTH / 2. - 200.;
    let settings = LevelSettings {
        height: WIN_HEIGHT - 150.,
        left: -x,
        right: x, 
    };

    let thickness = 10.;
    let width = x * 2. + thickness;
    let y = settings.height / 2.;

    let lines = [
        // horizonal split
        (0., 0., Vec2::new(width, thickness), Color::WHITE),
        // net
        (0., 0., Vec2::new(thickness * 1.5, settings.height), Color::BLACK),
        // sidelines
        (-x, 0., Vec2::new(thickness, settings.height), Color::WHITE),
        (x, 0., Vec2::new(thickness, settings.height), Color::WHITE),
        (0., -y, Vec2::new(width, thickness), Color::WHITE),
        (0., y, Vec2::new( width, thickness), Color::WHITE),
    ];

    for (x, y, size, color) in lines.iter() {
        commands.spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(*x, *y, 0.),
            sprite: Sprite {
                color: *color,
                custom_size: Some(*size),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Name::new("LevelLine"));
    }

    commands.insert_resource(settings);
}
