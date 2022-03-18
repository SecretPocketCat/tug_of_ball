use crate::{
    extra::TransformBundle,
    palette::PaletteColor,
    physics::PhysLayer,
    render::{COURT_LINE_Z, COURT_Z, NET_Z, SHADOW_Z},
    reset::Persistent,
    score::{GameOverEvt, ScoreChangeType, ScoreChangedEvt, NET_OFFSET_GAME, NET_OFFSET_POINT},
    GameState, BASE_VIEW_HEIGHT, BASE_VIEW_WIDTH,
};
use bevy::{
    math::Vec2,
    prelude::*,
    sprite::{Sprite, SpriteBundle},
};
use bevy_inspector_egui::Inspectable;
use bevy_prototype_lyon::prelude::*;
use bevy_tweening::{lens::TransformPositionLens, Animator, EaseFunction, Tween, TweeningType};
use heron::*;
use rand::*;
use std::{ops::RangeInclusive, time::Duration};

pub struct LevelPlugin;
impl Plugin for LevelPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<NetOffset>()
            .add_startup_system(setup)
            .add_system(draw_court)
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(handle_net_offset));
    }
}

#[derive(Component)]
pub struct Net;

#[derive(Default)]
pub struct NetOffset {
    pub target: f32,
    pub current_offset: f32,
    pub reset_queued: bool,
}

impl NetOffset {
    pub fn reset(&mut self) {
        self.current_offset = 0.;
        self.target = 0.;
        self.reset_queued = false;
    }
}

#[derive(Component)]
pub struct Court;

#[derive(Component)]
pub struct InitialRegion(pub CourtRegion);

pub struct ServingRegion(pub CourtRegion);

