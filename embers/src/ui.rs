pub mod loading_screen;
pub mod main_menu;
pub mod dim;

use bevy::ecs::bundle::NoBundleEffect;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use std::sync::RwLock;

fn scalable<C: Component>(scalable: fn(UIScale) -> C) -> impl Bundle<Effect: NoBundleEffect> {
    (
        scalable(*UI_SCALE.read().unwrap()),
        ScalableComponent::new(scalable),
    )
}

fn dynamic_scalable<C: Component, F: Fn(UIScale) -> C + Send + Sync + 'static>(
    dynamic_scalable: F,
) -> impl Bundle<Effect: NoBundleEffect> {
    (
        dynamic_scalable(*UI_SCALE.read().unwrap()),
        ScalableComponent::dynamic(dynamic_scalable),
    )
}

fn ui_button(label: impl Into<String>) -> impl Bundle {
    (
        Button,
        scalable(|scale| Node {
            width: px(scale * 200),
            height: px(scale * 20),
            margin: UiRect::all(px(scale * 3)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        }),
        BUTTON_BACKGROUND_DEFAULT,
        children![(Text::new(label),)],
    )
}

#[derive(Message)]
struct RescaleUI;

type UIScale = i32;

#[derive(Component)]
enum ScalableComponent<C: Component> {
    Static(fn(UIScale) -> C),
    Dynamic(Box<dyn Fn(UIScale) -> C + Send + Sync>),
}

impl<C: Component> ScalableComponent<C> {
    fn new(scalable: fn(UIScale) -> C) -> Self {
        Self::Static(scalable)
    }
    fn dynamic<F: Fn(UIScale) -> C + Send + Sync + 'static>(dynamic_scalable: F) -> Self {
        Self::Dynamic(Box::new(dynamic_scalable))
    }
    fn apply(&self, component: &mut C) {
        *component = match self {
            ScalableComponent::Static(scalable) => scalable(*UI_SCALE.read().unwrap()),
            ScalableComponent::Dynamic(dynamic_scalable) => {
                dynamic_scalable(*UI_SCALE.read().unwrap())
            }
        }
    }
}

static UI_SCALE: RwLock<UIScale> = RwLock::new(3);

fn rescale_components<C: Component<Mutability = Mutable>>(
    mut scalables: Query<(&ScalableComponent<C>, &mut C)>,
    mut rescale_messages: MessageReader<RescaleUI>,
) {
    for _ in rescale_messages.read() {
        for (scalable, mut component) in scalables.iter_mut() {
            scalable.apply(&mut component);
        }
    }
}

const BUTTON_BACKGROUND_DEFAULT: BackgroundColor = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));

pub(super) fn plugin(app: &mut App) {
    app.add_message::<RescaleUI>()
        .add_systems(
            Update,
            (rescale_components::<Node>, rescale_components::<Outline>),
        )
        .add_plugins((loading_screen::plugin, main_menu::plugin, dim::plugin));
}
