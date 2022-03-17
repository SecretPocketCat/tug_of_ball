use crate::{palette::PaletteColor, reset::Persistent, GameState};
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

pub struct ScorePlugin;
impl Plugin for ScorePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<Score>()
            .add_event::<ScoreChangedEvt>()
            .add_startup_system(setup)
            .add_system_set(SystemSet::on_enter(GameState::Game).with_system(reset_score))
            .add_system(update_score_ui);
    }
}

#[derive(Component, Inspectable)]
struct PointsText;

#[derive(Default)]
pub struct Score {
    pub left_player: PlayerScore,
    pub right_player: PlayerScore,
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
        points_text_q.single_mut().sections[0].value = format!(
            "{} | {}",
            score.left_player.points, score.right_player.points
        );
    }
}

pub fn add_point_to_score(
    score: &mut Score,
    score_ev_w: &mut EventWriter<ScoreChangedEvt>,
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
        return true;
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

    // // todo: endgame scoring - either too high or difference high enough
    // if scoring.games >= 6 {
    // }

    false
}

fn reset_score(mut score: ResMut<Score>) {
    score.left_player = PlayerScore::default();
    score.right_player = PlayerScore::default();
}
