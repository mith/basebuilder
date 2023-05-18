use bevy::prelude::{App, Plugin};

use self::character_ui::CharacterUiPlugin;

mod character_ui;
mod drag_and_drop;
mod inventory_grid;

pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CharacterUiPlugin)
            .add_plugin(drag_and_drop::DragAndDropPlugin);
    }
}
