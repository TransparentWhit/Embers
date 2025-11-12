pub mod main_menu;
pub mod world;

use bevy::prelude::*;
use std::sync::LazyLock;

static BUTTON_NODE: LazyLock<Node> = LazyLock::new(|| Node {
    width: px(400),
    height: px(50),
    margin: UiRect::all(px(20)),
    justify_content: JustifyContent::Center,
    align_items: AlignItems::Center,
    ..default()
});

const BUTTON_BACKGROUND_DEFAULT: BackgroundColor = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));

#[macro_export]
macro_rules! ui_button {
    ($label: expr $(,$extra:expr)* $(,)?) => {
        (
            Button,
            BUTTON_NODE.clone(),
            BUTTON_BACKGROUND_DEFAULT,
            $($extra,)*
            children![
                (
                    Text::new($label),
                )
            ]
        )
    };
}
