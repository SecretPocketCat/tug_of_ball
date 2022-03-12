use crate::player::{get_swing_multiplier, Player, PlayerSwing};
use crate::GameState;
use crate::{
    animation::TransformRotation,
    player::{PlayerDash, SwingRangeSprite, SWING_LABEL},
    player_action::PlayerActionStatus,
};
use bevy::{math::Vec2, prelude::*};
use bevy_inspector_egui::Inspectable;
use bevy_time::{ScaledTime, ScaledTimeDelta};
use bevy_tweening::lens::{TransformPositionLens, TransformRotationLens, TransformScaleLens};
use bevy_tweening::*;
use interpolation::EaseFunction;
use std::time::Duration;

pub struct PlayerAnimationPlugin;
impl Plugin for PlayerAnimationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(animate.after(SWING_LABEL))
            .add_system(unblock_animation)
            .add_system_set(
                SystemSet::on_update(GameState::Game)
                    .with_system(animate_dash_state_ui)
                    .with_system(animate_swing_charge_ui),
            );
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
pub struct AgentAnimationData {
    pub animation: PlayerAnimation,
    pub face_e: Entity,
    pub body_e: Entity,
    pub body_root_e: Entity,
}

#[derive(Component, Inspectable)]
pub struct AgentAnimationBlock(pub f32);

fn animate(
    mut commands: Commands,
    player_anim_q: Query<(
        Entity,
        &AgentAnimationData,
        Option<&AgentAnimationBlock>,
        ChangeTrackers<AgentAnimationData>,
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

                        commands.entity(anim_e).insert(AgentAnimationBlock(dur));
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

                        commands.entity(anim_e).insert(AgentAnimationBlock(dur));
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
    mut block_q: Query<(Entity, &mut AgentAnimationBlock)>,
    time: ScaledTime,
) {
    for (e, mut block) in block_q.iter_mut() {
        block.0 -= time.scaled_delta_seconds();

        if block.0 < 0. {
            commands.entity(e).remove::<AgentAnimationBlock>();
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
            end,
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

fn animate_dash_state_ui(
    mut q: Query<(&Parent, &mut TransformRotation), With<SwingRangeSprite>>,
    dash_q: Query<&PlayerDash>,
    time: ScaledTime,
) {
    for (parent, mut rot) in q.iter_mut() {
        if let Ok(dash) = dash_q.get(parent.0) {
            match dash.status {
                PlayerActionStatus::Ready | PlayerActionStatus::Charging(..) => {
                    rot.rotation_rad += time.scaled_delta_seconds() * rot.rotation_max_rad * 3.;
                    if rot.rotation_rad.abs() > rot.rotation_max_rad.abs() {
                        rot.rotation_rad = rot.rotation_max_rad;
                    }
                }
                PlayerActionStatus::Active(..) => {
                    let mult = 1. - dash.timer.percent();
                    rot.rotation_rad = rot.rotation_max_rad * mult * rot.rotation_max_rad.signum();
                }
                PlayerActionStatus::Cooldown => rot.rotation_rad = 0.,
            };
        }
    }
}

fn animate_swing_charge_ui(
    player_q: Query<(&Player, &PlayerSwing)>,
    mut aim_charge_q: Query<&mut Transform>,
    time: ScaledTime,
) {
    for (player, player_swing) in player_q.iter() {
        if let Ok(mut t) = aim_charge_q.get_mut(player.aim_charge_e) {
            if let PlayerActionStatus::Charging(dur) = player_swing.status {
                let scale = get_swing_multiplier(dur);
                t.scale = Vec2::splat(scale).extend(1.);
            } else if !matches!(player_swing.status, PlayerActionStatus::Active(_)) {
                t.scale =
                    Vec2::splat((t.scale.x - (time.scaled_delta_seconds() * 3.)).clamp(0., 1.))
                        .extend(1.);
            }
        }
    }
}
