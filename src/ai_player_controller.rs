use crate::{
    animation::inverse_lerp,
    ball::{Ball, BallHitEvt, BALL_MAX_SPEED, BALL_MIN_SPEED},
    level::{CourtSettings, InitialRegion, NetOffset},
    player::{spawn_player, Player, PlayerAim, PlayerMovement, PlayerSwing, AIM_RING_RADIUS},
    player_action::PlayerActionStatus,
    GameState,
};
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use big_brain::prelude::*;

pub struct AiPlayerControllerPlugin;
impl Plugin for AiPlayerControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(setup))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(collect_inputs))
            .add_system_to_stage(BigBrainStage::Actions, stand_still)
            .add_system_to_stage(BigBrainStage::Scorers, score_move_to_ball)
            .add_system_to_stage(BigBrainStage::Actions, move_to_ball_action)
            .add_system_to_stage(BigBrainStage::Scorers, score_move_to_center)
            .add_system_to_stage(BigBrainStage::Actions, move_to_center_action)
            .add_system_to_stage(BigBrainStage::Scorers, score_aim_to_center)
            .add_system_to_stage(BigBrainStage::Actions, aim_to_center_action)
            .add_system_to_stage(BigBrainStage::Scorers, score_swing)
            .add_system_to_stage(BigBrainStage::Actions, swing_action);
    }
}

#[derive(Debug, Clone, Component)]
pub struct AiPlayer;

#[derive(Component, Default, Inspectable)]
pub struct AiPlayerInputs {
    ball_is_approaching: bool,
    predicted_swing_pos: Vec2,
    dir_to_center: Vec2,
    distance_to_center: f32,
}

#[derive(Debug, Clone, Component)]
pub struct StandStillAction;

#[derive(Debug, Clone, Component)]
pub struct MoveToBallAction;

#[derive(Debug, Clone, Component)]
pub struct MoveToBallScorer;

#[derive(Debug, Clone, Component)]
pub struct MoveDiagonallyToPlayerAction;

#[derive(Debug, Clone, Component)]
pub struct MoveDiagonallyToPlayerScorer;

#[derive(Debug, Clone, Component)]
pub struct MoveToCenterAction;

#[derive(Debug, Clone, Component)]
pub struct MoveToCenterScorer;

#[derive(Debug, Clone, Component)]
pub struct MoveToOuterLineAction;

#[derive(Debug, Clone, Component)]
pub struct MoveToOuterLineScorer;

#[derive(Debug, Clone, Component)]
pub struct SwingScorer;

#[derive(Debug, Clone, Component)]
pub struct SwingAction;

#[derive(Debug, Clone, Component)]
pub struct AimToCenterScorer;

#[derive(Debug, Clone, Component)]
pub struct AimToCenterAction;

// what thinkers are needed?
// movement thinker
// aim thinker
// swing thinker
// dodge thinker

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, region: Res<InitialRegion>) {
    if cfg!(feature = "debug") {
        let move_thinker = Thinker::build()
            .picker(FirstToScore::new(0.2))
            .when(MoveToBallScorer, MoveToBallAction)
            // .when(MoveDiagonallyToPlayerScorer, MoveDiagonallyToPlayerAction)
            // .when(MoveToOuterLineScorer, MoveToOuterLineAction)
            .when(MoveToCenterScorer, MoveToCenterAction)
            // .otherwise(MoveToBallAction);
            .otherwise(StandStillAction);

        let swing_thinker = Thinker::build()
            .picker(FirstToScore::new(0.2))
            .when(SwingScorer, SwingAction);

        let aim_thinker = Thinker::build()
            .picker(FirstToScore::new(0.2))
            .when(AimToCenterScorer, AimToCenterAction);

        spawn_player(2, &mut commands, &asset_server, &region)
            .insert(AiPlayerInputs::default())
            .insert(AiPlayer)
            .insert(move_thinker)
            .with_children(|b| {
                b.spawn().insert(swing_thinker);
                b.spawn().insert(aim_thinker);
            });
    }
}

