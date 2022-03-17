use crate::{
    animation::{
        get_scale_in_anim, get_scale_out_anim, inverse_lerp, TransformRotation, TweenDoneAction,
    },
    ball::{spawn_ball, Ball, BallBouncedEvt, BallStatus},
    extra::TransformBundle,
    impl_player_action_timer,
    level::{CourtRegion, CourtSettings, InitialRegion, NetOffset, ServingRegion},
    palette::PaletteColor,
    physics::PhysLayer,
    player_action::{ActionTimer, PlayerActionStatus},
    player_animation::{PlayerAnimation, PlayerAnimationData},
    render::{PLAYER_Z, SHADOW_Z},
    score::{add_point_to_score, GameOverEvt, PlayerScore, Score, ScoreChangedEvt},
    trail::FadeOutTrail,
    GameSetupPhase, GameState, BASE_VIEW_WIDTH,
};
use bevy::{
    ecs::system::EntityCommands,
    math::Vec2,
    prelude::*,
    sprite::{Sprite, SpriteBundle},
};
use bevy_extensions::Vec2Conversion;
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};

use bevy_tweening::*;
use heron::*;

pub const PLAYER_SIZE: f32 = 56.;
pub const AIM_RING_ROTATION_DEG: f32 = 50.;
// pub const AIM_RING_RADIUS: f32 = 350.;
pub const AIM_RING_RADIUS: f32 = 100.;
// todo: get rid of this by fixing the animation system order and sue an enum label for that
pub const SWING_LABEL: &str = "swing";

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_set(
            SystemSet::on_enter(GameState::Game).with_system(setup.label(GameSetupPhase::Player)),
        )
        .add_system_set(
            SystemSet::on_update(GameState::Game)
                .with_system(move_player.before(SWING_LABEL))
                .with_system(aim)
                .with_system(swing)
                .with_system(on_ball_bounced),
        )
        .add_system_to_stage(CoreStage::Last, follow_scale);
    }
}

#[derive(Component, Inspectable)]
pub struct Player {
    pub id: usize,
    pub aim_e: Entity,
    pub aim_charge_e: Entity,
    side: f32,
}

impl Player {
    pub fn is_left(&self) -> bool {
        is_left_player_id(self.id)
    }

    pub fn get_sign(&self) -> f32 {
        if self.is_left() {
            -1.
        } else {
            1.
        }
    }
}

// todo: just add a side enum and add it to player or as a component? (covered by the size field - currently quite a mess)
pub fn is_left_player_id(id: usize) -> bool {
    id == 1
}

#[derive(Component, Inspectable)]
pub struct Inactive;

#[derive(Component, Inspectable)]
pub struct PlayerGui;

