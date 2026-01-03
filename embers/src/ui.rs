pub mod dim;
pub mod loading_screen;
pub mod main_menu;

use bevy::prelude::*;

fn ui_button(label: impl Into<String>) -> impl Bundle {
    (
        Button,
        Node {
            width: px(200),
            height: px(20),
            margin: UiRect::all(px(3)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BUTTON_BACKGROUND_DEFAULT,
        children![(Text::new(label))],
    )
}

const BUTTON_BACKGROUND_DEFAULT: BackgroundColor = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(UiScale(3.)).add_plugins((
        loading_screen::plugin,
        main_menu::plugin,
        dim::plugin,
    ));
}
