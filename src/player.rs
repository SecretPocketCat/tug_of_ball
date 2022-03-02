use std::{default, time::Duration, f32::consts::PI, ops::Add};

use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2, render::render_resource::{Texture, FilterMode}};
use bevy_extensions::Vec2Conversion;
use bevy_input::{ActionInput, ActionState};
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::{TransformRotationLens, TransformScaleLens, TransformPositionLens};
use bevy_tweening::*;
use heron::rapier_plugin::{PhysicsWorld, rapier2d::prelude::{RigidBodyActivation, ColliderSet}};
use heron::*;

use crate::{InputAction, InputAxis, PlayerInput, WIN_WIDTH, PhysLayer, ball::{BallBouncedEvt, spawn_ball, BallStatus, Ball}, level::CourtRegion, TransformBundle};

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
        let is_cooldown = if let ActionStatus::Cooldown = status { true } else { false };
        let is_active = if let ActionStatus::Active(_) = status { true } else { false };

        if is_cooldown || is_active {
            let t = self.get_timer_mut();
            t.tick(scaled_delta_time);

            if t.just_finished() {
                *t = Timer::from_seconds(cooldown_sec, false);
                *self.get_action_status_mut() = if is_cooldown { ActionStatus::Ready } else { ActionStatus::Cooldown };
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
}

#[derive(Component, Inspectable)]
struct PlayerAnimationData {
    animation: PlayerAnimation,
    face_e: Entity,
    body_e: Entity,
    body_root_e: Entity,
}

#[derive(Component, Inspectable)]
pub struct Player {
    pub(crate) id: usize,
    side: f32,
}

impl Player {
    pub fn is_left(&self) -> bool {
        self.id == 1
    }

    pub fn get_sign(&self) -> f32 {
        if self.is_left() { -1. } else { 1. }
    }
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerMovement {
    speed: f32,
    charging_speed: f32,
    pub(crate) last_dir: Vec2,
    movement_ease: f32,
    rotation_ease: f32,
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
    direction: Vec2,
}

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
pub struct TransformRotation(f32);

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
    fn new(
        id: usize,
        initial_dir: Vec2) -> Self {
        Self {
            player: Player { 
                id: id,
                side: -initial_dir.x.signum(),
            },
            movement: PlayerMovement {
                last_dir: initial_dir,
                speed: 550.,
                charging_speed: 125.,
                ..Default::default()
            },
            dash: PlayerDash {
                speed: 2200.,
                duration_sec: 0.085,
                cooldown_sec: 0.4,
                ..Default::default()
            },
            swing: PlayerSwing {
                duration_sec: 0.35,
                cooldown_sec: 0.35,
                ..Default::default()
            },
            score: PlayerScore {
                ..Default::default()
            }
        }
    }
}

pub struct Players {
    left: Entity,
    right: Entity,
}

pub struct ServingRegion(pub CourtRegion);

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
            .add_system(move_player)
            .add_system(aim)
            .add_system(handle_swing_input)
            .add_system(on_ball_bounced)
            .add_system(rotate)
            .add_system(animate)
            .add_system_set_to_stage(
                CoreStage::PostUpdate, 
                SystemSet::new()
                    .with_system(handle_action_cooldown::<PlayerDash, Vec2>)
                    .with_system(handle_action_cooldown::<PlayerSwing, f32>)
            );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mut left = None;
    let mut right = None;

    for i in 1..=2 {
        let x = WIN_WIDTH / 2.- 100.;
        let x = if i == 1 { -x } else { x }; 
        let is_left = x < 0.;
        let mut body_e = None;
        let mut body_root_e = None;

        // face
        let face_e = commands.spawn_bundle(SpriteBundle {
            texture: asset_server.load("art-ish/face_happy.png"),
            sprite: Sprite {
                flip_x: !is_left,
                ..Default::default()
            },
            ..Default::default()
        }).insert(Animator::<Transform>::default())
        .id();

        let player_e = commands
        .spawn_bundle(TransformBundle::from_xyz(x, 0., 0.))
        .insert_bundle(PlayerBundle::new(
            i, 
            if is_left { Vec2::X } else { -Vec2::X },
        ))
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Sphere {
            radius: 100.,
        })
        .insert(CollisionLayers::none())
        .insert(Name::new("Player"))
        .with_children(|b| {
            // circle
            let rotation_speed: f32 = 15.0;
            let rotation_speed = if is_left { -rotation_speed } else { rotation_speed };
            b.spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/player_circle.png"),
                sprite: Sprite {
                    color: Color::rgba(1., 1., 1., 0.5),
                    ..Default::default()
                },
                ..Default::default()
            }).insert(TransformRotation(rotation_speed.to_radians()));

            // aim
            b.spawn_bundle(TransformBundle::default())
            .insert(PlayerAim::default())
            .with_children(|b| {
                // aim arrow
                b.spawn_bundle(SpriteBundle {
                    texture: asset_server.load("art-ish/aim_arrow.png"),
                    transform: Transform::from_xyz(0., 135., 0.),
                    sprite: Sprite {
                        color: Color::rgba(1., 1., 1., 0.5),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });

            // shadow
            b.spawn_bundle(SpriteBundle {
                texture: asset_server.load("art-ish/player_body.png"),
                sprite: Sprite {
                    color: Color::rgba(0., 0., 0., 0.5),
                    ..Default::default()
                },
                transform: Transform {
                    scale: Vec3::new(1.0, 0.5, 2.),
                    translation: Vec3::Y * -25.,
                    ..Default::default()
                },
                ..Default::default()
            });

            // body root
            body_root_e = Some(b.spawn_bundle(TransformBundle::from_xyz(0., 0., 3.))
            .add_child(face_e)
            .with_children(|b| {
                // body
                body_e = Some(b.spawn_bundle(SpriteBundle {
                    texture: asset_server.load("art-ish/player_body.png"),
                    ..Default::default()
                }).insert(Animator::<Transform>::default())
                .id());
            }).insert(Animator::<Transform>::default())
            .id());
        }).insert(PlayerAnimationData {
            animation: PlayerAnimation::Idle,
            face_e,
            body_e: body_e.unwrap(),
            body_root_e: body_root_e.unwrap(),
        })
        .id();

        if is_left {
            left = Some(player_e);
        }
        else {
            right = Some(player_e);
        }
    }

    commands.insert_resource(Players {
        left: left.unwrap(),
        right: right.unwrap(),
    });
}

