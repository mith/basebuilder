use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::EguiContexts;

use crate::labor::build_structure::BuildToolState;
use crate::labor::chop_tree::FellingToolState;
use crate::labor::dig_tile::DigToolState;

pub struct ToolbarPlugin;

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(toolbar);
    }
}

enum Tool {
    Dig,
    Build,
    Chop,
}

#[derive(SystemParam)]
struct ToolStates<'w> {
    dig_tool_next_state: ResMut<'w, NextState<DigToolState>>,
    build_tool_next_state: ResMut<'w, NextState<BuildToolState>>,
    chop_tool_next_state: ResMut<'w, NextState<FellingToolState>>,
}

fn toolbar(mut contexts: EguiContexts, mut tool_states: ToolStates) {
    egui::Window::new("Toolbar").show(contexts.ctx_mut(), |ui| {
        if ui.button("Dig").clicked() {
            switch_to_tool(&mut tool_states, Tool::Dig)
        }
        if ui.button("Build").clicked() {
            switch_to_tool(&mut tool_states, Tool::Build)
        }
        if ui.button("Chop tree").clicked() {
            switch_to_tool(&mut tool_states, Tool::Chop)
        }
    });
}

fn clear_active_tool(tool_states: &mut ToolStates) {
    tool_states.dig_tool_next_state.set(DigToolState::Inactive);
    tool_states
        .build_tool_next_state
        .set(BuildToolState::Inactive);
    tool_states
        .chop_tool_next_state
        .set(FellingToolState::Inactive);
}

fn switch_to_tool(tool_states: &mut ToolStates, tool: Tool) {
    clear_active_tool(tool_states);

    match tool {
        Tool::Dig => tool_states
            .dig_tool_next_state
            .set(DigToolState::Designating),
        Tool::Build => tool_states
            .build_tool_next_state
            .set(BuildToolState::Placing),
        Tool::Chop => tool_states
            .chop_tool_next_state
            .set(FellingToolState::Designating),
    }
}
