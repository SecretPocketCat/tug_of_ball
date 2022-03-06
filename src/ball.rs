use std::time::Duration;

use bevy::{
    math::Vec2,
    prelude::*,
    sprite::{Sprite, SpriteBundle},
};
use bevy_extensions::Vec2Conversion;

use bevy_inspector_egui::Inspectable;
use bevy_prototype_lyon::prelude::*;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::{SpriteColorLens, TransformScaleLens};
use bevy_tweening::*;
use heron::*;
use rand::*;

use crate::{
    animation::TweenDoneAction,
    extra::TransformBundle,
    input::PlayerInput,
    level::{CourtRegion, CourtSettings, InitialRegion, NetOffset, ServingRegion},
    palette::{Palette, PaletteColor},
    physics::PhysLayer,
    player::{Player, PlayerAim, PlayerSwing},
    player_action::ActionStatus,
    render::{BALL_Z, PLAYER_Z, SHADOW_Z},
    trail::{FadeOutTrail, Trail},
};

const BALL_SIZE: f32 = 35.;

pub struct BallPlugin;
impl Plugin for BallPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system_to_stage(StartupStage::PostStartup, setup)
            .add_system(movement)
            .add_system(bounce)
            .add_system_to_stage(CoreStage::PostUpdate, handle_collisions)
            .add_system_to_stage(CoreStage::PostUpdate, handle_regions)
            .add_event::<BallBouncedEvt>();
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    region: Res<InitialRegion>,
    court_set: Res<CourtSettings>,
) {
    spawn_ball(
        &mut commands,
        &asset_server,
        region.0,
        0,
        region.0.get_player_id(),
        &court_set,
    );
    commands.insert_resource(ServingRegion(region.0));
}

#[derive(Default, Component, Inspectable)]
pub struct Ball {
    dir: Vec2,
    size: f32,
    speed: f32,
    prev_pos: Vec3,
    pub region: CourtRegion,
    bounce_e: Option<Entity>,
    pub trail_e: Option<Entity>,
}

#[derive(Default, Component, Inspectable)]
pub struct BallBounce {
    gravity: f32,
    velocity: f32,
    max_velocity: f32,
    count: usize,
}

#[derive(Default, Component, Inspectable)]
pub enum BallStatus {
    Serve(CourtRegion, u8, usize),
    Fault(u8, usize),
    Rally(usize),
    #[default]
    Used,
}

pub struct BallBouncedEvt {
    pub ball_e: Entity,
    pub bounce_count: usize,
    pub side: f32,
}

// nice2have: try - slowly speedup during rally?
fn movement(
    mut ball_q: Query<(&mut Ball, &mut Transform)>,
    mut bounce_q: Query<&mut BallBounce>,
    time: ScaledTime,
    net: Res<NetOffset>,
) {
    for (mut ball, mut ball_t) in ball_q.iter_mut() {
        if ball.dir == Vec2::ZERO {
            continue;
        }

        let speed = ball.dir.length();

        if speed < 0.025 {
            ball.dir = Vec2::ZERO;
            return;
        }

        // very simple drag
        let drag_mult = if speed < 0.25 { 1. } else { 0.35 };
        ball.dir *= 1. - drag_mult * time.scaled_delta_seconds();

        // move
        ball_t.translation += ball.dir.to_vec3() * ball.speed * time.scaled_delta_seconds();

        let net_x = net.0;
        let ball_x = ball_t.translation.x;
        let ball_prev_x = ball.prev_pos.x;
        if (ball_prev_x < net_x && ball_x > net_x) || (ball_prev_x > net_x && ball_x < net_x) {
            if let Ok(mut bounce) = bounce_q.get_mut(ball.bounce_e.unwrap()) {
                bounce.count = 0;
                info!("crossed net extra check");
            }
        }

        ball.prev_pos = ball_t.translation;
    }
}

fn get_bounce_velocity(dir_len: f32, max_velocity: f32) -> f32 {
    dir_len.sqrt().min(1.) * max_velocity
}

fn bounce(
    mut bounce_query: Query<
        (&mut BallBounce, &mut Transform, &GlobalTransform, &Parent),
        Without<Ball>,
    >,
    mut ball_q: Query<(Entity, &mut Ball, &mut BallStatus, &Transform)>,
    mut ev_w_bounce: EventWriter<BallBouncedEvt>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    palette: Res<Palette>,
    time: ScaledTime,
    net: Res<NetOffset>,
) {
    for (mut ball_bounce, mut t, _bounce_global_t, p) in bounce_query.iter_mut() {
        if let Ok((ball_e, ball, mut ball_status, ball_t)) = ball_q.get_mut(p.0) {
            if ball.dir == Vec2::ZERO {
                continue;
            }

            ball_bounce.velocity += ball_bounce.gravity * time.scaled_delta_seconds();
            t.translation.y += ball_bounce.velocity * time.scaled_delta_seconds();

            if t.translation.y <= 0. {
                t.translation.y = 0.01;
                ball_bounce.velocity =
                    get_bounce_velocity(ball.dir.length(), ball_bounce.max_velocity);
                ball_bounce.count += 1;
                trace!("Bounce {}", ball_bounce.count);

                // eval serve on bounce
                if let BallStatus::Serve(region, fault_count, player_id) = *ball_status {
                    if ball.region != region.get_inverse().unwrap() {
                        // fault
                        *ball_status = BallStatus::Fault(fault_count + 1, player_id);
                        debug!("Bad serve {:?} => {:?}", region, ball.region);
                    } else {
                        // good serve
                        *ball_status = BallStatus::Rally(player_id);
                        debug!("Good serve {:?} => {:?}", region, ball.region);
                    }
                }

                ev_w_bounce.send(BallBouncedEvt {
                    ball_e,
                    bounce_count: ball_bounce.count,
                    side: if ball_t.translation.x < net.0 {
                        -1.
                    } else {
                        1.
                    },
                });

                spawn_bounce_track(
                    &mut commands,
                    &asset_server,
                    &palette,
                    ball_t.translation.truncate().extend(SHADOW_Z),
                );
                debug!("Bounced {} times", ball_bounce.count);
            }
        }
    }
}