#[derive(Default)]
pub struct CourtSettings {
    // nice2have: replace by proper bounds
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub base_region_size: Vec3,
    pub region_x: f32,
    pub view: Vec2,
    pub win_treshold: f32,
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let x = BASE_VIEW_WIDTH / 2. - 300.;
    let height = BASE_VIEW_HEIGHT - 320.;
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
        win_treshold: x / 2.,
        ..Default::default()
    };

    let lines = [
        // horizonal split
        (0., 0., Vec2::new(width - 10., thickness), COURT_LINE_Z),
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
            .insert(Name::new("LevelLine"))
            .insert(Persistent);
    }

    let colliders = [
        (-region_x, region_y, CourtRegion::TopLeft),
        (-region_x, -region_y, CourtRegion::BottomLeft),
        (region_x, region_y, CourtRegion::TopRight),
        (region_x, -region_y, CourtRegion::BottomRight),
    ];

    for (x, y, region) in colliders.iter() {
        spawn_region(&mut commands, *region, *x, *y, region_size);
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
        .insert(Persistent)
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
                b.spawn_bundle(SpriteBundle {
                    texture: asset_server.load("art-ish/net_post.png"),
                    transform: Transform::from_xyz(0., *y, *z_offset),
                    sprite: Sprite {
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(PaletteColor::CourtPost)
                .with_children(|b| {
                    let z = NET_Z + z_offset;
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

    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &PathBuilder::new().build().0,
            DrawMode::Fill(FillMode::color(Color::rgb_u8(32, 40, 61))),
            Transform::from_xyz(0., 0., COURT_Z),
        ))
        .insert(Court)
        .insert(Persistent);

    // dashed tug lines
    let dash_line_x = settings.win_treshold;
    for x in [-dash_line_x, dash_line_x].iter() {
        commands
            .spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/stroke.png"),
                transform: Transform::from_xyz(*x, 0., COURT_LINE_Z - 0.1),
                sprite: Sprite {
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(PaletteColor::CourtPost)
            .insert(Persistent);
    }

    // cheeky bg - maybe just set for camera?
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::splat(8000.)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PaletteColor::Background)
        .insert(Persistent);

    commands.insert_resource(settings);
}

fn draw_court(mut court_q: Query<&mut Path, With<Court>>, court: Res<CourtSettings>) {
    if court.is_added() || court.is_changed() {
        for mut path in court_q.iter_mut() {
            trace!("drawing court");
            let mut path_builder = PathBuilder::new();
            let radius = 20.;
            let top_l = Vec2::new(court.left, court.top);
            let top_r = Vec2::new(court.right, court.top);
            let btm_l = Vec2::new(court.left, court.bottom);
            let btm_r = Vec2::new(court.right, court.bottom);
            path_builder.move_to(top_r - Vec2::X * radius);
            path_builder.quadratic_bezier_to(top_r, top_r - Vec2::Y * radius);
            path_builder.line_to(btm_r + Vec2::Y * radius);
            path_builder.quadratic_bezier_to(btm_r, btm_r - Vec2::X * radius);
            path_builder.line_to(btm_l + Vec2::X * radius);
            path_builder.quadratic_bezier_to(btm_l, btm_l + Vec2::Y * radius);
            path_builder.line_to(top_l - Vec2::Y * radius);
            path_builder.quadratic_bezier_to(top_l, top_l + Vec2::X * radius);

            path_builder.close();
            let shape = path_builder.build();
            path.0 = shape.0;
        }
    }
}

fn spawn_region(commands: &mut Commands, region: CourtRegion, x: f32, y: f32, region_size: Vec3) {
    commands
        .spawn_bundle(TransformBundle::from_xyz(x, y, COURT_Z))
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Cuboid {
            half_extends: region_size,
            border_radius: None,
        })
        .insert(CollisionLayers::all::<PhysLayer>())
        .insert(region)
        .insert(Name::new("Region"))
        .insert(Persistent);
}

fn handle_net_offset(
    mut score_ev_r: EventReader<ScoreChangedEvt>,
    mut game_over_ev_w: EventWriter<GameOverEvt>,
    mut commands: Commands,
    mut net: ResMut<NetOffset>,
    court: Res<CourtSettings>,
    net_q: Query<(Entity, &Transform), With<Net>>,
    mut region_q: Query<(Entity, &CourtRegion, &mut Transform, &mut CollisionShape), Without<Net>>,
    settings: Res<CourtSettings>,
) {
    if let Ok((net_e, net_t)) = net_q.get_single() {
        let mut target_offset = 0.;

        for ev in score_ev_r.iter() {
            let mut offset = match ev.score_type {
                ScoreChangeType::Point => NET_OFFSET_POINT,
                ScoreChangeType::Game => NET_OFFSET_GAME,
            };

            if !ev.left_side_scored {
                offset *= -1.;
            }

            target_offset += offset;
        }

        if target_offset != 0. || net.reset_queued {
            if net.reset_queued {
                net.reset();
            } else {
                net.target += target_offset;
            }

            // tween net
            commands.entity(net_e).insert(Animator::new(Tween::new(
                EaseFunction::QuadraticInOut,
                TweeningType::Once,
                Duration::from_millis(400),
                TransformPositionLens {
                    start: net_t.translation,
                    end: Vec3::new(net.target, net_t.translation.y, net_t.translation.z),
                },
            )));

            if net.target.abs() > court.win_treshold {
                game_over_ev_w.send(GameOverEvt {
                    left_has_won: net.target > 0.,
                });
            } else {
                // resize regions
                for (region_e, region, region_t, _region_coll_shape) in region_q.iter_mut() {
                    let x = if region.is_left() {
                        -settings.region_x + net.target / 2.
                    } else {
                        settings.region_x + net.target / 2.
                    };
                    let side_mult = if region.is_left() { 1. } else { -1. };
                    let mut extends = settings.base_region_size;
                    extends.x += (net.target / 2.) * side_mult;
                    spawn_region(&mut commands, *region, x, region_t.translation.y, extends);

                    commands.entity(region_e).despawn_recursive();
                }
            }
        }

        net.current_offset = net_t.translation.x;
    }
}
