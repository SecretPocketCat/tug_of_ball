use std::time::Duration;

use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_input::ActionInput;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::{TransformPositionLens, TransformScaleLens};
use bevy_tweening::*;
use heron::rapier_plugin::PhysicsWorld;
use heron::*;
use rand::*;

use crate::{player::{PlayerSwing, ActionStatus, PlayerMovement, Player, ServingRegion}, PlayerInput, InputAxis, wall::Wall, WIN_WIDTH, level::{CourtRegion, CourtSettings}, PhysLayer, BALL_Z, TransformBundle};

const BALL_SIZE: f32 = 35.;

#[derive(Default, Component, Inspectable)]
pub struct Ball {
    dir: Vec2,
    size: f32,
    speed: f32,
    prev_pos: Vec3,
    pub(crate) region: CourtRegion,
    bounce_e: Option<Entity>,
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
    pub(crate) ball_e: Entity,
    pub(crate) bounce_count: usize,
    pub(crate) side: f32,
}

pub struct BallPlugin;
impl Plugin for BallPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
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
) {
    let region = CourtRegion::TopLeft;
    // let region = CourtRegion::get_random_left();
    // let region = CourtRegion::get_random();
    spawn_ball(&mut commands, &asset_server, region, 0, region.get_player_id());
    commands.insert_resource(ServingRegion(region));
}

// nice2have: try - slowly speedup during rally?
fn movement(
    mut ball_q: Query<(&mut Ball, &mut Transform)>,
    mut bounce_q: Query<&mut BallBounce>,
    time: ScaledTime,
) {
    for (mut ball, mut t) in ball_q.iter_mut() {
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
        t.translation += ball.dir.to_vec3() * ball.speed * time.scaled_delta_seconds();

        if t.translation.x.signum() != ball.prev_pos.x.signum() {
            let mut bounce = bounce_q.get_mut(ball.bounce_e.unwrap()).unwrap();
            bounce.count = 0;
            debug!("crossed net");
        }

        ball.prev_pos = t.translation;
    }
}

fn get_bounce_velocity(dir_len: f32, max_velocity: f32) -> f32 {
    dir_len.sqrt().min(1.) * max_velocity
}

fn bounce(
    mut bounce_query: Query<(&mut BallBounce, &mut Transform, &Parent), Without<Ball>>,
    mut ball_q: Query<(Entity, &mut Ball, &mut BallStatus, &Transform)>,
    mut ev_w_bounce: EventWriter<BallBouncedEvt>,
    time: ScaledTime,
) {
    for (mut ball_bounce, mut t, p) in bounce_query.iter_mut() {
        if let Ok((ball_e, mut ball, mut ball_status, ball_t)) = ball_q.get_mut(p.0) {
            if ball.dir == Vec2::ZERO {
                continue;
            }
    
            ball_bounce.velocity += ball_bounce.gravity * time.scaled_delta_seconds();
            t.translation.y += ball_bounce.velocity * time.scaled_delta_seconds();
    
            if t.translation.y <= 0. {
                t.translation.y = 0.01;
                ball_bounce.velocity = get_bounce_velocity(ball.dir.length(), ball_bounce.max_velocity);
                ball_bounce.count += 1;
                trace!("Bounce {}", ball_bounce.count);
    
                // eval serve on bounce
                if let BallStatus::Serve(region, fault_count, player_id) = *ball_status {
                    if ball.region != region.get_inverse().unwrap() {
                        // fault
                        *ball_status = BallStatus::Fault(fault_count + 1, player_id);
                        debug!("Bad serve {:?} => {:?}", region, ball.region);
                    }
                    else {
                        // good serve
                        *ball_status = BallStatus::Rally(player_id);
                        debug!("Good serve {:?} => {:?}", region, ball.region);
                    }
                }
    
                ev_w_bounce.send(BallBouncedEvt {
                    ball_e,
                    bounce_count: ball_bounce.count,
                    side: ball_t.translation.x.signum(),
                });
                debug!("Bounced {} times", ball_bounce.count);
            }
        } 
    }
}

