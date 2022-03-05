use std::time::Duration;

use bevy::{
    math::Vec2,
    prelude::*,
    sprite::{
        collide_aabb::{collide},
        Sprite, SpriteBundle,
    },
};
use bevy_extensions::Vec2Conversion;
use bevy_input::{ActionState};
use bevy_inspector_egui::Inspectable;

use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::{TransformPositionLens, TransformRotationLens, TransformScaleLens};
use bevy_tweening::*;
use heron::*;
use interpolation::EaseFunction;

use crate::{
    ball::{spawn_ball, Ball, BallBouncedEvt, BallStatus},
    extra::TransformBundle,
    input::PlayerInput,
    level::{CourtRegion, CourtSettings, InitialRegion, Net, NetOffset},
    palette::PaletteColor,
    physics::PhysLayer,
    render::{PLAYER_Z, SHADOW_Z},
    score::{add_point_to_score, Score},
    trail::FadeOutTrail,
    tween::{inverse_lerp, TweenDoneAction},
    InputAction, InputAxis, WIN_HEIGHT, WIN_WIDTH,
};

pub const AIM_RING_ROTATION_DEG: f32 = 50.;

#[derive(Inspectable, Clone, Copy)]
pub enum ActionStatus<TActiveData: Default> {
    Ready,
    Active(TActiveData),
    Cooldown,
}

impl<TActiveData: Default> Default for ActionStatus<TActiveData> {
    fn default() -> Self {
        ActionStatus::Ready
    }
}

trait ActionTimer<TActiveData: Default> {
    fn get_timer_mut(&mut self) -> &mut Timer;

    fn get_action_status_mut(&mut self) -> &mut ActionStatus<TActiveData>;

    fn get_cooldown_sec(&self) -> f32;

    fn handle_action_timer(&mut self, scaled_delta_time: Duration) {
        let cooldown_sec = self.get_cooldown_sec();
        let status = self.get_action_status_mut();
        let is_cooldown = if let ActionStatus::Cooldown = status {
            true
        } else {
            false
        };
        let is_active = if let ActionStatus::Active(_) = status {
            true
        } else {
            false
        };

        if is_cooldown || is_active {
            let t = self.get_timer_mut();
            t.tick(scaled_delta_time);

            if t.just_finished() {
                *t = Timer::from_seconds(cooldown_sec, false);
                *self.get_action_status_mut() = if is_cooldown {
                    ActionStatus::Ready
                } else {
                    ActionStatus::Cooldown
                };
            }
        }
    }
}

#[derive(Default, Component, Inspectable, PartialEq, Debug)]
pub enum PlayerAnimation {
    #[default]
    Idle,
    Walking,
    Running,
    Dashing,
    Celebrating,
    Shooting,
}

#[derive(Component, Inspectable)]
pub struct PlayerAnimationData {
    pub animation: PlayerAnimation,
    face_e: Entity,
    body_e: Entity,
    body_root_e: Entity,
}

#[derive(Component, Inspectable)]
pub struct PlayerAnimationBlock(pub f32);