// todo: movement lerp + rotation lerp
// possibly during dash as well
fn move_player(
    input: Res<PlayerInput>,
    mut query: Query<(&Player, &mut PlayerMovement, &mut PlayerDash, &mut Transform, &PlayerSwing, &mut PlayerAnimationData)>,
    time: ScaledTime,
) {
    for (player, mut player_movement, mut player_dash, mut t, player_swing, mut p_anim) in query.iter_mut() {
        let dir = input.get_xy_axes(player.id, &InputAxis::X, &InputAxis::Y);
        let swing_ready = matches!(player_swing.status, ActionStatus::Ready);
        let charging = swing_ready && input.held(player.id, InputAction::Swing);
        let speed = if charging { player_movement.charging_speed } else { player_movement.speed };
        let mut move_by = (dir * speed).to_vec3();
        let mut dashing = false;

        if input.just_pressed(player.id, InputAction::Dash) {
            if let ActionStatus::Ready = player_dash.status {
                player_dash.status = ActionStatus::Active(if dir != Vec2::ZERO { dir } else { player_movement.last_dir });
                player_dash.timer = Timer::from_seconds(player_dash.duration_sec, false);
                p_anim.animation = PlayerAnimation::Dashing;
                dashing = true;
            }
        }

        if let ActionStatus::Active(dir) = player_dash.status {
            if !player_dash.timer.finished() {
                move_by = (dir * player_dash.speed).to_vec3();
                dashing = true;
            }
            else {
                p_anim.animation = PlayerAnimation::Idle;
            }
        }

        let res_pos = t.translation + move_by * time.scaled_delta_seconds();
        if res_pos.x.signum() == t.translation.x.signum() {
            if move_by.truncate() != Vec2::ZERO {
                t.translation = res_pos;
                player_movement.last_dir = move_by.truncate().normalize_or_zero();

                if !dashing {
                    if charging && p_anim.animation != PlayerAnimation::Walking {
                        p_anim.animation = PlayerAnimation::Walking;
                    }
                    else if !charging && p_anim.animation != PlayerAnimation::Running {
                        p_anim.animation = PlayerAnimation::Running;
                    }
                }
            }
            else if p_anim.animation != PlayerAnimation::Idle {
                p_anim.animation = PlayerAnimation::Idle;
            }
        }
    }
}