#[derive(Component, Inspectable)]
pub struct FollowScale {
    followed_e: Entity,
    scale_multiplier: Vec3,
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerMovement {
    speed: f32,
    charging_speed: f32,
    easing_time: f32,
    time_to_max_speed: f32,
    pub raw_dir: Vec2,
    last_non_zero_raw_dir: Vec2,
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerDash {
    pub status: PlayerActionStatus<Vec2>,
    #[inspectable(ignore)]
    pub timer: Timer,
    pub duration_sec: f32,
    cooldown_sec: f32,
    speed: f32,
}

impl_player_action_timer!(PlayerDash, Vec2);

#[derive(Default, Component, Inspectable)]
pub struct PlayerAim {
    pub raw_dir: Vec2,
    pub dir: Vec2,
}

#[derive(Component, Inspectable)]
pub struct SwingRangeSprite;

#[derive(Default, Component, Inspectable)]
pub struct PlayerSwing {
    pub status: PlayerActionStatus<f32>,
    pub duration_sec: f32,
    pub cooldown_sec: f32,
    #[inspectable(ignore)]
    pub timer: Timer,
}

impl PlayerSwing {
    pub fn start_cooldown(&mut self) {
        self.status = PlayerActionStatus::Cooldown;
        self.timer = Timer::from_seconds(self.cooldown_sec, false);
    }
}

impl_player_action_timer!(PlayerSwing, f32);

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    movement: PlayerMovement,
    dash: PlayerDash,
    swing: PlayerSwing,
    score: PlayerScore,
}

// todo: just remove the bundle and insert the components directly?
impl PlayerBundle {
    fn new(id: usize, initial_dir: Vec2, aim_e: Entity, aim_charge_e: Entity) -> Self {
        Self {
            player: Player {
                id,
                side: -initial_dir.x.signum(),
                aim_e,
                aim_charge_e,
            },
            movement: PlayerMovement {
                speed: 550.,
                charging_speed: 125.,
                time_to_max_speed: 0.11,
                ..Default::default()
            },
            dash: PlayerDash {
                speed: 2200.,
                duration_sec: 0.085,
                cooldown_sec: 0.5,
                ..Default::default()
            },
            swing: PlayerSwing {
                duration_sec: 0.15,
                cooldown_sec: 0.35,
                ..Default::default()
            },
            score: PlayerScore {
                ..Default::default()
            },
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, region: Res<InitialRegion>) {
    if cfg!(feature = "debug") {
        spawn_player(1, &mut commands, &asset_server, &region);
    } else {
        for id in 1..=2 {
            spawn_player(id, &mut commands, &asset_server, &region);
        }
    }
}

pub fn spawn_player<'a, 'b, 'c>(
    id: usize,
    commands: &'c mut Commands<'a, 'b>,
    asset_server: &Res<AssetServer>,
    region: &Res<InitialRegion>,
) -> EntityCommands<'a, 'b, 'c> {
    let x = BASE_VIEW_WIDTH / 4.;
    let x = if id == 1 { -x } else { x };
    let is_left = x < 0.;
    let mut player_y = 150.;
    let player_size = Vec2::splat(PLAYER_SIZE);
    let is_serving = region.0.is_left() == is_left;
    if (is_serving && region.0.is_bottom()) || (!is_serving && region.0.is_top()) {
        player_y *= -1.;
    }

    let initial_dir = if is_left { Vec2::X } else { -Vec2::X };

    let mut body_e = None;
    let mut body_root_e = None;

    // face
    let face_e = commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("art-ish/face_happy.png"),
            sprite: Sprite {
                flip_x: !is_left,
                custom_size: Some(player_size),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Animator::<Transform>::default())
        .insert(PaletteColor::PlayerFace)
        .id();

