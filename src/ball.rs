use std::time::Duration;

use bevy::{
    math::Vec2,
    prelude::*,
    sprite::{Sprite, SpriteBundle},
};
use bevy_extensions::Vec2Conversion;

use crate::{
    animation::{inverse_lerp, TweenDoneAction},
    extra::TransformBundle,
    level::{CourtRegion, CourtSettings, InitialRegion, NetOffset, ServingRegion},
    palette::{Palette, PaletteColor},
    physics::PhysLayer,
    player::{Player, PlayerAim, PlayerSwing},
    player_action::PlayerActionStatus,
    render::{BALL_Z, PLAYER_Z, SHADOW_Z},
    trail::{FadeOutTrail, Trail},
    GameSetupPhase, GameState,
};
use bevy_inspector_egui::Inspectable;
use bevy_prototype_lyon::prelude::*;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::{SpriteColorLens, TransformScaleLens};
use bevy_tweening::*;
use heron::*;
use rand::*;

pub const BALL_MIN_SPEED: f32 = 400.;
pub const BALL_MAX_SPEED: f32 = 2000.;
pub const BALL_GRAVITY: f32 = -1100.;
pub const TARGET_X_OFFSET: f32 = 100.;
const BALL_SIZE: f32 = 35.;

pub struct BallPlugin;
impl Plugin for BallPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_set(
            SystemSet::on_enter(GameState::Game).with_system(setup.label(GameSetupPhase::Ball)),
        )
        .add_system_to_stage(CoreStage::PostUpdate, handle_collisions)
        .add_system_to_stage(CoreStage::PostUpdate, handle_regions)
        .add_system_set(SystemSet::on_update(GameState::Game).with_system(move_ball))
        .add_event::<BallBouncedEvt>()
        .add_event::<BallHitEvt>();
    }
}

#[derive(Default, Component, Inspectable)]
pub struct Ball {
    pub dir: Vec2,
    pub speed: f32,
    pub region: CourtRegion,
    pub bounce_e: Option<Entity>,
    pub trail_e: Option<Entity>,
    pub predicted_bounce_pos: Vec2,
    prev_pos: Vec3,
    size: f32,
}

