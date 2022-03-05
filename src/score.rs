use bevy::prelude::*;

use bevy_inspector_egui::Inspectable;

use crate::palette::PaletteColor;

#[derive(Component, Inspectable)]
struct PointsText;

#[derive(Default)]
pub struct PlayerScore {
    pub points: u8,
    pub games: u8,
}

#[derive(Default)]
pub struct Score {
    pub left_player: PlayerScore,
    pub right_player: PlayerScore,
}

pub struct ScorePlugin;
impl Plugin for ScorePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<Score>()
            .add_startup_system(setup)
            .add_system(update_score_ui);
    }
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
        .insert(Name::new("ScoreText"));
}

fn update_score_ui(score: Res<Score>, mut points_text_q: Query<&mut Text, With<PointsText>>) {
    if score.is_changed() {
        points_text_q.single_mut().sections[0].value = format!(
            "{} | {}",
            score.left_player.points, score.right_player.points
        );
    }
}

pub fn add_point_to_score(score: &mut Score, add_to_left_player: bool) -> bool {
    let (mut scoring, mut other) = if add_to_left_player {
        (&mut score.left_player, &mut score.right_player)
    } else {
        (&mut score.right_player, &mut score.left_player)
    };

    scoring.points += 1;

    let required_points = (other.points + 2).max(4);
    #[cfg(feature = "debug")]
    {
        required_points = 100;
    }

    if scoring.points >= required_points {
        scoring.games += 1;
        scoring.points = 0;
        other.points = 0;
        return true;
    } else if scoring.points == other.points && scoring.points > 3 {
        // hacky way to get ADV in the UI
        // nice2have: redo
        scoring.points = 3;
        other.points = 3;
    }

    // // todo: endgame scoring - either too high or difference high enough
    // if scoring.games >= 6 {
    // }

    false
}