#[derive(Component, Inspectable)]
pub struct Player {
    pub(crate) id: usize,
    pub(crate) aim_e: Entity,
    aim_charge_e: Entity,
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

pub fn is_left_player_id(id: usize) -> bool {
    id == 1
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerMovement {
    speed: f32,
    charging_speed: f32,
    easing_time: f32,
    time_to_max_speed: f32,
    last_non_zero_raw_dir: Vec2,
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerDash {
    status: ActionStatus<Vec2>,
    duration_sec: f32,
    cooldown_sec: f32,
    speed: f32,
    #[inspectable(ignore)]
    timer: Timer,
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerScore {
    pub(crate) points: u8,
    pub(crate) games: u8,
    // pub(crate) sets: u8,
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerAim {
    pub(crate) direction: Vec2,
}

#[derive(Component, Inspectable)]
pub struct AimSprite;

// nice2have: macro?
impl ActionTimer<Vec2> for PlayerDash {
    fn get_cooldown_sec(&self) -> f32 {
        self.cooldown_sec
    }

    fn get_timer_mut(&mut self) -> &mut Timer {
        &mut self.timer
    }

    fn get_action_status_mut(&mut self) -> &mut ActionStatus<Vec2> {
        &mut self.status
    }
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerSwing {
    pub(crate) status: ActionStatus<f32>,
    duration_sec: f32,
    cooldown_sec: f32,
    #[inspectable(ignore)]
    pub(crate) timer: Timer,
}

#[derive(Default, Component, Inspectable)]
pub struct TransformRotation(f32, f32);

impl PlayerSwing {
    pub fn start_cooldown(&mut self) {
        self.status = ActionStatus::Cooldown;
        self.timer = Timer::from_seconds(self.cooldown_sec, false);
    }
}

// nice2have: macro?
impl ActionTimer<f32> for PlayerSwing {
    fn get_cooldown_sec(&self) -> f32 {
        self.cooldown_sec
    }

    fn get_timer_mut(&mut self) -> &mut Timer {
        &mut self.timer
    }

    fn get_action_status_mut(&mut self) -> &mut ActionStatus<f32> {
        &mut self.status
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    movement: PlayerMovement,
    dash: PlayerDash,
    swing: PlayerSwing,
    score: PlayerScore,
}

impl PlayerBundle {
    fn new(id: usize, initial_dir: Vec2, aim_e: Entity, aim_charge_e: Entity) -> Self {
        Self {
            player: Player {
                id: id,
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
                duration_sec: 0.35,
                cooldown_sec: 0.35,
                ..Default::default()
            },
            score: PlayerScore {
                ..Default::default()
            },
        }
    }
}

const SWING_LABEL: &str = "swing";

pub struct ServingRegion(pub CourtRegion);

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system_to_stage(StartupStage::PostStartup, setup)
            .add_system(move_player.before(SWING_LABEL))
            .add_system(aim)
            .add_system(handle_swing_input.label(SWING_LABEL))
            .add_system(on_ball_bounced)
            .add_system(rotate)
            .add_system(update_aim_rotation)
            .add_system(animate.after(SWING_LABEL))
            .add_system(unblock_animation)
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .with_system(handle_action_cooldown::<PlayerDash, Vec2>)
                    .with_system(handle_action_cooldown::<PlayerSwing, f32>),
            );
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, region: Res<InitialRegion>) {
    for i in 1..=2 {
        let x = WIN_WIDTH / 4.;
        let x = if i == 1 { -x } else { x };
        let is_left = x < 0.;
        let mut player_y = 150.;
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
                direction: initial_dir,
                ..Default::default()
            })
            .with_children(|b| {
                // aim arrow
                b.spawn_bundle(SpriteBundle {
                    texture: asset_server.load("art-ish/aim_arrow.png"),
                    transform: Transform::from_xyz(0., 135., -0.4),
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
            .id();

        let _player_e = commands
            .spawn_bundle(TransformBundle::from_xyz(x, player_y, PLAYER_Z))
            .insert_bundle(PlayerBundle::new(i, initial_dir, aim_e, aim_charge_e))
            .insert(RigidBody::KinematicPositionBased)
            .insert(CollisionShape::Sphere { radius: 100. })
            .insert(CollisionLayers::none())
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
                    ..Default::default()
                })
                .insert(PaletteColor::PlayerAim)
                .insert(AimSprite)
                .insert(TransformRotation(
                    rotation_speed.to_radians(),
                    rotation_speed.to_radians(),
                ));

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
                                    ..Default::default()
                                })
                                .insert(PaletteColor::Player)
                                .insert(Animator::<Transform>::default())
                                .insert(Name::new("player_body"))
                                .with_children(|b| {
                                    // shadow
                                    b.spawn_bundle(SpriteBundle {
                                        texture: asset_server.load("art-ish/player_body.png"),
                                        transform: Transform {
                                            scale: Vec3::new(1.0, 0.5, 1.),
                                            translation: Vec3::new(-5., -30., -PLAYER_Z + SHADOW_Z),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    })
                                    .insert(PaletteColor::Shadow)
                                    .insert(Name::new("player_shadow"));
                                })
                                .id(),
                            );
                        })
                        .insert(Animator::<Transform>::default())
                        .id(),
                );
            })
            .insert(PlayerAnimationData {
                animation: PlayerAnimation::Idle,
                face_e,
                body_e: body_e.unwrap(),
                body_root_e: body_root_e.unwrap(),
            })
            .id();
    }
}