#[derive(Default, Component, Inspectable)]
pub struct BallBounce {
    pub count: usize,
    height: f32,
    target_height: f32,
    gravity_mult: f32,
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

pub struct BallHitEvt {
    pub ball_e: Entity,
    pub player_id: usize,
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

fn move_ball(
    mut ball_q: Query<(Entity, &mut Ball, &mut Transform, &mut BallStatus)>,
    mut bounce_q: Query<(&mut BallBounce, &mut Transform), Without<Ball>>,
    mut ev_w_bounce: EventWriter<BallBouncedEvt>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    palette: Res<Palette>,
    time: ScaledTime,
    net: Res<NetOffset>,
) {
    for (ball_e, mut ball, mut ball_t, mut ball_status) in ball_q.iter_mut() {
        if ball.dir == Vec2::ZERO {
            continue;
        }

        let speed = ball.dir.length();

        if speed < 0.025 {
            ball.dir = Vec2::ZERO;
            return;
        }

        // move
        ball_t.translation += (ball.dir * ball.speed).to_vec3() * time.scaled_delta_seconds();

        let net_x = net.0;
        let ball_x = ball_t.translation.x;
        let ball_prev_x = ball.prev_pos.x;

        ball.prev_pos = ball_t.translation;

        // bounce
        if let Ok((mut ball_bounce, mut bounce_t)) = bounce_q.get_mut(ball.bounce_e.unwrap()) {
            if (ball_prev_x < net_x && ball_x > net_x) || (ball_prev_x > net_x && ball_x < net_x) {
                ball_bounce.count = 0;
                trace!("crossed net extra check");
            }

            if ball.dir == Vec2::ZERO {
                continue;
            }

            bounce_t.translation.y += ball_bounce.height * time.scaled_delta_seconds();
            ball_bounce.height +=
                BALL_GRAVITY * ball_bounce.gravity_mult * time.scaled_delta_seconds();

            if bounce_t.translation.y <= 0. {
                bounce_t.translation.y = 0.;
                ball_bounce.count += 1;
                // ball_bounce.target_height *=
                //     1. - (0.3 * inverse_lerp(BALL_MAX_SPEED, BALL_MIN_SPEED, ball.speed));
                ball_bounce.height = ball_bounce.target_height;
                // ball.speed *= 0.8;

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

                if ball_bounce.count <= 4 {
                    spawn_bounce_track(
                        &mut commands,
                        &asset_server,
                        &palette,
                        ball_t.translation.truncate().extend(SHADOW_Z),
                    );
                }

                debug!("Bounced {} times", ball_bounce.count);
            }
        }
    }
}

// nice2have: 'auto dash swing'?
fn handle_collisions(
    mut coll_er: EventReader<CollisionEvent>,
    mut ball_hit_ew: EventWriter<BallHitEvt>,
    mut ball_q: Query<(&mut Ball, &mut BallStatus, &Transform)>,
    mut ball_bounce_q: Query<(&mut BallBounce, &Parent)>,
    player_aim_q: Query<&PlayerAim>,
    mut player_q: Query<(&Player, &mut PlayerSwing, &GlobalTransform)>,
    net: Res<NetOffset>,
) {
    for ev in coll_er.iter() {
        if ev.is_started() {
            let mut b_bounce;
            let ball_e;
            let other_e;
            let (entity_1, entity_2) = ev.rigid_body_entities();
            if let Ok((bounce, parent)) = ball_bounce_q.get_mut(entity_1) {
                b_bounce = bounce;
                ball_e = parent.0;
                other_e = entity_2;
            } else if let Ok((bounce, parent)) = ball_bounce_q.get_mut(entity_2) {
                b_bounce = bounce;
                ball_e = parent.0;
                other_e = entity_1;
            } else {
                continue;
            }

            if let Ok((player, mut swing, _player_t)) = player_q.get_mut(other_e) {
                if let Ok((mut ball, mut status, ball_t)) = ball_q.get_mut(ball_e) {
                    if let PlayerActionStatus::Active(swing_strength) = swing.status {
                        if !swing.timer.finished() {
                            swing.start_cooldown();

                            if let Ok(aim) = player_aim_q.get(player.aim_e) {
                                ball.dir = aim.dir.normalize();
                                ball.speed = BALL_MIN_SPEED
                                    .lerp(&BALL_MAX_SPEED, &swing_strength.min(1.5))
                                    + ball.speed * 0.25;

                                // todo: better calc distance/target
                                // should be based on strength, distance to net (the closer the shorter-ish the distance?), the current height!
                                // let dist =
                                let target_x = if player.is_left() {
                                    net.0 + TARGET_X_OFFSET
                                } else {
                                    net.0 - TARGET_X_OFFSET
                                };
                                let a = (target_x - ball_t.translation.x).abs();
                                let angle = Quat::from_rotation_arc_2d(
                                    -Vec2::X * player.get_sign(),
                                    ball.dir,
                                )
                                .to_euler(EulerRot::XYZ)
                                .2;
                                let dist = (a / angle.cos()).max(300.);
                                let time = dist / ball.speed;
                                let time_apex = time / 2.;
                                b_bounce.gravity_mult =
                                    inverse_lerp(BALL_MIN_SPEED, BALL_MAX_SPEED, ball.speed) * 2.0
                                        + 1.;
                                b_bounce.target_height =
                                    -BALL_GRAVITY * time_apex * b_bounce.gravity_mult;
                                b_bounce.height = b_bounce.target_height;

                                // todo: fix
                                ball.predicted_bounce_pos =
                                    ball_t.translation.truncate() + (ball.dir * ball.speed);

                                match *status {
                                    BallStatus::Serve(_, _, player_id)
                                        if player_id != player.id =>
                                    {
                                        // vollied serve
                                        *status = BallStatus::Rally(player.id);
                                        trace!("Vollied serve");
                                    }
                                    BallStatus::Rally(..) => {
                                        // set rally player on hit
                                        *status = BallStatus::Rally(player.id);
                                    }
                                    _ => {}
                                }
                            }

                            ball_hit_ew.send(BallHitEvt {
                                ball_e,
                                player_id: player.id,
                            });
                        }
                    }
                }
            }
        }
    }
}

// todo: fix out of bounds
fn handle_regions(
    mut commands: Commands,
    mut coll_events: EventReader<CollisionEvent>,
    ball_q: Query<&GlobalTransform, With<Ball>>,
    mut ball_mut_q: Query<&mut Ball>,
    mut ball_bounce_q: Query<(Entity, &mut BallBounce, &Transform, &Parent)>,
    region_q: Query<&CourtRegion>,
    court_set: Res<CourtSettings>,
    entity_q: Query<Entity, Without<Ball>>,
) {
    let all_events: Vec<CollisionEvent> = coll_events.iter().cloned().collect();
    for (bounce_e, mut bounce, bounce_t, ball_e) in ball_bounce_q.iter_mut() {
        let mut region = None;

        if let Ok(ball_t) = ball_q.get(ball_e.0) {
            for (i, ev) in all_events.iter().enumerate() {
                let other_e;
                let (entity_1, entity_2) = ev.rigid_body_entities();
                if bounce_e == entity_1 {
                    other_e = entity_2;
                } else if bounce_e == entity_2 {
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
                if let Ok(mut ball) = ball_mut_q.get_mut(ball_e.0) {
                    trace!("{:?} => {:?}", ball.region, r);

                    if (ball.region.is_left() && r.is_right())
                        || (ball.region.is_right() && r.is_left())
                    {
                        bounce.count = 0;
                        trace!("Crossed net");
                        trace!("height over net {}", bounce_t.translation.y);

                        // todo: is this at all needed?
                        // 'net detection'
                        // if bounce_t.translation.y < 20. {
                        //     debug!("hit net");
                        //     let hit_vel_mult = 0.25;
                        //     ball.dir *= Vec2::new(-hit_vel_mult, hit_vel_mult);
                        //     // todo: cut ball speed/vel
                        //     // bounce.height *= 0.5;

                        //     if let Ok(e) = entity_q.get(ball.trail_e.unwrap()) {
                        //         commands.entity(e).insert(FadeOutTrail {
                        //             stop_trail: true,
                        //             ..Default::default()
                        //         });
                        //     }
                        // }
                    }

                    ball.region = *r;
                }
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
            ..Default::default()
        })
        .insert(PaletteColor::Ball)
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Sphere { radius: 15. })
        .insert(CollisionLayers::all::<PhysLayer>())
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
    // let x = rng.gen_range((court_set.right / 2.)..=court_set.right) as f32;
    let x = court_set.right - 20.;
    let x = if serve_region.is_left() { -x } else { x };
    let y = rng.gen_range(120..=280) as f32;
    let y = if serve_region.is_bottom() { -y } else { y };
    commands
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
            region: serve_region,
            bounce_e: Some(bounce_e),
            trail_e: Some(trail_e),
            ..Default::default()
        })
        .insert(BallStatus::Serve(serve_region, fault_count, player_id))
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
        )));
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
