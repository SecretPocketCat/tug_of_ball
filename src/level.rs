use std::ops::RangeInclusive;

use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use heron::*;
use rand::*;

use crate::{WIN_WIDTH, WIN_HEIGHT, PhysLayer};

pub struct CourtSettings {
    // nice2have: replace by proper bounds
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
}

#[derive(Default, Component, Inspectable, Clone, Copy, Debug, PartialEq)]
pub enum CourtRegion {
    #[default]
    OutOfBounds,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl CourtRegion {
    pub fn is_left(&self) -> bool {
        *self == CourtRegion::BottomLeft || *self == CourtRegion::TopLeft
    }

    pub fn is_right(&self) -> bool {
        *self == CourtRegion::BottomRight || *self == CourtRegion::TopRight
    }

    pub fn is_top(&self) -> bool {
        *self == CourtRegion::TopLeft || *self == CourtRegion::TopRight
    }

    pub fn is_bottom(&self) -> bool {
        *self == CourtRegion::BottomRight || *self == CourtRegion::BottomLeft
    }

    pub fn is_out_of_bounds(&self) -> bool {
        *self == CourtRegion::OutOfBounds
    }

    pub fn get_inverse(&self) -> Option<Self> {
        match self {
            CourtRegion::OutOfBounds => None,
            CourtRegion::TopLeft => Some(Self::BottomRight),
            CourtRegion::TopRight => Some(Self::BottomLeft),
            CourtRegion::BottomLeft => Some(Self::TopRight),
            CourtRegion::BottomRight => Some(Self::TopLeft),
        }
    }

    pub fn get_player_id(&self) -> usize {
        if self.is_left() { 1 } else { 2 }
    }

    pub fn get_random() -> Self {
        Self::get_random_from_range(0..=3)
    }

    pub fn get_random_left() -> Self {
        Self::get_random_from_range(0..=1)
    }
    
    pub fn get_random_right() -> Self {
        Self::get_random_from_range(2..=3)
    }

    pub fn get_random_from_range(range: RangeInclusive<usize>) -> Self {
        let mut rng = rand::thread_rng();
        [
            CourtRegion::TopLeft,
            CourtRegion::BottomLeft,
            CourtRegion::TopRight,
            CourtRegion::BottomRight,
        ][rng.gen_range(range)]
    }

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
    let x = WIN_WIDTH / 2. - 300.;
    let height = WIN_HEIGHT - 250.;
    let y = height / 2.;

    let settings = CourtSettings {
        left: -x,
        right: x,
        top: y,
        bottom: -y,
    };

    let thickness = 10.;
    let width = x * 2. + thickness;

    let lines = [
        // horizonal split
        (0., 0., Vec2::new(width, thickness), Color::WHITE),
        // net
        (0., 0., Vec2::new(thickness * 1.5, height), Color::BLACK),
        // sidelines
        (-x, 0., Vec2::new(thickness, height), Color::WHITE),
        (x, 0., Vec2::new(thickness, height), Color::WHITE),
        (0., -y, Vec2::new(width, thickness), Color::WHITE),
        (0., y, Vec2::new( width, thickness), Color::WHITE),
    ];

    for (x, y, size, color) in lines.iter() {
        commands.spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(*x, *y, 1.),
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
    let region_size = Vec3::new(width / 4., height / 4. + thickness / 4., 0.);
    let sensors = [
        (-region_x, region_y, CourtRegion::TopLeft),
        (-region_x, -region_y, CourtRegion::BottomLeft),
        (region_x, region_y, CourtRegion::TopRight),
        (region_x, -region_y, CourtRegion::BottomRight),
    ];

    for (x, y, region) in sensors.iter() {
        commands.spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(*x, *y, 0.),
            sprite: Sprite {
                color: Color::rgb_u8(170, 200, 55),
                custom_size: Some(region_size.truncate() * 2.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(GlobalTransform::default())
        .insert(RigidBody::Sensor)
        .insert(CollisionShape::Cuboid {
            half_extends: region_size,
            border_radius: None,
        })
        .insert(region.clone())
        .insert(Name::new("Region"));
    }

    // // bounds region
    // commands.spawn()
    //     .insert(Transform::default())
    //     .insert(GlobalTransform::default())
    //     .insert(RigidBody::Sensor)
    //     .insert(CollisionShape::Cuboid {
    //         half_extends: Vec3::new(x + thickness / 2., y + thickness / 2., 0.),
    //         border_radius: None,
    //     })
    //     .insert(Name::new("Bounds region"));

    commands.insert_resource(settings);
}
