use std::ops::RangeInclusive;

use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use heron::*;
use rand::*;

use crate::{WIN_WIDTH, WIN_HEIGHT, PhysLayer, COURT_LINES_Z, COURT_Z, palette::PaletteColor};

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
    asset_server: Res<AssetServer>,
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

    let thickness = 12.;
    let width = x * 2. + thickness;

    let lines = [
        // net
        (0., 5., Vec2::new(thickness * 0.8, height), 0.9),
        // horizonal split
        (0., 0., Vec2::new(width, thickness), 0.),
        // sidelines
        (-x, 0., Vec2::new(thickness, height), 0.),
        (x, 0., Vec2::new(thickness, height), 0.),
        (0., -y, Vec2::new(width, thickness), 0.),
        (0., y, Vec2::new( width, thickness), 0.),
    ];

    for (x, y, size, z_offset) in lines.iter() {
        commands.spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(*x, *y, COURT_LINES_Z + z_offset),
            sprite: Sprite {
                custom_size: Some(*size),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PaletteColor::CourtLines)
        .insert(Name::new("LevelLine"));
    }

    // net shadow
    commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("art-ish/net_post.png"),
        sprite: Sprite {
            custom_size: Some(lines[0].2),
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(-7., -3., COURT_LINES_Z + 0.8),
            scale: Vec3::new(1., 0.97, 1.),
            ..Default::default()
        },
        ..Default::default()
    })
    .insert(PaletteColor::Shadow);

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
            transform: Transform::from_xyz(*x, *y, COURT_Z),
            sprite: Sprite {
                custom_size: Some(region_size.truncate() * 2.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PaletteColor::Court)
        .insert(GlobalTransform::default())
        .insert(RigidBody::Sensor)
        .insert(CollisionShape::Cuboid {
            half_extends: region_size,
            border_radius: None,
        })
        .insert(region.clone())
        .insert(Name::new("Region"));
    }

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::splat(5000.)),
            ..Default::default()
        },
        ..Default::default()
    })
    .insert(PaletteColor::Background);

    let post_offset = 11.;

    for (y, z_offset) in
        [(y + post_offset, 0.5),
        (-y + post_offset, 0.9)].iter() {
            commands.spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/net_post.png"),
                transform: Transform::from_xyz(0., *y, COURT_LINES_Z + z_offset),
                sprite: Sprite {
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(PaletteColor::CourtPost)
            .with_children(|b| {
                b.spawn_bundle(SpriteBundle {
                    texture: asset_server.load("art-ish/net_post.png"),
                    transform: Transform {
                        scale: Vec3::new(1.0, 0.5, 1.),
                        translation: Vec3::new(-3., -17., -0.1),
                        ..Default::default()
                    },
                    sprite: Sprite {
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(PaletteColor::Shadow);
            });
    }

    commands.insert_resource(settings);
}
