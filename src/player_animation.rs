use crate::animation::{get_fade_out_sprite_anim, get_scale_out_tween};
use crate::player::PLAYER_SIZE;
use crate::GameState;
use crate::{
    animation::TransformRotation,
    player::{SwingRangeSprite, SWING_LABEL},
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
            .add_system(unblock_animation);
    }
}

#[derive(Default, Inspectable, PartialEq, Debug)]
pub enum PlayerAnimation {
    #[default]
    Idle,
    Walking,
    Running,
    Celebrating,
    Loss,
    Swinging,
}

#[derive(Component, Inspectable)]
pub struct PlayerAnimationData {
    pub animation: PlayerAnimation,
    pub face_e: Entity,
    pub jump_e: Entity,
    pub body_e: Entity,
    pub body_root_e: Entity,
}

#[derive(Component, Inspectable)]
pub struct AgentAnimationBlock(pub f32);

fn animate(
    mut commands: Commands,
    player_anim_q: Query<(
        Entity,
        &PlayerAnimationData,
        Option<&AgentAnimationBlock>,
        ChangeTrackers<PlayerAnimationData>,
    )>,
    sprite_q: Query<&Sprite>,
    mut animator_q: Query<(&mut Animator<Transform>, &Transform)>,
) {
    for (anim_e, anim, block, anim_tracker) in player_anim_q.iter() {
        if anim_tracker.is_changed() || anim_tracker.is_added() {
            if block.is_some() {
                continue;
            }

            let mut stop_anim_entities: Vec<Entity> = Vec::new();
            let mut body_root_tween = None;
            let mut face_anim = None;

            debug!("anim change to {:?}", anim.animation);
            match anim.animation {
                PlayerAnimation::Swinging => {
                    stop_anim_entities.push(anim.face_e);
                    stop_anim_entities.push(anim.body_e);
                    stop_anim_entities.push(anim.body_root_e);
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
                    body_root_tween = Some(get_move_tween(500, 40., 12.));
                }
                PlayerAnimation::Loss => {
                    if let Ok((_, t)) = animator_q.get_mut(anim.body_e) {
                        stop_anim_entities.push(anim.face_e);
                        stop_anim_entities.push(anim.body_e);
                        body_root_tween = Some(get_loss_tween(t));
                        if let Ok(sprite) = sprite_q.get(anim.face_e) {
                            face_anim = Some(get_fade_out_sprite_anim(sprite.color, 1000, None));
                        }
                    }
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

            if let Some(face_anim) = face_anim {
                commands.entity(anim.face_e).insert(face_anim);
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

fn get_loss_tween(transform: &Transform) -> Tracks<Transform> {
    let dur = 3000;
    let scale_end = Vec3::new(2.5, 0.1, transform.scale.z);
    let scale_tween = Tween::new(
        EaseFunction::QuadraticOut,
        TweeningType::Once,
        Duration::from_millis(dur),
        TransformScaleLens {
            start: transform.scale,
            end: scale_end,
        },
    );
    let offset_tween = Tween::new(
        EaseFunction::QuadraticOut,
        TweeningType::Once,
        Duration::from_millis(dur),
        TransformPositionLens {
            start: transform.translation,
            end: transform.translation - Vec3::Y * PLAYER_SIZE / 2.,
        },
    );

    Tracks::new([
        scale_tween.then(get_scale_out_tween(scale_end, 1500, None)),
        Sequence::new([offset_tween]),
    ])
}