fn aim(
    input: Res<PlayerInput>,
    player_q: Query<(&Player, &PlayerMovement, &PlayerAnimationData)>,
    mut aim_q: Query<(&mut PlayerAim, &mut Transform, &Parent)>,
    mut face_q: Query<&mut Transform, Without<PlayerAim>>,
    time: ScaledTime,
) {
    for (aim, mut aim_t, aim_parent) in aim_q.iter_mut() {
        if let Ok((p, p_movement, p_anim)) = player_q.get(aim_parent.0) {
            let mut input_dir = input.get_xy_axes(p.id, &InputAxis::X, &InputAxis::Y);

            if input_dir == Vec2::ZERO {
                input_dir = p_movement.last_dir;
            }
    
            let clamp_x = 1.;
            let clamp_y = 0.8;
            let player_x_sign = p.get_sign();
    
            if input_dir == Vec2::new(player_x_sign, 0.) {
                // player aiming into their court/backwards - just aim straight
                input_dir = Vec2::new(-player_x_sign, 0.);
            }
            else if player_x_sign < 0. {
                input_dir = input_dir.clamp(Vec2::new(clamp_x, -clamp_y), Vec2::new(clamp_x, clamp_y));
            }
            else {
                input_dir = input_dir.clamp(Vec2::new(-clamp_x, -clamp_y), Vec2::new(-clamp_x, clamp_y));
            }

            // todo: extract this to extensions
            aim_t.rotation = Quat::from_axis_angle(-Vec3::Z, input_dir.angle_between(Vec2::Y));

            if let Ok(mut face_t) = face_q.get_mut(p_anim.face_e) {
                let axis = if p.is_left() { Vec2::X } else { -Vec2::X };
                face_t.rotation = Quat::from_axis_angle(-Vec3::Z, input_dir.angle_between(axis) * 0.25);
            }
        }
    }
}

