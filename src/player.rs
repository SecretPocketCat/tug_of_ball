use std::{default, time::Duration};

use bevy::{prelude::*, sprite::{SpriteBundle, Sprite}, math::Vec2};
use bevy_extensions::Vec2Conversion;
use bevy_input::{ActionInput, ActionState};
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use heron::rapier_plugin::{PhysicsWorld, rapier2d::prelude::{RigidBodyActivation, ColliderSet}};
use heron::*;

use crate::{InputAction, InputAxis, PlayerInput, WIN_WIDTH, PhysLayer, ball::{BallBouncedEvt, Ball, BallScorable, spawn_ball}};

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

#[derive(Default, Component, Inspectable)]
pub struct Player {
    pub(crate) id: usize,
    side: f32,
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerMovement {
    speed: f32,
    charging_speed: f32,
    pub(crate) last_dir: Vec2,
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
    pub(crate) score: usize,
}

// todo: macro?
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

impl PlayerSwing {
    pub fn start_cooldown(&mut self) {
        self.status = ActionStatus::Cooldown;
        self.timer = Timer::from_seconds(self.cooldown_sec, false);
    }
}

// todo: macro?
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
    fn new(id: usize, initial_dir: Vec2) -> Self {
        Self {
            player: Player { 
                id: id,
                side: -initial_dir.x.signum(),
                ..Default::default()
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

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app
            .add_startup_system(setup)
            .add_system(movement)
            .add_system(handle_swing_input)
            .add_system(on_ball_bounced)
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
        let size = Vec2::splat(50.);
        let is_left = x < 0.;
        let entity = commands.spawn_bundle(SpriteBundle {
            // texture: asset_server.load("icon.png"),
            transform: Transform::from_xyz(x, 0., 0.),
            sprite: Sprite {
                color: if i == 1 { Color::RED } else { Color::MIDNIGHT_BLUE },
                custom_size: Some(size),
                ..Default::default()
            },
            ..Default::default()
        }).insert_bundle(PlayerBundle::new(i, if is_left { Vec2::X } else { -Vec2::X }))
        .insert(RigidBody::KinematicPositionBased)
        .insert(CollisionShape::Sphere {
            radius: 100.,
        })
        .insert(CollisionLayers::none())
        .insert(Name::new("Player"))
        .id();

        if is_left {
            left = Some(entity);
        }
        else {
            right = Some(entity);
        }
    }

    commands.insert_resource(Players {
        left: left.unwrap(),
        right: right.unwrap(),
    });
}

// todo: movement easing
// possibly during dash as well
fn movement(
    input: Res<PlayerInput>,
    mut query: Query<(&Player, &mut PlayerMovement, &mut PlayerDash, &mut Transform, &PlayerSwing)>,
    time: ScaledTime,
) {
    for (player, mut player_movement, mut player_dash, mut t, player_swing) in query.iter_mut() {
        let dir = input.get_xy_axes(player.id, &InputAxis::X, &InputAxis::Y);
        let swing_ready = if let ActionStatus::Ready = player_swing.status { true } else { false };
        let speed = if swing_ready && input.held(player.id, InputAction::Swing) { player_movement.charging_speed } else { player_movement.speed };
        let mut move_by = (dir * speed * time.scaled_delta_seconds()).to_vec3();

        if input.just_pressed(player.id, InputAction::Dash) {
            if let ActionStatus::Ready = player_dash.status {
                player_dash.status = ActionStatus::Active(if dir != Vec2::ZERO { dir } else { player_movement.last_dir });
                player_dash.timer = Timer::from_seconds(player_dash.duration_sec, false);
            }
        }

        if let ActionStatus::Active(dir) = player_dash.status {
            if !player_dash.timer.finished() {
                move_by = (dir * player_dash.speed * time.scaled_delta_seconds()).to_vec3();
            }
        }

        let res_pos = t.translation + move_by;
        if res_pos.x.signum() == t.translation.x.signum() {
            if move_by.truncate() != Vec2::ZERO {
                t.translation += move_by;
                player_movement.last_dir = move_by.truncate().normalize_or_zero();
            }
        }
    }
}

// todo: on swing down cancel prev swing
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
    ball_score_q: Query<Entity, With<BallScorable>>,
    asset_server: Res<AssetServer>,
    // players: Res<Players>,
) {
    for ev in ev_r_ball_bounced.iter() {
        if ev.bouce_count > 1 {
            let (scoring_player, mut score) = player_q
                .iter_mut()
                .filter(|p| p.0.side == -ev.side)
                .nth(0)
                .unwrap();

            if let Ok(_) = ball_score_q.get(ev.ball_e.clone()) {
                spawn_ball(&mut commands, &asset_server);

                score.score += 1;
                commands.entity(ev.ball_e).remove::<BallScorable>();
                commands.entity(ev.ball_e).remove::<CollisionShape>();
                // todo: tween out and destroy the ball
                debug!("Player {} has lost a point to too many bounces!", scoring_player.id);
            }
        }
    }
}