fn spawn_bounce_track(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    palette: &Res<Palette>,
    pos: Vec3,
) {
    let end_col = palette.get_color(&PaletteColor::Shadow);
    let tween = Tween::new(
        EaseFunction::QuadraticOut,
        TweeningType::Once,
        Duration::from_millis(1500),
        SpriteColorLens {
            start: end_col,
            end: Color::NONE,
        },
    )
    .with_completed_event(true, TweenDoneAction::DespawnRecursive.into());

    commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("art-ish/ball.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(1.0, 0.5) * BALL_SIZE),
                color: Color::NONE,
                ..Default::default()
            },
            transform: Transform {
                translation: pos + Vec3::new(-3., -14., 0.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Animator::new(tween));
}

// nice2have: 'auto dash swing'?
fn handle_collisions(
    mut coll_events: EventReader<CollisionEvent>,
    _input: Res<PlayerInput>,
    mut ball_q: Query<(&mut Ball, &mut BallStatus, &Children)>,
    mut ball_bounce_q: Query<&mut BallBounce>,
    player_aim_q: Query<&PlayerAim>,
    mut player_q: Query<(&Player, &mut PlayerSwing, &GlobalTransform)>,
) {
    for ev in coll_events.iter() {
        if ev.is_started() {
            let mut ball;
            let mut status;
            let other_e;
            let bounce_e;
            let (entity_1, entity_2) = ev.rigid_body_entities();
            if let Ok(b) = ball_q.get_mut(entity_1) {
                ball = b.0;
                status = b.1;
                bounce_e = b.2.iter().next().unwrap();
                other_e = entity_2;
            } else if let Ok(b) = ball_q.get_mut(entity_2) {
                ball = b.0;
                status = b.1;
                bounce_e = b.2.iter().next().unwrap();
                other_e = entity_1;
            } else {
                continue;
            }

            let mut ball_bounce = ball_bounce_q.get_mut(*bounce_e).unwrap();

            if let Ok((player, mut swing, _player_t)) = player_q.get_mut(other_e) {
                if let ActionStatus::Active(ball_speed_multiplier) = swing.status {
                    if !swing.timer.finished() {
                        swing.start_cooldown();

                        if let Ok(aim) = player_aim_q.get(player.aim_e) {
                            let mut dir = aim.direction;

                            let clamp_x = 1.;
                            let clamp_y = 0.8;

                            let player_sign = player.get_sign();
                            if dir == Vec2::new(player_sign, 0.) {
                                // player aiming into their court/backwards - just aim straight
                                dir = Vec2::new(-player_sign, 0.);
                            } else if player.is_left() {
                                dir = dir.clamp(
                                    Vec2::new(clamp_x, -clamp_y),
                                    Vec2::new(clamp_x, clamp_y),
                                );
                            } else {
                                dir = dir.clamp(
                                    Vec2::new(-clamp_x, -clamp_y),
                                    Vec2::new(-clamp_x, clamp_y),
                                );
                            }

                            ball.dir = dir * ball_speed_multiplier;
                            ball_bounce.velocity =
                                get_bounce_velocity(dir.length(), ball_bounce.max_velocity);

                            let rot = Quat::from_rotation_arc_2d(Vec2::Y, dir)
                                .to_euler(EulerRot::XYZ)
                                .2
                                .to_degrees();
                            trace!("Hit rot {:?}", rot);

                            match *status {
                                BallStatus::Serve(_, _, player_id) if player_id != player.id => {
                                    // vollied serve
                                    *status = BallStatus::Rally(player.id);
                                    trace!("Vollied serve");
                                }
                                BallStatus::Rally(..) => {
                                    // set rally player on hit, also applies to
                                    *status = BallStatus::Rally(player.id);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn handle_regions(
    mut commands: Commands,
    mut coll_events: EventReader<CollisionEvent>,
    ball_q: Query<(Entity, &GlobalTransform), With<Ball>>,
    mut ball_mut_q: Query<&mut Ball>,
    mut ball_bounce_q: Query<(&mut BallBounce, &Transform)>,
    region_q: Query<&CourtRegion>,
    court_set: Res<CourtSettings>,
    entity_q: Query<Entity, Without<Ball>>,
) {
    let all_events: Vec<CollisionEvent> = coll_events.iter().cloned().collect();
    for (ball_e, ball_t) in ball_q.iter() {
        let mut region = None;

        for (i, ev) in all_events.iter().enumerate() {
            let other_e;
            let (entity_1, entity_2) = ev.rigid_body_entities();
            if ball_e == entity_1 {
                other_e = entity_2;
            } else if ball_e == entity_2 {
                other_e = entity_1;
            } else {
                continue;
            }

            if let Ok(r) = region_q.get(other_e) {
                if ev.is_started() {
                    trace!("[{}] Entered {:?}", i, r);

                    // entered region
                    region = Some(r);
                } else {
                    trace!("[{}] Exited {:?}", i, r);

                    // exited region
                    if region.is_none()
                        && *r != CourtRegion::OutOfBounds
                        && (ball_t.translation.x < court_set.left
                            || ball_t.translation.x > court_set.right
                            || ball_t.translation.y < court_set.bottom
                            || ball_t.translation.y > court_set.top)
                    {
                        region = Some(&CourtRegion::OutOfBounds);
                    }
                }
            }
        }

        if let Some(r) = region {
            if let Ok(mut ball) = ball_mut_q.get_mut(ball_e) {
                trace!("{:?} => {:?}", ball.region, r);

                if (ball.region.is_left() && r.is_right())
                    || (ball.region.is_right() && r.is_left())
                {
                    if let Ok((mut bounce, bounce_t)) =
                        ball_bounce_q.get_mut(ball.bounce_e.unwrap())
                    {
                        bounce.count = 0;
                        trace!("Crossed net");
                        trace!("height over net {}", bounce_t.translation.y);

                        if bounce_t.translation.y < 20. {
                            debug!("hit net");
                            let hit_vel_mult = 0.25;
                            ball.dir *= Vec2::new(-hit_vel_mult, hit_vel_mult);
                            bounce.velocity *= 0.5;

                            if let Ok(e) = entity_q.get(ball.trail_e.unwrap()) {
                                commands.entity(e).insert(FadeOutTrail {
                                    stop_trail: true,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }

                ball.region = *r;
            }
        }
    }
}

pub fn spawn_ball(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    serve_region: CourtRegion,
    fault_count: u8,
    player_id: usize,
    court_set: &Res<CourtSettings>,
) {
    let bounce_e = commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("art-ish/ball.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::ONE * BALL_SIZE),
                ..Default::default()
            },
            transform: Transform::from_xyz(0., 0., 0.5),
            ..Default::default()
        })
        .insert(BallBounce {
            gravity: -420.,
            max_velocity: 200.,
            ..Default::default()
        })
        .insert(PaletteColor::Ball)
        .id();

    let shadow = commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("art-ish/ball.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(1.0, 0.5) * BALL_SIZE),
                ..Default::default()
            },
            transform: Transform {
                translation: Vec3::new(-3., -14., -BALL_Z + SHADOW_Z),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PaletteColor::Shadow)
        .id();

    let trail_e = commands
        .spawn_bundle(GeometryBuilder::build_as(
            &PathBuilder::new().build().0,
            DrawMode::Fill(FillMode::color(Color::rgb_u8(32, 40, 61))),
            Transform::from_xyz(0., 0., PLAYER_Z + 0.5),
        ))
        .insert(Trail {
            points: Vec::new(),
            transform_e: bounce_e,
            duration_sec: 0.3,
            max_width: 30.,
        })
        .insert(Name::new("BallTrail"))
        .id();

    let mut rng = rand::thread_rng();
    let x = rng.gen_range((court_set.right / 2.)..=court_set.right) as f32;
    let x = if serve_region.is_left() { -x } else { x };
    let y = rng.gen_range(120..=280) as f32;
    let y = if serve_region.is_bottom() { -y } else { y };
    let _ball_e = commands
        .spawn_bundle(TransformBundle {
            transform: Transform {
                translation: Vec3::new(x, y, BALL_Z),
                scale: Vec3::ZERO,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(GlobalTransform::default())
        .insert(Ball {
            size: BALL_SIZE,
            speed: 1100.,
            region: serve_region,
            bounce_e: Some(bounce_e),
            trail_e: Some(trail_e),
            ..Default::default()
        })
        .insert(BallStatus::Serve(serve_region, fault_count, player_id))
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Sphere { radius: 15. })
        .insert(CollisionLayers::all::<PhysLayer>())
        .insert(Name::new("Ball"))
        .add_child(bounce_e)
        .add_child(shadow)
        .insert(Animator::new(Delay::new(Duration::from_millis(500)).then(
            Tween::new(
                EaseFunction::BackOut,
                TweeningType::Once,
                Duration::from_millis(450),
                TransformScaleLens {
                    start: Vec2::ZERO.extend(1.),
                    end: Vec3::ONE,
                },
            ),
        )))
        .id();
}
