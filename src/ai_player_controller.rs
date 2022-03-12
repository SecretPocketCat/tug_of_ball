use crate::{
    animation::inverse_lerp,
    ball::{Ball, BallBounce, BallHitEvt, BALL_MAX_SPEED},
    input_binding::{InputAction, InputAxis, PlayerInput},
    level::{InitialRegion, NetOffset},
    player::{
        get_swing_multiplier_clamped, spawn_player, Player, PlayerAim, PlayerDash, PlayerMovement,
        PlayerSwing, SWING_LABEL,
    },
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
            .add_system_to_stage(BigBrainStage::Scorers, score_swing)
            .add_system_to_stage(BigBrainStage::Actions, swing_action);
    }
}

#[derive(Debug, Clone, Component)]
pub struct AiPlayer;

#[derive(Debug, Clone, Inspectable)]
pub struct BallData {
    entity: Entity,
    distance: f32,
}

#[derive(Component, Default, Inspectable)]
pub struct AiPlayerInputs {
    closest_incoming_ball: Option<BallData>,
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
pub struct MoveToCenterLineAction;

#[derive(Debug, Clone, Component)]
pub struct MoveToCenterLineScorer;

#[derive(Debug, Clone, Component)]
pub struct MoveToOuterLineAction;

#[derive(Debug, Clone, Component)]
pub struct MoveToOuterLineScorer;

#[derive(Debug, Clone, Component)]
pub struct SwingScorer;

#[derive(Debug, Clone, Component)]
pub struct SwingAction;

// what thinkers are needed?
// movement thinker
// aim thinker
// swing thinker
// dodge thinker

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, region: Res<InitialRegion>) {
    let move_thinker = Thinker::build()
        .picker(FirstToScore::new(0.2))
        .when(MoveToBallScorer, MoveToBallAction)
        // .when(MoveDiagonallyToPlayerScorer, MoveDiagonallyToPlayerAction)
        // .when(MoveToOuterLineScorer, MoveToOuterLineAction)
        // .when(MoveToCenterLineScorer, MoveToCenterLineAction)
        // .otherwise(MoveToBallAction);
        .otherwise(StandStillAction);

    let swing_thinker = Thinker::build()
        .picker(FirstToScore::new(0.2))
        .when(SwingScorer, SwingAction);

    spawn_player(2, &mut commands, &asset_server, &region)
        .insert(AiPlayerInputs::default())
        .insert(AiPlayer)
        .insert(move_thinker)
        .with_children(|b| {
            b.spawn().insert(swing_thinker);
        });
}

fn on_ball_hit(
    mut ball_hit_er: EventReader<BallHitEvt>,
    ball_q: Query<&Ball>,
    ai_q: Query<&Player, With<AiPlayer>>,
) {
    for ev in ball_hit_er.iter() {
        if let Ok(ball) = ball_q.get(ev.ball_e) {
            // ball.dir

            for p in ai_q.iter() {
                if p.id != ev.player_id {
                    // todo: calc an intersection
                    // pick a point on the trajectory of the ball and calc how long it would take the player to get there
                    // pick one of the closest points taking the ball travel time into consideration
                }
            }
        }
    }
}

fn collect_inputs(
    mut ai_q: Query<(&mut AiPlayerInputs, &GlobalTransform, &Player), With<AiPlayer>>,
    ball_q: Query<(Entity, &Ball, &GlobalTransform), Without<AiPlayer>>,
) {
    for (mut inputs, ai_t, player) in ai_q.iter_mut() {
        if let Some((e, ball, ball_t)) = ball_q
            .iter()
            .filter(|(_, b, _)| {
                (player.is_left() && b.dir.x < 0.) || (!player.is_left() && b.dir.x > 0.)
            })
            .max_by(|(_, _, t1), (_, _, t2)| {
                if player.is_left() {
                    t1.translation.x.partial_cmp(&t2.translation.x).unwrap()
                } else {
                    t2.translation.x.partial_cmp(&t1.translation.x).unwrap()
                }
            })
        {
            inputs.closest_incoming_ball = Some(BallData {
                entity: e,
                distance: (ball_t.translation - ai_t.translation).length(),
            });
        } else {
            inputs.closest_incoming_ball = None;
        }
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
    ball_q: Query<(&Ball, &GlobalTransform), Without<Player>>,
    ball_bounce_q: Query<&BallBounce>,
    net: Res<NetOffset>,
) {
    for (Actor(actor), mut score) in score_q.iter_mut() {
        if let Ok((inputs, player, t)) = inputs_q.get(*actor) {
            match &inputs.closest_incoming_ball {
                Some(ball_data) => {
                    if let Ok((ball, ball_t)) = ball_q.get(ball_data.entity) {
                        if let Ok(b_bounce) = ball_bounce_q.get(ball.bounce_e.unwrap()) {
                            // if b_bounce.count <= 1 && ball.speed >= BALL_MAX_SPEED * 0.8 {
                            //     // ignore, if it hasn't bounced and is quite fast
                            //     score.set(0.);
                            // } else
                            if player.is_left() {
                                if ball_t.translation.x <= t.translation.x {
                                    score.set(1.);
                                } else {
                                    score.set(inverse_lerp(ball.max_speed, 0., ball.speed));
                                }
                            } else {
                                info!("speed: {}", ball.speed);

                                if ball_t.translation.x >= t.translation.x {
                                    score.set(1.);
                                } else {
                                    score.set(inverse_lerp(ball.max_speed, 0., ball.speed));
                                    if score.get() > 0. {
                                        info!("score: {}, speed: {}", score.get(), ball.speed);
                                    }
                                }
                            }
                        }
                    }
                }
                None => score.set(0.),
            }
        }
    }
}

fn move_to_ball_action(
    mut action_q: Query<(&Actor, &mut ActionState), With<MoveToBallAction>>,
    mut q: Query<(&mut PlayerMovement, &AiPlayerInputs, &GlobalTransform)>,
    ball_q: Query<&GlobalTransform, (With<Ball>, Without<Player>)>,
) {
    for (Actor(actor), mut state) in action_q.iter_mut() {
        if let Ok((mut movement, inputs, t)) = q.get_mut(*actor) {
            match *state {
                ActionState::Requested | ActionState::Executing => {
                    match &inputs.closest_incoming_ball {
                        Some(ball_data) => {
                            if let Ok(ball_t) = ball_q.get(ball_data.entity) {
                                let dist_clamp_max = 50.;
                                let dist_mult = inverse_lerp(
                                    0.,
                                    dist_clamp_max,
                                    ball_data.distance.min(dist_clamp_max),
                                );
                                movement.raw_dir =
                                    (ball_t.translation - t.translation).truncate().normalize()
                                        * dist_mult;
                            }
                        }
                        None => movement.raw_dir = Vec2::ZERO,
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
    inputs_q: Query<&AiPlayerInputs>,
) {
    for (Actor(actor), mut score) in score_q.iter_mut() {
        if let Ok(parent) = parent_q.get(*actor) {
            if let Ok(inputs) = inputs_q.get(parent.0) {
                match &inputs.closest_incoming_ball {
                    Some(ball_data) => {
                        // todo: get treshold value from swing or somewhere
                        if ball_data.distance < 100. {
                            score.set(1.);
                        } else {
                            score.set(0.);
                        }
                    }
                    None => score.set(0.),
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
                                swing.status = PlayerActionStatus::Active(0.3);
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
