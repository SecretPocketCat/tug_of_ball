use std::{ops::RangeInclusive, time::Duration};

use bevy::{
    math::Vec2,
    prelude::*,
    sprite::{Sprite, SpriteBundle},
};

use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use bevy_tweening::{lens::TransformPositionLens, Animator, EaseFunction, Tween, TweeningType};
use heron::*;
use rand::*;

use crate::{
    palette::PaletteColor, score::Score, PhysLayer, TransformBundle, COURT_LINE_Z, COURT_Z, NET_Z,
    SHADOW_Z, WIN_HEIGHT, WIN_WIDTH,
};

#[derive(Component)]
pub struct Net;

pub struct NetOffset(pub f32);

pub struct CourtSettings {
    // nice2have: replace by proper bounds
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) top: f32,
    pub(crate) bottom: f32,
    pub(crate) base_region_size: Vec3,
    pub(crate) region_x: f32,
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

#[allow(dead_code)]
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
        if self.is_left() {
            1
        } else {
            2
        }
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
        app.insert_resource(NetOffset(0.))
            .add_startup_system(setup)
            .add_system(handle_net_offset);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let x = WIN_WIDTH / 2. - 300.;
    let height = WIN_HEIGHT - 250.;
    let y = height / 2.;
    let thickness = 12.;
    let width = x * 2. + thickness;
    let region_x = x / 2. + thickness / 4.;
    let region_y = y / 2. + thickness / 4.;
    let region_size = Vec3::new(width / 4., height / 4. + thickness / 4., 0.);

    let settings = CourtSettings {
        left: -x,
        right: x,
        top: y,
        bottom: -y,
        base_region_size: region_size,
        region_x,
    };

    let lines = [
        // horizonal split
        (0., 0., Vec2::new(width, thickness), COURT_LINE_Z),
        // sidelines
        (-x, 0., Vec2::new(thickness, height), COURT_LINE_Z),
        (x, 0., Vec2::new(thickness, height), COURT_LINE_Z),
        (0., -y, Vec2::new(width, thickness), COURT_LINE_Z),
        (0., y, Vec2::new(width, thickness), COURT_LINE_Z),
    ];

    for (x, y, size, z) in lines.iter() {
        commands
            .spawn_bundle(SpriteBundle {
                transform: Transform::from_xyz(*x, *y, *z),
                sprite: Sprite {
                    custom_size: Some(*size),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(PaletteColor::CourtLines)
            .insert(Name::new("LevelLine"));
    }

    let colliders = [
        (-region_x, region_y, CourtRegion::TopLeft, Color::ORANGE),
        (
            -region_x,
            -region_y,
            CourtRegion::BottomLeft,
            Color::ALICE_BLUE,
        ),
        (region_x, region_y, CourtRegion::TopRight, Color::GREEN),
        (
            region_x,
            -region_y,
            CourtRegion::BottomRight,
            Color::FUCHSIA,
        ),
    ];

    for (x, y, region, debug_col) in colliders.iter() {
        spawn_region(&mut commands, *region, *x, *y, region_size, *debug_col);
    }

    // net
    let net_size = Vec2::new(thickness * 0.8, height);
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_xyz(0., 5., NET_Z),
            sprite: Sprite {
                custom_size: Some(net_size),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PaletteColor::CourtLines)
        .insert(Net)
        .insert(Name::new("Net"))
        .with_children(|b| {
            // shadow
            b.spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/net_post.png"),
                sprite: Sprite {
                    custom_size: Some(net_size),
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(-7., -3., -NET_Z + SHADOW_Z),
                    scale: Vec3::new(1., 0.97, 1.),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(PaletteColor::Shadow);

            // posts
            let post_offset = 11.;
            for (y, z_offset) in [(y + post_offset, -0.1), (-y + post_offset, 0.1)].iter() {
                let z = NET_Z + z_offset;
                b.spawn_bundle(SpriteBundle {
                    texture: asset_server.load("art-ish/net_post.png"),
                    transform: Transform::from_xyz(0., *y, z),
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
                            translation: Vec3::new(-3., -17., -z + SHADOW_Z),
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
        });

    // cheeky bg - maybe just set for camera?
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::splat(5000.)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PaletteColor::Background);

    commands.insert_resource(settings);
}

fn spawn_region(
    commands: &mut Commands,
    region: CourtRegion,
    x: f32,
    y: f32,
    region_size: Vec3,
    debug_color: Color,
) {
    commands
        .spawn_bundle(TransformBundle::from_xyz(x, y, COURT_Z))
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Cuboid {
            half_extends: region_size,
            border_radius: None,
        })
        .insert(CollisionLayers::all::<PhysLayer>())
        .insert(region.clone())
        .insert(Name::new("Region"))
        .with_children(|b| {
            #[cfg(feature = "debug")]
            {
                let mut col = debug_color;
                col.set_a(0.3);
                b.spawn_bundle(SpriteBundle {
                    // transform: Transform::from_xyz(*x, *y, COURT_Z + 0.5),
                    sprite: Sprite {
                        custom_size: Some(region_size.truncate() * 2.),
                        color: col,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Name::new("RegionDebug"));
            }
        });
}

fn handle_net_offset(
    mut commands: Commands,
    score: Res<Score>,
    mut offset: ResMut<NetOffset>,
    net_q: Query<(Entity, &Transform), With<Net>>,
    mut region_q: Query<(Entity, &CourtRegion, &mut Transform, &mut CollisionShape), Without<Net>>,
    settings: Res<CourtSettings>,
) {
    if score.is_changed() {
        let offset_mult = -50.;
        offset.0 = (score.right_player.games as f32 - score.left_player.games as f32) * offset_mult;

        #[cfg(feature = "debug")]
        {
            offset.0 =
                (score.right_player.points as f32 - score.left_player.points as f32) * offset_mult;
        }

        // tween net
        if let Ok((net_e, net_t)) = net_q.get_single() {
            commands.entity(net_e).insert(Animator::new(Tween::new(
                EaseFunction::QuadraticInOut,
                TweeningType::Once,
                Duration::from_millis(400),
                TransformPositionLens {
                    start: net_t.translation,
                    end: Vec3::new(offset.0, net_t.translation.y, net_t.translation.z),
                },
            )));
        }

        // resize regions
        for (region_e, region, mut region_t, mut region_coll_shape) in region_q.iter_mut() {
            let x = if region.is_left() {
                -settings.region_x + offset.0 / 2.
            } else {
                settings.region_x + offset.0 / 2.
            };
            let side_mult = if region.is_left() { 1. } else { -1. };
            let mut extends = settings.base_region_size;
            extends.x += (offset.0 / 2.) * side_mult;
            spawn_region(
                &mut commands,
                *region,
                x,
                region_t.translation.y,
                extends,
                Color::NONE,
            );

            commands.entity(region_e).despawn_recursive();
        }
    }
}
