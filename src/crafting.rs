use std::collections::VecDeque;

use bevy::{prelude::*, reflect::TypeUuid};
use serde::Deserialize;

use crate::item::Item;

#[derive(Clone, Debug, Deserialize, TypeUuid)]
#[uuid = "1ca725c1-5a0d-484f-8d04-a5a42960e208"]
pub struct Recipe {
    pub materials: Vec<(Item, u32)>,
    pub products: Vec<(Item, u32)>,
    pub crafting_time: f32,
    pub name: String,
}

pub struct ActiveCraft {
    pub blueprint: Recipe,
    pub timer: Timer,
}

#[derive(Component, Default)]
pub struct CraftingQueue(pub VecDeque<ActiveCraft>);