// todo: on swing down cancel prev swing?
fn handle_swing_input(
    input: Res<ActionInput<InputAction, InputAxis>>,
    mut query: Query<(&Player, &mut PlayerSwing, &mut CollisionLayers)>,
) {
    for (player, mut player_swing, mut coll_layers) in query.iter_mut() {
        if let Some(ActionState::Released(key_data)) = input.get_button_action_state(player.id, &InputAction::Swing) {
            if let ActionStatus::Ready = player_swing.status {
                player_swing.status = ActionStatus::Active((key_data.duration * 3.0).clamp(0.4, 1.));
                player_swing.timer = Timer::from_seconds(player_swing.duration_sec, false);
                *coll_layers = CollisionLayers::all::<PhysLayer>();
            }
        }
        else {
            match player_swing.status {
                ActionStatus::Ready | ActionStatus::Cooldown => {
                    *coll_layers = CollisionLayers::none();
                }
                _ => {}
            }
        }
    }
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
    mut player_q: Query<(&Player, &mut PlayerScore)>,
    mut ball_q: Query<(&Ball, &mut BallStatus)>,
    asset_server: Res<AssetServer>,
    mut serving_region: ResMut<ServingRegion>,
) {
    for ev in ev_r_ball_bounced.iter() {
        if let Ok((ball, mut status)) = ball_q.get_mut(ev.ball_e.clone()){
            let ball_res = match *status {
                BallStatus::Fault(count, player_id) => {
                    // tofix: rarely a double fault is a false positive
                    // nice2have: limit might come from an upgrade
                    let limit = 1;
                    let losing_player = if count > limit { Some(player_id) } else { None };
                    let fault_count = if count > limit { 0 } else { count };
                    Some((losing_player, fault_count, "double fault"))
                },
                BallStatus::Rally(player_id) => {
                    // nice2have: limit might come from an upgrade
                    let bounce_limit = 1;

                    // out of bounds
                    if ball.region.is_out_of_bounds() && ev.bounce_count == 1 {
                        Some((Some(player_id), 0, "shooting out of bounds"))
                    }
                    else if ev.bounce_count > bounce_limit {
                        let (player, _) = player_q
                            .iter()
                            .filter(|p| p.0.side == ev.side)
                            .nth(0)
                            .unwrap();

                        Some((Some(player.id), 0, "too many bounces"))
                    }
                    else {
                        None
                    }
                },
                BallStatus::Serve(..) | BallStatus::Used => None,
            };

            if let Some((losing_player, fault_count, reason)) = ball_res {
                let mut swap_serve = false;

                if let Some(losing_player) = losing_player {
                    let mut score = None;
                    let mut other_score = None;

                    for (p, s) in player_q.iter_mut() {
                        if p.id == losing_player {
                            other_score = Some(s);
                        }
                        else {
                            score = Some(s);
                        }
                    }

                    swap_serve = add_point(&mut score.unwrap(), &mut other_score.unwrap());
                    debug!("Player {} has lost a point to {}! (bounce_count: {})", losing_player, reason, ev.bounce_count);
                }

                *status = BallStatus::Used;
                commands.entity(ev.ball_e).despawn_recursive();
                // todo: tween out and destroy the ball

                if swap_serve {
                    serving_region.0 = if serving_region.0.is_left() { CourtRegion::get_random_right() } else { CourtRegion::get_random_left() };
                }
                spawn_ball(&mut commands, &asset_server, serving_region.0, fault_count, serving_region.0.get_player_id());
            }
        }
    }
}

fn add_point(score: &mut PlayerScore, other_player_score: &mut PlayerScore) -> bool {
    score.points += 1;

    let required_points = (other_player_score.points + 2).max(4);

    if score.points >= required_points {
        score.games += 1;
        score.points = 0;
        other_player_score.points = 0;
        return true
    }
    else if score.points == other_player_score.points && score.points > 3 {
        // hacky way to get ADV in the UI
        // todo: redo
        score.points = 3;
        other_player_score.points = 3;
    }

    if score.games >= 6 {
        // todo: game done event?
    }

    false
}

fn animate(
    player_q: Query<(&PlayerAnimationData, ChangeTrackers<PlayerAnimationData>)>,
    mut animator_q: Query<(&mut Animator<Transform>, &Transform)>,
) {
    for (anim, anim_tracker) in player_q.iter() {
        if anim_tracker.is_changed() || anim_tracker.is_added() {
            let mut stop_anim_entities: Vec::<Entity> = Vec::new();
            let mut body_root_tween = None;

            info!("anim change to {:?}", anim.animation);
            match anim.animation {
                // todo: set the proper face sprite for each anim

                PlayerAnimation::Dashing => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    stop_anim_entities.push(anim.body_root_e);
                    
                    // if let Ok((mut animator, t)) = animator_q.get_mut(anim.body_e) {
                    //     animator.set_tweenable(get_dash_tween(t));
                    //     animator.rewind();
                    //     animator.state = AnimatorState::Playing;
                    // }
                },
                PlayerAnimation::Idle => {
                    stop_anim_entities.push(anim.body_root_e);

                    if let Ok((mut animator, t)) = animator_q.get_mut(anim.face_e) {
                        animator.set_tweenable(get_idle_face_tween());
                        animator.rewind();
                        animator.state = AnimatorState::Playing;
                    }
                    
                    if let Ok((mut animator, t)) = animator_q.get_mut(anim.body_e) {
                        animator.set_tweenable(get_idle_body_tween());
                        animator.rewind();
                        animator.state = AnimatorState::Playing;
                    }
                },
                PlayerAnimation::Walking => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    body_root_tween = Some(get_move_tween(400, 4., 3.));
                },
                PlayerAnimation::Running => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    body_root_tween = Some(get_move_tween(300, 5., 8.));
                },
                PlayerAnimation::Celebrating => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    body_root_tween = Some(get_move_tween(500, 20., 12.));
                },
            }

            for e in stop_anim_entities.iter() {
                if let Ok((mut animator, t)) = animator_q.get_mut(*e) {
                    animator.set_tweenable(get_reset_trans_tween(t, 250));
                    animator.rewind();
                    animator.state = AnimatorState::Playing;
                }
            }

            if let Some(move_tween) = body_root_tween {
                if let Ok((mut animator, t)) = animator_q.get_mut(anim.body_root_e) {
                    animator.set_tweenable(move_tween);
                    animator.state = AnimatorState::Playing;
                }
            }
        }
    }
}