fn collect_inputs(
    mut ball_hit_er: EventReader<BallHitEvt>,
    mut ai_q: Query<(&mut AiPlayerInputs, &GlobalTransform, &Player), With<AiPlayer>>,
    ball_q: Query<(Entity, &Ball, &GlobalTransform), Without<AiPlayer>>,
    court: Res<CourtSettings>,
    net: Res<NetOffset>,
) {
    for ev in ball_hit_er.iter() {
        for (mut inputs, _p_t, p) in ai_q.iter_mut() {
            if let Ok((_ball_e, ball, _ball_t)) = ball_q.get(ev.ball_e) {
                inputs.ball_is_approaching =
                    (p.is_left() && ball.dir.x < 0.) || (!p.is_left() && ball.dir.x > 0.);
                inputs.predicted_swing_pos = ball.predicted_bounce_pos
                    + ball.dir
                        * (inverse_lerp(BALL_MIN_SPEED, BALL_MAX_SPEED, ball.speed) * 600. + 200.);
            } else {
                inputs.ball_is_approaching = false;
            }
        }
    }

    for (mut inputs, p_t, _p) in ai_q.iter_mut() {
        // todo: fix for leftie
        let diff =
            Vec2::new((court.right - net.current_offset) / 2., 0.) - p_t.translation.truncate();
        inputs.dir_to_center = diff.normalize();
        inputs.distance_to_center = diff.length();
    }
}