// nice2have: lerp dash
fn move_player(
    input: Res<PlayerInput>,
    mut query: Query<(
        &Player,
        &mut PlayerMovement,
        &mut PlayerDash,
        &mut Transform,
        &PlayerSwing,
        &mut PlayerAnimationData,
    )>,
    aim_q: Query<(&PlayerAim, &Parent)>,
    net_q: Query<&GlobalTransform, With<Net>>,
    time: ScaledTime,
    net_offset: Res<NetOffset>,
) {
    for (p_aim, parent) in aim_q.iter() {
        if let Ok((
            player,
            mut player_movement,
            mut player_dash,
            mut player_t,
            player_swing,
            mut p_anim,
        )) = query.get_mut(parent.0)
        {
            let dir_raw = input.get_xy_axes_raw(player.id, &InputAxis::MoveX, &InputAxis::MoveY);
            let swing_ready = matches!(player_swing.status, ActionStatus::Ready);
            let charging = swing_ready && input.held(player.id, InputAction::Swing);
            let speed = if charging {
                player_movement.charging_speed
            } else {
                player_movement.speed
            };
            let mut dashing = false;
            let dir = if dir_raw != Vec2::ZERO {
                dir_raw
            } else {
                player_movement.last_non_zero_raw_dir
            };
            let mut move_by = (dir * speed).to_vec3();

            if input.just_pressed(player.id, InputAction::Dash) {
                if let ActionStatus::Ready = player_dash.status {
                    let dir = dir_raw.normalize_or_zero();
                    player_dash.status = ActionStatus::Active(if dir != Vec2::ZERO {
                        dir
                    } else {
                        p_aim.direction
                    });
                    player_dash.timer = Timer::from_seconds(player_dash.duration_sec, false);
                    p_anim.animation = PlayerAnimation::Dashing;
                    dashing = true;
                }
            }

            if let ActionStatus::Active(dash_dir) = player_dash.status {
                if !player_dash.timer.finished() {
                    move_by = (dash_dir * player_dash.speed).to_vec3();
                    dashing = true;
                } else {
                    p_anim.animation = PlayerAnimation::Idle;
                }
            } else if input.held(player.id, InputAction::LockPosition) {
                move_by = Vec3::ZERO;
            }

            let mut final_pos = player_t.translation + move_by * time.scaled_delta_seconds();

            if !dashing {
                // easing
                let ease_time_delta = if dir_raw == Vec2::ZERO {
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
                player_movement.easing_time = player_movement.time_to_max_speed;
            }

            // nice2have: get/store properly
            let player_size = Vec2::splat(80.);
            let is_left = player.is_left();
            // nice2have: get (from resource or component)
            let player_area_size = if is_left {
                Vec2::new(WIN_WIDTH / 2. + net_offset.0, WIN_HEIGHT)
            } else {
                Vec2::new(WIN_WIDTH / 2. - net_offset.0, WIN_HEIGHT)
            };
            let pos_offset = Vec3::new(player_area_size.x / 2., 0., 0.);
            let player_area_pos = if is_left {
                Vec3::X * net_offset.0 - pos_offset
            } else {
                Vec3::X * net_offset.0 + pos_offset
            };

            let coll = collide(final_pos, player_size, player_area_pos, player_area_size);

            if coll.is_some() {
                player_movement.easing_time = 0.;
                player_movement.last_non_zero_raw_dir = Vec2::ZERO;

                // nice2have: using colliders would probably make more sense
                // need to handle side coll in case the player gets pushed by a moving net

                if let Ok(net_t) = net_q.get_single() {
                    let player_x = player_t.translation.x;
                    let player_half_w = player_size.x / 2.;
                    let net_x = net_t.translation.x;

                    if is_left && (player_x + player_half_w) > net_x {
                        player_t.translation.x = net_x - player_half_w;
                    } else if !is_left && (player_x - player_half_w) < net_x {
                        player_t.translation.x = net_x + player_half_w;
                    }
                }

                if p_anim.animation != PlayerAnimation::Idle {
                    p_anim.animation = PlayerAnimation::Idle;
                }

                trace!("{}: {:?}", if is_left { "LeftP" } else { "RightP" }, coll);
            } else {
                if (final_pos - player_t.translation).length().abs() > 0.1 {
                    if !dashing {
                        if charging && p_anim.animation != PlayerAnimation::Walking {
                            p_anim.animation = PlayerAnimation::Walking;
                        } else if !charging && p_anim.animation != PlayerAnimation::Running {
                            p_anim.animation = PlayerAnimation::Running;
                        }
                    }
                } else {
                    if p_anim.animation != PlayerAnimation::Idle {
                        p_anim.animation = PlayerAnimation::Idle;
                    }
                }

                player_t.translation = final_pos;

                if dir_raw != Vec2::ZERO {
                    player_movement.last_non_zero_raw_dir = dir_raw;
                }
            }
        }
    }
}

fn aim(
    input: Res<PlayerInput>,
    player_q: Query<(&Player, &PlayerAnimationData, &PlayerSwing)>,
    mut aim_q: Query<(&mut PlayerAim, &mut Transform, &Parent)>,
    mut transform_q: Query<&mut Transform, Without<PlayerAim>>,
    time: ScaledTime,
) {
    for (mut aim, mut aim_t, aim_parent) in aim_q.iter_mut() {
        if let Ok((p, p_anim, player_swing)) = player_q.get(aim_parent.0) {
            // start with aim dir
            let mut dir_raw = input.get_xy_axes_raw(p.id, &InputAxis::AimX, &InputAxis::AimY);
            if dir_raw == Vec2::ZERO {
                // fallback to movement dir
                dir_raw = input.get_xy_axes_raw(p.id, &InputAxis::MoveX, &InputAxis::MoveY);
            }

            let mut dir = dir_raw.normalize_or_zero();

            // swing charge UI
            if let Ok(mut t) = transform_q.get_mut(p.aim_charge_e) {
                if let ActionStatus::Ready = player_swing.status {
                    if let Some(ActionState::Held(action_data)) =
                        input.get_button_action_state(p.id, &InputAction::Swing)
                    {
                        let scale = get_swing_multiplier(action_data.duration);
                        t.scale = Vec2::splat(scale).extend(1.);
                    }
                } else if let ActionStatus::Active(_) = player_swing.status {
                } else {
                    t.scale =
                        Vec2::splat((t.scale.x - (time.scaled_delta_seconds() * 3.)).clamp(0., 1.))
                            .extend(1.);
                }
            }

            if dir == Vec2::ZERO {
                continue;
            }

            let clamp_x = 1.;
            let clamp_y = 0.8;
            let player_x_sign = p.get_sign();

            if dir == Vec2::new(player_x_sign, 0.) {
                // player aiming into their court/backwards - just aim straight
                dir = Vec2::new(-player_x_sign, 0.);
            } else if player_x_sign < 0. {
                dir = dir.clamp(Vec2::new(clamp_x, -clamp_y), Vec2::new(clamp_x, clamp_y));
            } else {
                dir = dir.clamp(Vec2::new(-clamp_x, -clamp_y), Vec2::new(-clamp_x, clamp_y));
            }

            // nice2have: extract this to extensions
            let target_rotation = Quat::from_axis_angle(-Vec3::Z, dir.angle_between(Vec2::Y));
            let limit = 260f32.to_radians() * time.scaled_delta_seconds() * dir_raw.length();
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
            aim.direction = clamped_dir.truncate();

            if let Ok(mut face_t) = transform_q.get_mut(p_anim.face_e) {
                let axis = if p.is_left() { Vec2::X } else { -Vec2::X };
                face_t.rotation =
                    Quat::from_axis_angle(-Vec3::Z, aim.direction.angle_between(axis) * 0.25);
            }
        }
    }
}

// nice2have: on swing down cancel prev swing?
fn handle_swing_input(
    _commands: Commands,
    input: Res<PlayerInput>,
    mut query: Query<(
        Entity,
        &Player,
        &mut PlayerSwing,
        &mut CollisionLayers,
        &mut PlayerAnimationData,
    )>,
) {
    for (_e, player, mut player_swing, mut coll_layers, mut anim) in query.iter_mut() {
        if let Some(ActionState::Released(key_data)) =
            input.get_button_action_state(player.id, &InputAction::Swing)
        {
            if let ActionStatus::Ready = player_swing.status {
                player_swing.status =
                    ActionStatus::Active(get_swing_mutliplier_clamped(key_data.duration));
                player_swing.timer = Timer::from_seconds(player_swing.duration_sec, false);
                *coll_layers = CollisionLayers::all::<PhysLayer>();

                anim.animation = PlayerAnimation::Shooting;
                // commands.entity(e).insert(component)
            }
        } else {
            match player_swing.status {
                ActionStatus::Ready | ActionStatus::Cooldown => {
                    *coll_layers = CollisionLayers::none();
                }
                _ => {}
            }
        }
    }
}

fn get_swing_mutliplier_clamped(duration: f32) -> f32 {
    get_swing_multiplier(duration).clamp(0.4, 1.)
}

fn get_swing_multiplier(duration: f32) -> f32 {
    ((duration * 1.8).sin().abs() * 1.15).min(1.)
}

fn handle_action_cooldown<T: ActionTimer<TActiveData> + Component, TActiveData: Default>(
    mut query: Query<&mut T>,
    time: ScaledTime,
) {
    for mut activity in query.iter_mut() {
        activity.handle_action_timer(time.scaled_delta());
    }
}

fn on_ball_bounced(
    mut commands: Commands,
    mut ev_r_ball_bounced: EventReader<BallBouncedEvt>,
    player_q: Query<&Player>,
    mut ball_q: Query<(&Ball, &mut BallStatus, &Transform)>,
    asset_server: Res<AssetServer>,
    mut serving_region: ResMut<ServingRegion>,
    entity_q: Query<Entity>,
    mut score: ResMut<Score>,
    court_set: Res<CourtSettings>,
) {
    for ev in ev_r_ball_bounced.iter() {
        if let Ok((ball, mut status, ball_t)) = ball_q.get_mut(ev.ball_e.clone()) {
            let ball_res = match *status {
                BallStatus::Fault(count, player_id) => {
                    // tofix: rarely a double fault is a false positive
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
                        let player = player_q
                            .iter()
                            .filter(|p| p.side == ev.side)
                            .nth(0)
                            .unwrap();

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
                    swap_serve = add_point_to_score(&mut score, !is_left_player_id(losing_player));
                    debug!(
                        "Player {} has lost a point to {}! (bounce_count: {})",
                        losing_player, reason, ev.bounce_count
                    );
                }

                *status = BallStatus::Used;
                commands.entity(ev.ball_e).insert(Animator::new(
                    Tween::new(
                        EaseFunction::QuadraticIn,
                        TweeningType::Once,
                        Duration::from_millis(450),
                        TransformScaleLens {
                            start: ball_t.scale,
                            end: Vec3::ZERO,
                        },
                    )
                    .with_completed_event(true, TweenDoneAction::DespawnRecursive.into()),
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

fn animate(
    mut commands: Commands,
    player_anim_q: Query<(
        Entity,
        &PlayerAnimationData,
        Option<&PlayerAnimationBlock>,
        ChangeTrackers<PlayerAnimationData>,
    )>,
    mut animator_q: Query<(&mut Animator<Transform>, &Transform)>,
) {
    for (anim_e, anim, block, anim_tracker) in player_anim_q.iter() {
        if anim_tracker.is_changed() || anim_tracker.is_added() {
            if block.is_some() {
                continue;
            }

            let mut stop_anim_entities: Vec<Entity> = Vec::new();
            let mut body_root_tween = None;

            debug!("anim change to {:?}", anim.animation);
            match anim.animation {
                PlayerAnimation::Shooting => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_root_e);

                    if let Ok((mut animator, t)) = animator_q.get_mut(anim.body_e) {
                        let (tween, dur) = get_body_scale_tween(t, 1.8, 300);
                        animator.set_tweenable(tween);
                        animator.rewind();
                        animator.state = AnimatorState::Playing;

                        commands.entity(anim_e).insert(PlayerAnimationBlock(dur));
                    }
                }
                PlayerAnimation::Dashing => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_root_e);

                    if let Ok((mut animator, t)) = animator_q.get_mut(anim.body_e) {
                        let (tween, dur) = get_body_scale_tween(t, 1.3, 220);
                        animator.set_tweenable(tween);
                        animator.rewind();
                        animator.state = AnimatorState::Playing;

                        commands.entity(anim_e).insert(PlayerAnimationBlock(dur));
                    }
                }
                PlayerAnimation::Idle => {
                    stop_anim_entities.push(anim.body_root_e);

                    if let Ok((mut animator, t)) = animator_q.get_mut(anim.face_e) {
                        animator.set_tweenable(get_idle_face_tween(t.translation.z));
                        animator.rewind();
                        animator.state = AnimatorState::Playing;
                    }

                    if let Ok((mut animator, t)) = animator_q.get_mut(anim.body_e) {
                        animator.set_tweenable(get_idle_body_tween(t.translation.z));
                        animator.rewind();
                        animator.state = AnimatorState::Playing;
                    }
                }
                PlayerAnimation::Walking => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    body_root_tween = Some(get_move_tween(400, 4., 3.));
                }
                PlayerAnimation::Running => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    body_root_tween = Some(get_move_tween(300, 5., 8.));
                }
                PlayerAnimation::Celebrating => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    body_root_tween = Some(get_move_tween(500, 20., 12.));
                }
            }

            for e in stop_anim_entities.iter() {
                if let Ok((mut animator, t)) = animator_q.get_mut(*e) {
                    animator.set_tweenable(get_reset_trans_tween(t, 250));
                    animator.rewind();
                    animator.state = AnimatorState::Playing;
                }
            }

            if let Some(move_tween) = body_root_tween {
                if let Ok((mut animator, _t)) = animator_q.get_mut(anim.body_root_e) {
                    animator.set_tweenable(move_tween);
                    animator.state = AnimatorState::Playing;
                }
            }
        }
    }
}