// todo: 'auto dash swing'?
fn handle_collisions(
    mut coll_events: EventReader<CollisionEvent>,
    input: Res<PlayerInput>,
    mut ball_q: Query<(&mut Ball, &mut BallStatus, &Children)>,
    mut ball_bounce_q: Query<&mut BallBounce>,
    mut player_q: Query<(&Player, &PlayerMovement, &mut PlayerSwing, &GlobalTransform)>,
    wall_q: Query<&Sprite, With<Wall>>,
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
                bounce_e = b.2.iter().nth(0).unwrap();
                other_e = entity_2;
            } else if let Ok(b) = ball_q.get_mut(entity_2) {
                ball = b.0;
                status = b.1;
                bounce_e = b.2.iter().nth(0).unwrap();
                other_e = entity_1;
            } else {
                continue;
            }

            let mut ball_bounce = ball_bounce_q.get_mut(bounce_e.clone()).unwrap();

            if let Ok((player, movement, mut swing, player_t)) = player_q.get_mut(other_e) {
                if let ActionStatus::Active(ball_speed_multiplier) = swing.status {
                    if !swing.timer.finished() {
                        swing.start_cooldown();
                        let mut dir = input.get_xy_axes(player.id, &InputAxis::X, &InputAxis::Y);

                        if dir == Vec2::ZERO {
                            dir = movement.last_dir;
                        }

                        let clamp_x = 1.;
                        let clamp_y = 0.8;
                        let player_x = player_t.translation.x;
                        let player_x_sign = player_x.signum();

                        if dir == Vec2::new(player_x_sign, 0.) {
                            // player aiming into their court/backwards - just aim straight
                            dir = Vec2::new(-player_x_sign, 0.);
                        }
                        else if player_x < 0. {
                            dir = dir.clamp(Vec2::new(clamp_x, -clamp_y), Vec2::new(clamp_x, clamp_y));
                        }
                        else {
                            dir = dir.clamp(Vec2::new(-clamp_x, -clamp_y), Vec2::new(-clamp_x, clamp_y));
                        }

                        ball.dir = dir * ball_speed_multiplier;
                        ball_bounce.velocity = get_bounce_velocity(dir.length(), ball_bounce.max_velocity);

                        let rot = Quat::from_rotation_arc_2d(Vec2::Y, dir).to_euler(EulerRot::XYZ).2.to_degrees();
                        debug!("Hit rot {:?}", rot);

                        match *status {
                            BallStatus::Serve(_, _, player_id) if player_id != player.id => {
                                // vollied serve
                                *status = BallStatus::Rally(player.id);
                                trace!("Vollied serve");
                            },
                            BallStatus::Rally(..) => {
                                // set rally player on hit, also applies to 
                                *status = BallStatus::Rally(player.id);
                            },
                            _ => {}
                        }
                    }
                }
            }
            // todo: also handle 'net collision here' based on bounce height
            else if let Ok(wall_sprite) = wall_q.get(other_e) {
                let size = wall_sprite.custom_size.unwrap();
                let is_hor = size.x > size.y;
                let x = if is_hor { 1. } else { -1. }; 
                ball.dir *= Vec2::new(x, -x);
            }
        }
    }
}

fn handle_regions(
    mut coll_events: EventReader<CollisionEvent>,
    ball_q: Query<(Entity, &GlobalTransform), With<Ball>>,
    mut ball_mut_q: Query<&mut Ball>,
    mut ball_bounce_q: Query<&mut BallBounce>,
    region_q: Query<&CourtRegion>,
    court_set: Res<CourtSettings>,
) {
    let all_events: Vec<CollisionEvent> = coll_events.iter().cloned().collect();
    for (ball_e, ball_t) in ball_q.iter() {
        let mut region = None;

        let mut i = -1;
        for ev in all_events.iter() {
            i += 1;
            let other_e;
            let (entity_1, entity_2) = ev.rigid_body_entities();
            if ball_e == entity_1 {
                other_e = entity_2;
            }
            else if ball_e == entity_2 {
                other_e = entity_1;
            } else {
                continue;
            }

            if let Ok(r) = region_q.get(other_e) {
                if ev.is_started() {
                    trace!("[{}] Entered {:?}", i, r);

                    // entered region
                    region = Some(r);
                }
                else {
                    trace!("[{}] Exited {:?}", i, r);

                    // exited region
                    if region.is_none() && *r != CourtRegion::OutOfBounds &&
                        (ball_t.translation.x < court_set.left ||
                        ball_t.translation.x > court_set.right ||
                        ball_t.translation.y < court_set.bottom ||
                        ball_t.translation.y > court_set.top) {
                        region = Some(&CourtRegion::OutOfBounds);
                    }
                }
            }
        }

        if let Some(r) = region {
            if let Ok(mut ball) = ball_mut_q.get_mut(ball_e) {
                trace!("{:?} => {:?}", ball.region, r);

                if (ball.region.is_left() && r.is_right()) ||
                    (ball.region.is_right() && r.is_left()) {
                    let mut bounce = ball_bounce_q.get_mut(ball.bounce_e.unwrap()).unwrap();
                    bounce.count = 0;
                    trace!("Crossed net");
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
    player_id: usize
) {
    let bounce = commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("art-ish/ball.png"),
        sprite: Sprite {
            color: Color::YELLOW,
            custom_size: Some(Vec2::ONE * BALL_SIZE),
            ..Default::default()
        },
        transform: Transform::from_xyz(0., 0., 0.5),
        ..Default::default()
        }).insert(BallBounce {
            gravity: -420.,
            max_velocity: 200.,
            ..Default::default()
        })
        .id();

    let shadow = commands.spawn_bundle(SpriteBundle {
        texture: asset_server.load("art-ish/ball.png"),
        sprite: Sprite {
            color: Color::rgba(0., 0., 0., 0.5),
            custom_size: Some(Vec2::new(1.0, 0.5) * BALL_SIZE),
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0., -13., -0.5),
            ..Default::default()
        },
        ..Default::default()
        }).id();

    let mut rng = rand::thread_rng();
    let x = WIN_WIDTH / 2. - 330.;
    let x = if serve_region.is_left() { -x } else { x };
    let y = rng.gen_range(120..=280) as f32;
    let y = if serve_region.is_bottom() { -y } else { y };
    commands.spawn_bundle(TransformBundle {
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
            bounce_e: Some(bounce.clone()),
            ..Default::default()
        })
        .insert(BallStatus::Serve(serve_region, fault_count, player_id))
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Sphere {
            radius: 15.,
        })
        .insert(CollisionLayers::all::<PhysLayer>())
        .insert(Name::new("Ball"))
        .add_child(bounce)
        .add_child(shadow)
        .insert(Animator::new(
            Delay::new(Duration::from_millis(500)).then(Tween::new(
            EaseFunction::BackOut,
            TweeningType::Once,
            Duration::from_millis(450),
            TransformScaleLens {
                start: Vec2::ZERO.extend(1.),
                end: Vec3::ONE,
            }
        )) ));
}
