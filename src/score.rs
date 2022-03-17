use std::time::Duration;

use crate::{
    animation::{get_scale_out_anim, TweenDoneAction},
    level::{Net, NetOffset},
    palette::PaletteColor,
    player::{Inactive, Player, PlayerGui},
    player_animation::{PlayerAnimation, PlayerAnimationData},
    reset::Persistent,
    GameState,
};
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

pub const GAME_SCORE_TARGET: u8 = 5;
pub const NET_OFFSET_POINT: f32 = 30.;
pub const NET_OFFSET_GAME: f32 = 90.;

pub struct ScorePlugin;
impl Plugin for ScorePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<Score>()
            .add_event::<ScoreChangedEvt>()
            .add_event::<GameOverEvt>()
            .add_startup_system(setup)
            .add_system_set(SystemSet::on_enter(GameState::Game).with_system(reset_score))
            .add_system_to_stage(CoreStage::Last, on_game_over)
            .add_system(update_score_ui);
    }
}

#[derive(Component, Inspectable)]
struct PointsText;

#[derive(Default)]
pub struct Score {
    pub left_player: PlayerScore,
    pub right_player: PlayerScore,
    pub left_has_won: Option<bool>,
}

#[derive(Default, Component, Inspectable)]
pub struct PlayerScore {
    pub points: u8,
    pub games: u8,
}

pub enum ScoreChangeType {
    Point,
    Game,
}

pub struct ScoreChangedEvt {
    pub left_side_scored: bool,
    pub score_type: ScoreChangeType,
}

pub struct GameOverEvt {
    pub left_has_won: bool,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::Center,
                position_type: PositionType::Relative,
                margin: Rect {
                    top: Val::Auto,
                    bottom: Val::Px(10.0),
                    right: Val::Auto,
                    left: Val::Auto,
                },
                ..Default::default()
            },
            text: Text::with_section(
                "",
                TextStyle {
                    font: asset_server.load("fonts/Typo_Round_Regular_Demo.otf"),
                    font_size: 100.0,
                    color: Color::WHITE,
                },
                // Note: You can use `Default::default()` in place of the `TextAlignment`
                TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    ..Default::default()
                },
            ),
            ..Default::default()
        })
        .insert(PaletteColor::Text)
        .insert(PointsText)
        .insert(Name::new("ScoreText"))
        .insert(Persistent);
}

fn update_score_ui(score: Res<Score>, mut points_text_q: Query<&mut Text, With<PointsText>>) {
    if score.is_changed() {
        let txt = if let Some(left_has_won) = score.left_has_won {
            format!("{} HAS WON", if left_has_won { "LEFT" } else { "RIGHT" })
        } else {
            format!(
                "{} | {}",
                points_to_str(score.left_player.points),
                points_to_str(score.right_player.points)
            )
        };
        points_text_q.single_mut().sections[0].value = txt;
    }
}

fn points_to_str(points: u8) -> String {
    match points {
        0 => "00".into(),
        1 => "15".into(),
        2 => "30".into(),
        3 => "40".into(),
        4 => "AD".into(),
        _ => points.to_string(),
    }
}

fn on_game_over(
    mut game_over_ev_r: EventReader<GameOverEvt>,
    mut score: ResMut<Score>,
    mut commands: Commands,
    mut player_q: Query<(Entity, &Player, &mut PlayerAnimationData)>,
    player_gui_q: Query<(Entity, &Transform), With<PlayerGui>>,
) {
    for ev in game_over_ev_r.iter() {
        for (player_e, player, mut player_anim) in player_q.iter_mut() {
            let mut e_cmds = commands.entity(player_e);

            e_cmds.insert(Inactive);

            if player.is_left() == ev.left_has_won {
                player_anim.animation = PlayerAnimation::Celebrating;
            } else {
                player_anim.animation = PlayerAnimation::Loss;
            }
        }

        for (gui_e, gui_t) in player_gui_q.iter() {
            commands.entity(gui_e).insert(get_scale_out_anim(
                gui_t.scale,
                350,
                Some(TweenDoneAction::DespawnRecursive),
            ));
        }

        score.left_has_won = Some(ev.left_has_won);

        break;
    }
}

pub fn add_point_to_score(
    score: &mut Score,
    score_ev_w: &mut EventWriter<ScoreChangedEvt>,
    game_over_ev_w: &mut EventWriter<GameOverEvt>,
    add_to_left_player: bool,
) -> bool {
    let (mut scoring, mut other) = if add_to_left_player {
        (&mut score.left_player, &mut score.right_player)
    } else {
        (&mut score.right_player, &mut score.left_player)
    };

    scoring.points += 1;

    let mut required_points = (other.points + 2).max(4);
    if cfg!(feature = "debug") {
        required_points = 100;
    }

    if scoring.points >= required_points {
        scoring.games += 1;
        scoring.points = 0;
        other.points = 0;

        score_ev_w.send(ScoreChangedEvt {
            left_side_scored: add_to_left_player,
            score_type: ScoreChangeType::Game,
        });

        if scoring.games >= GAME_SCORE_TARGET {
            game_over_ev_w.send(GameOverEvt {
                left_has_won: add_to_left_player,
            });
            return false;
        } else {
            return true;
        }
    } else if scoring.points == other.points && scoring.points > 3 {
        score_ev_w.send(ScoreChangedEvt {
            left_side_scored: add_to_left_player,
            score_type: ScoreChangeType::Point,
        });

        // hacky way to get ADV in the UI
        // nice2have: redo
        scoring.points = 3;
        other.points = 3;
    } else {
        score_ev_w.send(ScoreChangedEvt {
            left_side_scored: add_to_left_player,
            score_type: ScoreChangeType::Point,
        });
    }

    false
}

fn reset_score(mut commands: Commands, mut score: ResMut<Score>, mut net: ResMut<NetOffset>) {
    score.left_player = PlayerScore::default();
    score.right_player = PlayerScore::default();
    score.left_has_won = None;
    net.reset_queued = true;
}