fn get_move_tween(
    walk_cycle_ms: u64,
    pos_y: f32,
    rot: f32,
) -> Tracks<Transform> {
    let body_walk_pos_tween = Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(walk_cycle_ms / 2),
        TransformPositionLens {
            start: Vec3::ZERO,
            end: Vec3::new(0., pos_y, 3.),
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

fn get_reset_trans_tween(
    transform: &Transform,
    duration_ms: u64,
) -> Tracks<Transform> {
    let pos_tween = get_reset_tween(
        duration_ms,
        TransformPositionLens {
        start: transform.translation,
        end: Vec3::new(1., 1., transform.translation.z),
    });

    let scale_tween = get_reset_tween(
        duration_ms,
        TransformScaleLens {
        start: transform.scale,
        end: Vec3::ONE,
    });

    let rot_tween = get_reset_tween(
        duration_ms,
        TransformRotationLens {
        start: transform.rotation,
        end: Quat::IDENTITY,
    });

    Tracks::new([pos_tween, scale_tween, rot_tween])
}

fn get_reset_tween<T, L: Lens<T> + Send + Sync + 'static>(
    duration_ms: u64,
    lens: L) -> Tween<T> {
    Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::Once,
        Duration::from_millis(duration_ms),
        lens,
    )
}

fn get_idle_face_tween() -> Tween<Transform> {
    let pos_lens = TransformPositionLens {
        start: Vec3::ZERO,
        end: Vec3::new(0., -4., 0.),
    };
    Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(400),
        pos_lens.clone(),
    )
}

fn get_idle_body_tween() -> Tracks<Transform> {
    let pos_lens = TransformPositionLens {
        start: Vec3::ZERO,
        end: Vec3::new(0., -4., 0.),
    };
    let body_idle_size_tween = Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(400),
        TransformScaleLens {
            start: Vec2::ONE.to_vec3(),
            end: Vec2::new(1.075, 0.925).to_vec3(),
        }
    );
    let body_idle_pos_tween = Tween::new(
        EaseFunction::QuadraticInOut,
        TweeningType::PingPong,
        Duration::from_millis(400),
        pos_lens.clone(),
    );
    
    Tracks::new([body_idle_size_tween, body_idle_pos_tween])
}

// fn get_dash_tween(
//     transform: &Transform
// ) -> Sequence<Transform> {
//     let dash_tween = Tween::new(
//         EaseFunction::QuadraticOut,
//         TweeningType::Once,
//         Duration::from_millis(150),
//         TransformRotationLens {
//             start: transform.rotation,
//             end: Quat::from_rotation_y(360f32.to_radians()),
//         }
//     );
    
//     dash_tween.then(get_reset_trans_tween(transform, 150))
// }

fn rotate(
    mut q: Query<(&TransformRotation, &mut Transform)>,
    time: ScaledTime,
) {
    for (r, mut t) in q.iter_mut() {
        t.rotate(Quat::from_rotation_z(r.0 * time.scaled_delta_seconds()));
    }
}