fn unblock_animation(
    mut commands: Commands,
    mut block_q: Query<(Entity, &mut PlayerAnimationBlock)>,
    time: ScaledTime,
) {
    for (e, mut block) in block_q.iter_mut() {
        block.0 -= time.scaled_delta_seconds();

        if block.0 < 0. {
            commands.entity(e).remove::<PlayerAnimationBlock>();
        }
    }
}

fn get_move_tween(walk_cycle_ms: u64, pos_y: f32, rot: f32) -> Tracks<Transform> {
    let body_walk_pos_tween = Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(walk_cycle_ms / 2),
        TransformPositionLens {
            start: Vec3::ZERO,
            end: Vec3::new(0., pos_y, 0.),
        },
    );
    let body_walk_rotation_tween = Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(walk_cycle_ms),
        TransformRotationLens {
            start: Quat::from_rotation_z(-rot.to_radians()),
            end: Quat::from_rotation_z(rot.to_radians()),
        },
    );

    Tracks::new([body_walk_pos_tween, body_walk_rotation_tween])
}

fn get_reset_trans_tween(transform: &Transform, duration_ms: u64) -> Tracks<Transform> {
    let pos_tween = get_reset_tween(
        duration_ms,
        TransformPositionLens {
            start: transform.translation,
            end: Vec3::new(0., 0., transform.translation.z),
        },
    );

    let scale_tween = get_reset_tween(
        duration_ms,
        TransformScaleLens {
            start: transform.scale,
            end: Vec3::ONE,
        },
    );

    let rot_tween = get_reset_tween(
        duration_ms,
        TransformRotationLens {
            start: transform.rotation,
            end: Quat::IDENTITY,
        },
    );

    Tracks::new([pos_tween, scale_tween, rot_tween])
}

