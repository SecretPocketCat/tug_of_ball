use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
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

    let region_x = x / 2. + thickness / 4.;
    let region_y = y / 2. + thickness / 4.;
    let region_size = Vec3::new(width / 4. + thickness / 4., settings.height / 4. + thickness / 4., 0.);
    let sensors = [
        ("Top Left Region", -region_x, region_y),
        ("Bottom Left Region", -region_x, -region_y),
        ("Top Right Region", region_x, region_y),
        ("Bottom Right Region", region_x, -region_y),
    ];

    for (name, x, y) in sensors.iter() {
        commands.spawn()
            .insert(Transform::from_xyz(*x, *y, 0.))
            .insert(GlobalTransform::default())
            .insert(RigidBody::Sensor)
            .insert(CollisionShape::Cuboid {
                half_extends: region_size,
                border_radius: None,
            })
            .insert(Name::new(*name));
    }

    // bounds region
    commands.spawn()
        .insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(RigidBody::Sensor)
        .insert(CollisionShape::Cuboid {
            half_extends: Vec3::new(x + thickness / 2., y + thickness / 2., 0.),
            border_radius: None,
        })
        .insert(Name::new("Bounds region"));

    commands.insert_resource(settings);
}
