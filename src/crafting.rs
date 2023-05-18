use std::collections::VecDeque;

use bevy::{prelude::*, reflect::TypeUuid};
use serde::Deserialize;

use crate::item::Item;

#[derive(Clone, Debug, Deserialize, TypeUuid)]
#[uuid = "1ca725c1-5a0d-484f-8d04-a5a42960e208"]
pub(crate) struct Recipe {
    pub(crate) materials: Vec<(Item, u32)>,
    pub(crate) products: Vec<(Item, u32)>,
    pub(crate) crafting_time: f32,
    pub(crate) name: String,
}

pub struct ActiveCraft {
    pub(crate) blueprint: Recipe,
    pub(crate) timer: Timer,
}

#[derive(Component, Default)]
pub(crate) struct CraftingQueue(pub(crate) VecDeque<ActiveCraft>);