fn get_reset_tween<T, L: Lens<T> + Send + Sync + 'static>(duration_ms: u64, lens: L) -> Tween<T> {
    Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::Once,
        Duration::from_millis(duration_ms),
        lens,
    )
}

fn get_idle_face_tween(z: f32) -> Tween<Transform> {
    Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(400),
        TransformPositionLens {
            start: Vec3::ZERO,
            end: Vec3::new(0., -4., z),
        },
    )
}

fn get_idle_body_tween(z: f32) -> Tracks<Transform> {
    let body_idle_size_tween = Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(400),
        TransformScaleLens {
            start: Vec3::ONE,
            end: Vec3::new(1.075, 0.925, 1.),
        },
    );
    let body_idle_pos_tween = Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(400),
        TransformPositionLens {
            start: Vec2::ZERO.extend(z),
            end: Vec3::new(0., -4., z),
        },
    );

    Tracks::new([body_idle_size_tween, body_idle_pos_tween])
}

fn get_body_scale_tween(transform: &Transform, scale: f32, dur: u64) -> (Sequence<Transform>, f32) {
    let end = (Vec2::ONE * scale).extend(1.);
    let t = Tween::new(
        EaseFunction::QuadraticOut,
        TweeningType::Once,
        Duration::from_millis(dur / 2),
        TransformScaleLens {
            start: transform.scale,
            end: end,
        },
    )
    .then(Tween::new(
        EaseFunction::QuadraticIn,
        TweeningType::Once,
        Duration::from_millis(dur / 2),
        TransformScaleLens {
            start: end,
            end: Vec3::ONE,
        },
    ));
    (t, 0.5)
}

fn rotate(mut q: Query<(&TransformRotation, &mut Transform)>, time: ScaledTime) {
    for (r, mut t) in q.iter_mut() {
        t.rotate(Quat::from_rotation_z(r.0 * time.scaled_delta_seconds()));
    }
}

fn update_aim_rotation(
    mut q: Query<(&Parent, &mut TransformRotation), With<AimSprite>>,
    dash_q: Query<&PlayerDash>,
    time: ScaledTime,
) {
    for (parent, mut rot) in q.iter_mut() {
        if let Ok(dash) = dash_q.get(parent.0) {
            match dash.status {
                ActionStatus::Ready => {
                    rot.0 += time.scaled_delta_seconds() * rot.1 * 3.;
                    if rot.0.abs() > rot.1.abs() {
                        rot.0 = rot.1;
                    }
                }
                ActionStatus::Active(_) => {
                    let mult = 1. - dash.timer.percent();
                    rot.0 = rot.1 * mult * rot.1.signum();
                }
                ActionStatus::Cooldown => rot.0 = 0.,
            };
        }
    }
}
