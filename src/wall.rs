use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use heron::*;

use crate::{WIN_WIDTH, WIN_HEIGHT};

#[derive(Default, Component, Inspectable)]
pub struct Wall;

pub struct WallPlugin;
impl Plugin for WallPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup);
    }
}

fn setup(
    mut commands: Commands,
) {
    let thickness = 30.;
    let x = WIN_WIDTH / 2. - thickness / 2.;
    let y = WIN_HEIGHT / 2. - thickness / 2.;

    let walls = [
        (-x, 0., Vec2::new(thickness, WIN_HEIGHT)),
        (x, 0., Vec2::new(thickness, WIN_HEIGHT)),
        (0., -y, Vec2::new(WIN_WIDTH, thickness)),
        (0., y, Vec2::new( WIN_WIDTH, thickness)),
    ];

    for (x, y, size) in walls.iter() {
        commands.spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(*x, *y, 0.),
            sprite: Sprite {
                color: Color::GRAY,
                custom_size: Some(*size),
                ..Default::default()
            },
            ..Default::default()
        }).insert(Wall)
        .insert(RigidBody::Static)
        .insert(CollisionShape::Cuboid {
            half_extends: (*size).to_vec3() / 2.,
            border_radius: None,
        })
        .insert(Name::new("Wall"));
    }

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(10., WIN_HEIGHT)),
            ..Default::default()
        },
        ..Default::default()
    })//.insert(Wall)
    // .insert(RigidBody::Static)
    // .insert(CollisionShape::Cuboid {
    //     half_extends: (*size).to_vec3() / 2.,
    //     border_radius: None,
    // })
    .insert(Name::new("Net"));
}