fn stand_still(
    mut action_q: Query<(&Actor, &mut ActionState), With<StandStillAction>>,
    mut move_q: Query<&mut PlayerMovement>,
) {
    for (Actor(actor), mut state) in action_q.iter_mut() {
        if let Ok(mut movement) = move_q.get_mut(*actor) {
            match *state {
                ActionState::Requested => {
                    movement.raw_dir = Vec2::ZERO;
                    *state = ActionState::Success;
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

fn score_move_to_ball(
    mut score_q: Query<(&Actor, &mut Score), With<MoveToBallScorer>>,
    inputs_q: Query<(&AiPlayerInputs, &Player, &GlobalTransform)>,
) {
    for (Actor(actor), mut score) in score_q.iter_mut() {
        if let Ok((inputs, _player, _t)) = inputs_q.get(*actor) {
            if inputs.ball_is_approaching {
                score.set(1.);
            } else {
                score.set(0.);
            }

            // if let Ok((ball, ball_t)) = ball_q.get(ball_data.entity) {
            //     if let Ok(b_bounce) = ball_bounce_q.get(ball.bounce_e.unwrap()) {
            //         // if b_bounce.count <= 1 && ball.speed >= BALL_MAX_SPEED * 0.8 {
            //         //     // ignore, if it hasn't bounced and is quite fast
            //         //     score.set(0.);
            //         // } else

            //         // if player.is_left() {
            //         //     if ball_t.translation.x <= t.translation.x {
            //         //         score.set(1.);
            //         //     } else {
            //         //         score.set(inverse_lerp(BALL_MAX_SPEED, 0., ball.speed));
            //         //     }
            //         // } else {
            //         //     if ball_t.translation.x >= t.translation.x {
            //         //         score.set(1.);
            //         //     } else {
            //         //         score.set(inverse_lerp(BALL_MAX_SPEED, 0., ball.speed));
            //         //         if score.get() > 0. {
            //         //             trace!("score: {}, speed: {}", score.get(), ball.speed);
            //         //         }
            //         //     }
            //         // }
            //     }
            // }
        }
    }
}

// todo: split into move_to_ball and move_to_predicted_bounce_pos
fn move_to_ball_action(
    mut action_q: Query<(&Actor, &mut ActionState), With<MoveToBallAction>>,
    mut q: Query<(&mut PlayerMovement, &AiPlayerInputs, &GlobalTransform)>,
    ball_q: Query<(&Ball, &GlobalTransform), (With<Ball>, Without<Player>)>,
) {
    for (Actor(actor), mut state) in action_q.iter_mut() {
        if let Ok((mut movement, inputs, t)) = q.get_mut(*actor) {
            match *state {
                ActionState::Requested | ActionState::Executing => {
                    if inputs.ball_is_approaching {
                        if let Ok((_ball, _ball_t)) = ball_q.get_single() {
                            let dist_clamp_max = 50.;
                            let dist = inputs.predicted_swing_pos - t.translation.truncate();
                            let dist_mult = inverse_lerp(0., dist_clamp_max, dist.length());

                            movement.raw_dir = dist.normalize() * dist_mult;
                        }
                    } else {
                        movement.raw_dir = Vec2::ZERO;
                    }
                    *state = ActionState::Executing;
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

// todo: better positioning prediction
fn score_move_to_center(
    mut score_q: Query<(&Actor, &mut Score), With<MoveToCenterScorer>>,
    inputs_q: Query<&AiPlayerInputs>,
) {
    for (Actor(actor), mut score) in score_q.iter_mut() {
        if let Ok(inputs) = inputs_q.get(*actor) {
            if !inputs.ball_is_approaching && inputs.distance_to_center > 250. {
                score.set(1.);
            } else {
                score.set(0.);
            }
        }
    }
}

fn move_to_center_action(
    mut action_q: Query<(&Actor, &mut ActionState), With<MoveToCenterAction>>,
    mut q: Query<(&mut PlayerMovement, &AiPlayerInputs, &GlobalTransform)>,
    _court_set: Res<CourtSettings>,
    _net: Res<NetOffset>,
) {
    for (Actor(actor), mut state) in action_q.iter_mut() {
        if let Ok((mut movement, inputs, _t)) = q.get_mut(*actor) {
            match *state {
                ActionState::Requested | ActionState::Executing => {
                    if inputs.distance_to_center > 10. {
                        movement.raw_dir = inputs.dir_to_center;
                    } else {
                        movement.raw_dir = Vec2::ZERO;
                    }

                    *state = ActionState::Executing;
                }
                ActionState::Cancelled => {
                    *state = ActionState::Failure;
                }
                _ => {}
            }
        }
    }
}

fn score_swing(
    mut score_q: Query<(&Actor, &mut Score), With<SwingScorer>>,
    parent_q: Query<&Parent>,
    player_q: Query<(&AiPlayerInputs, &Transform)>,
    ball_q: Query<&GlobalTransform, With<Ball>>,
) {
    for (Actor(actor), mut score) in score_q.iter_mut() {
        if let Ok(parent) = parent_q.get(*actor) {
            if let Ok((inputs, player_t)) = player_q.get(parent.0) {
                if inputs.ball_is_approaching {
                    if let Ok(ball_t) = ball_q.get_single() {
                        if (ball_t.translation - player_t.translation).length()
                            < AIM_RING_RADIUS * 0.75
                        {
                            score.set(1.);
                        } else {
                            score.set(0.);
                        }
                    }
                } else {
                    score.set(0.);
                }
            }
        }
    }
}

fn swing_action(
    mut action_q: Query<(&Actor, &mut ActionState), With<SwingAction>>,
    parent_q: Query<&Parent>,
    mut swing_q: Query<&mut PlayerSwing>,
) {
    for (Actor(actor), mut state) in action_q.iter_mut() {
        if let Ok(parent) = parent_q.get(*actor) {
            if let Ok(mut swing) = swing_q.get_mut(parent.0) {
                match *state {
                    ActionState::Requested | ActionState::Executing => {
                        match swing.status {
                            PlayerActionStatus::Ready => {
                                // todo: charge
                                swing.status = PlayerActionStatus::Active(0.125);
                                *state = ActionState::Success;
                            }
                            _ => {
                                *state = ActionState::Failure;
                            }
                        }
                    }
                    ActionState::Cancelled => {
                        *state = ActionState::Failure;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn score_aim_to_center(mut score_q: Query<(&Actor, &mut Score), With<AimToCenterScorer>>) {
    for (Actor(actor), mut score) in score_q.iter_mut() {
        // todo:
        score.set(1.);
    }
}

fn aim_to_center_action(
    mut action_q: Query<(&Actor, &mut ActionState), With<AimToCenterAction>>,
    parent_q: Query<&Parent>,
    player_q: Query<(&Transform, &Player)>,
    mut aim_q: Query<&mut PlayerAim>,
    net: Res<NetOffset>,
) {
    for (Actor(actor), mut state) in action_q.iter_mut() {
        if let Ok(parent) = parent_q.get(*actor) {
            if let Ok((p_t, p)) = player_q.get(parent.0) {
                if let Ok(mut p_aim) = aim_q.get_mut(p.aim_e) {
                    match *state {
                        ActionState::Requested | ActionState::Executing => {
                            p_aim.dir = (Vec2::new(net.current_offset, 0.)
                                - p_t.translation.truncate())
                            .normalize();
                            p_aim.raw_dir = p_aim.dir;
                        }
                        ActionState::Cancelled => {
                            *state = ActionState::Failure;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