    // aim
    let aim_e = commands
        .spawn_bundle(TransformBundle {
            transform: Transform::from_rotation(if is_left {
                Quat::from_rotation_z(-90f32.to_radians())
            } else {
                Quat::from_rotation_z(90f32.to_radians())
            }),
            ..Default::default()
        })
        .insert(PlayerAim {
            dir: initial_dir,
            raw_dir: Vec2::ZERO,
        })
        .insert(PlayerGui)
        .with_children(|b| {
            // aim arrow
            b.spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/aim_arrow.png"),
                transform: Transform::from_xyz(0., AIM_RING_RADIUS, -0.4),
                ..Default::default()
            })
            .insert(PaletteColor::PlayerAim);
        })
        .id();

    let aim_charge_e = commands
        .spawn_bundle(SpriteBundle {
            texture: asset_server.load("art-ish/aim_charge.png"),
            transform: Transform {
                translation: Vec3::new(0., 0., -0.7),
                scale: Vec3::Z,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PaletteColor::PlayerCharge)
        .insert(PlayerGui)
        .id();

    let mut p = commands.spawn_bundle(TransformBundle {
        transform: Transform {
            translation: Vec3::new(x, player_y, PLAYER_Z),
            scale: Vec2::ZERO.extend(1.),
            ..Default::default()
        },
        ..Default::default()
    });
    p.insert_bundle(PlayerBundle::new(id, initial_dir, aim_e, aim_charge_e))
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Sphere {
            radius: AIM_RING_RADIUS,
        })
        .insert(CollisionLayers::none())
        .insert(get_scale_in_anim(Vec3::ONE, 450, None))
        .insert(Name::new("Player"))
        .add_child(aim_e)
        .add_child(aim_charge_e)
        .with_children(|b| {
            // circle
            let rotation_speed = if is_left {
                -AIM_RING_ROTATION_DEG
            } else {
                AIM_RING_ROTATION_DEG
            };
            b.spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/player_circle.png"),
                transform: Transform::from_xyz(0., 0., -0.1),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(AIM_RING_RADIUS * 2.)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(PaletteColor::PlayerAim)
            .insert(SwingRangeSprite)
            .insert(PlayerGui)
            .insert(TransformRotation::new(rotation_speed.to_radians()));

            // body root
            body_root_e = Some(
                b.spawn_bundle(TransformBundle::from_xyz(0., 0., 0.))
                    .insert(Name::new("player_body_root"))
                    .add_child(face_e)
                    .with_children(|b| {
                        // body
                        body_e = Some(
                            b.spawn_bundle(SpriteBundle {
                                texture: asset_server.load("art-ish/player_body.png"),
                                sprite: Sprite {
                                    custom_size: Some(player_size),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .insert(PaletteColor::Player)
                            .insert(Animator::<Transform>::default())
                            .insert(Name::new("player_body"))
                            .id(),
                        );
                    })
                    .insert(Animator::<Transform>::default())
                    .id(),
            );

            // shadow
            b.spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/player_body.png"),
                transform: Transform {
                    translation: Vec3::new(-6., -22., -PLAYER_Z + SHADOW_Z),
                    ..Default::default()
                },
                sprite: Sprite {
                    custom_size: Some(player_size),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(PaletteColor::Shadow)
            .insert(FollowScale {
                followed_e: body_e.unwrap(),
                scale_multiplier: Vec3::new(1.0, 0.5, 1.),
            })
            .insert(Name::new("player_shadow"));
        })
        .insert(PlayerAnimationData {
            animation: PlayerAnimation::Idle,
            face_e,
            body_e: body_e.unwrap(),
            body_root_e: body_root_e.unwrap(),
        });
    p
}

// todo: slight acceleration
// nice2have: lerp dash
fn move_player(
    mut query: Query<
        (
            &Player,
            &mut PlayerMovement,
            &PlayerDash,
            &mut Transform,
            &PlayerSwing,
            &mut PlayerAnimationData,
        ),
        Without<Inactive>,
    >,
    time: ScaledTime,
    net: Res<NetOffset>,
    court: Res<CourtSettings>,
) {
    for (player, mut player_movement, player_dash, mut player_t, player_swing, mut p_anim) in
        query.iter_mut()
    {
        let charging = matches!(player_swing.status, PlayerActionStatus::Charging(_));
        let speed = if charging {
            player_movement.charging_speed
        } else {
            player_movement.speed
        };
        let dir = if player_movement.raw_dir != Vec2::ZERO {
            player_movement.raw_dir
        } else {
            player_movement.last_non_zero_raw_dir
        };
        let mut move_by = (dir * speed).to_vec3();
        let mut dashing = false;

        if let PlayerActionStatus::Active(dash_dir) = player_dash.status {
            move_by = (dash_dir * player_dash.speed).to_vec3();
            dashing = true;
        }

        let mut final_pos = player_t.translation + move_by * time.scaled_delta_seconds();

        if !dashing {
            // easing
            let ease_time_delta = if player_movement.raw_dir == Vec2::ZERO {
                -time.scaled_delta_seconds()
            } else {
                time.scaled_delta_seconds()
            };
            player_movement.easing_time += ease_time_delta;
            player_movement.easing_time = player_movement
                .easing_time
                .clamp(0., player_movement.time_to_max_speed);

            let ease_t = inverse_lerp(
                0.,
                player_movement.time_to_max_speed,
                player_movement.easing_time,
            );
            final_pos = player_t.translation.lerp(final_pos, ease_t);
        } else {
            // todo: ease dash as well
            player_movement.easing_time = player_movement.time_to_max_speed;
        }

        // nice2have: get/store properly
        let player_size = Vec2::splat(PLAYER_SIZE);
        let court_w_half = court.view.x / 2. + player_size.x;
        let player_area_size = if player.is_left() {
            Vec2::new(court_w_half + net.current_offset, court.view.y)
        } else {
            Vec2::new(court_w_half - net.current_offset, court.view.y)
        };
        let pos_offset = Vec2::new(player_area_size.x / 2., 0.);
        let player_area_pos = if player.is_left() {
            Vec2::X * net.current_offset - pos_offset
        } else {
            Vec2::X * net.current_offset + pos_offset
        };

        let half_bounds = player_area_size / 2. - player_size / 2.;
        let area_btm_left = player_area_pos - half_bounds;
        let area_top_right = player_area_pos + half_bounds;
        final_pos = final_pos
            .truncate()
            .clamp(area_btm_left, area_top_right)
            .extend(final_pos.z);

        if (final_pos - player_t.translation).length().abs() > 0.1 {
            if !dashing {
                if charging && p_anim.animation != PlayerAnimation::Walking {
                    p_anim.animation = PlayerAnimation::Walking;
                } else if !charging && p_anim.animation != PlayerAnimation::Running {
                    p_anim.animation = PlayerAnimation::Running;
                }
            }
        } else if p_anim.animation != PlayerAnimation::Idle {
            p_anim.animation = PlayerAnimation::Idle;
        }

        player_t.translation = final_pos;

        if player_movement.raw_dir != Vec2::ZERO {
            player_movement.last_non_zero_raw_dir = player_movement.raw_dir;
        }
    }
}

// todo: clamp angle based on Y distance from center?
fn aim(
    player_q: Query<(&Player, &PlayerAnimationData), Without<Inactive>>,
    mut aim_q: Query<(&mut PlayerAim, &mut Transform, &Parent)>,
    mut transform_q: Query<&mut Transform, Without<PlayerAim>>,
    time: ScaledTime,
) {
    for (mut aim, mut aim_t, aim_parent) in aim_q.iter_mut() {
        if let Ok((p, p_anim)) = player_q.get(aim_parent.0) {
            let mut dir = aim.raw_dir.normalize_or_zero();

            if dir == Vec2::ZERO {
                continue;
            }

            let clamp_x = 1.;
            let clamp_y = 0.75;
            let player_x_sign = p.get_sign();

            if dir == Vec2::new(player_x_sign, 0.) {
                // player aiming into their court/backwards - just aim straight
                dir = Vec2::new(-player_x_sign, 0.);
            } else if player_x_sign < 0. {
                dir = dir.clamp(Vec2::new(clamp_x, -clamp_y), Vec2::new(clamp_x, clamp_y));
            } else {
                dir = dir.clamp(Vec2::new(-clamp_x, -clamp_y), Vec2::new(-clamp_x, clamp_y));
            }

            // nice2have: extract this to extensions & for now just move to extra
            let target_rotation = Quat::from_axis_angle(-Vec3::Z, dir.angle_between(Vec2::Y));
            let limit = 260f32.to_radians() * time.scaled_delta_seconds() * aim.raw_dir.length();
            if target_rotation.angle_between(aim_t.rotation) <= limit {
                aim_t.rotation = Quat::from_axis_angle(-Vec3::Z, dir.angle_between(Vec2::Y));
            } else {
                let rotate_by = if target_rotation.to_euler(EulerRot::XYZ).2
                    > aim_t.rotation.to_euler(EulerRot::XYZ).2
                {
                    limit
                } else {
                    -limit
                };
                aim_t.rotate(Quat::from_rotation_z(rotate_by));
            }

            let clamped_dir = aim_t.rotation * Vec3::Y;
            aim.dir = clamped_dir.truncate();

            if let Ok(mut face_t) = transform_q.get_mut(p_anim.face_e) {
                let axis = if p.is_left() { Vec2::X } else { -Vec2::X };
                face_t.rotation =
                    Quat::from_axis_angle(-Vec3::Z, aim.dir.angle_between(axis) * 0.25);
            }
        }
    }
}

fn swing(
    mut query: Query<
        (
            &PlayerSwing,
            ChangeTrackers<PlayerSwing>,
            &mut CollisionLayers,
            &mut PlayerAnimationData,
        ),
        Without<Inactive>,
    >,
) {
    for (player_swing, player_swing_tracker, mut coll_layers, mut anim) in query.iter_mut() {
        if player_swing_tracker.is_changed() {
            match player_swing.status {
                PlayerActionStatus::Ready
                | PlayerActionStatus::Cooldown
                | PlayerActionStatus::Charging(_) => {
                    *coll_layers = CollisionLayers::none();
                }
                PlayerActionStatus::Active(_) => {
                    *coll_layers = CollisionLayers::all::<PhysLayer>();

                    // 2fix: animation should fire only after collision or the timer runs out
                    anim.animation = PlayerAnimation::Swinging;
                }
            }
        }
    }
}

// todo: move to ball.rs/split
fn on_ball_bounced(
    mut commands: Commands,
    mut ev_r_ball_bounced: EventReader<BallBouncedEvt>,
    mut score_ev_w: EventWriter<ScoreChangedEvt>,
    mut game_over_ev_w: EventWriter<GameOverEvt>,
    player_q: Query<&Player, Without<Inactive>>,
    mut ball_q: Query<(&Ball, &mut BallStatus, &Transform)>,
    asset_server: Res<AssetServer>,
    mut serving_region: ResMut<ServingRegion>,
    entity_q: Query<Entity>,
    mut score: ResMut<Score>,
    court_set: Res<CourtSettings>,
) {
    for ev in ev_r_ball_bounced.iter() {
        if let Ok((ball, mut status, ball_t)) = ball_q.get_mut(ev.ball_e) {
            let ball_res = match *status {
                BallStatus::Fault(count, player_id) => {
                    // nice2have: limit might come from an upgrade
                    let limit = 1;
                    let losing_player = if count > limit { Some(player_id) } else { None };
                    let fault_count = if count > limit { 0 } else { count };
                    Some((losing_player, fault_count, "double fault"))
                }
                BallStatus::Rally(player_id) => {
                    // nice2have: limit might come from an upgrade
                    let bounce_limit = 1;

                    // out of bounds
                    if ball.region.is_out_of_bounds() && ev.bounce_count == 1 {
                        Some((Some(player_id), 0, "shooting out of bounds"))
                    } else if ev.bounce_count > bounce_limit {
                        let player = player_q.iter().find(|p| p.side == ev.side).unwrap();

                        Some((Some(player.id), 0, "too many bounces"))
                    } else {
                        None
                    }
                }
                BallStatus::Serve(..) | BallStatus::Used => None,
            };

            if let Some((losing_player, fault_count, reason)) = ball_res {
                let mut swap_serve = false;

                if let Some(losing_player) = losing_player {
                    swap_serve = add_point_to_score(
                        &mut score,
                        &mut score_ev_w,
                        &mut game_over_ev_w,
                        !is_left_player_id(losing_player),
                    );

                    debug!(
                        "Player {} has lost a point to {}! (bounce_count: {})",
                        losing_player, reason, ev.bounce_count
                    );
                }

                *status = BallStatus::Used;
                commands.entity(ev.ball_e).insert(get_scale_out_anim(
                    ball_t.scale,
                    450,
                    Some(TweenDoneAction::DespawnRecursive),
                ));

                if let Ok(e) = entity_q.get(ball.trail_e.unwrap()) {
                    commands.entity(e).insert(FadeOutTrail {
                        decrease_duration_by: 1.,
                        ..Default::default()
                    });
                }

                if swap_serve {
                    serving_region.0 = if serving_region.0.is_left() {
                        CourtRegion::get_random_right()
                    } else {
                        CourtRegion::get_random_left()
                    };
                }

                // todo: skip if game over
                spawn_ball(
                    &mut commands,
                    &asset_server,
                    serving_region.0,
                    fault_count,
                    serving_region.0.get_player_id(),
                    &court_set,
                );
            }
        }
    }
}

fn follow_scale(follow_q: Query<(Entity, &FollowScale)>, mut transform_q: Query<&mut Transform>) {
    for (following_e, follow) in follow_q.iter() {
        if let Ok(followed_t) = transform_q.get(follow.followed_e) {
            if let Ok(mut following_t) = transform_q.get_mut(following_e) {
                following_t.scale = followed_t.scale * follow.scale_multiplier;
            }
        }
    }
}
