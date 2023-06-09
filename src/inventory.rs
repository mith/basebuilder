use bevy::{prelude::*, utils::HashSet};
use tracing::instrument;

use crate::item::Item;

#[derive(Component)]
pub struct Source;

#[derive(Component)]
pub struct Output;

#[derive(Component)]
pub struct Fuel;

pub const MAX_STACK_SIZE: u32 = 1000;

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stack {
    pub resource: Item,
    pub amount: u32,
}

impl Stack {
    pub fn new(resource: Item, amount: u32) -> Self {
        Self { resource, amount }
    }

    /// Add an amount to the stack, returning the amount that could not be added.
    pub fn add(&mut self, amount: u32) -> u32 {
        if self.amount + amount > MAX_STACK_SIZE {
            let overflow = self.amount + amount - MAX_STACK_SIZE;
            self.amount = MAX_STACK_SIZE;
            overflow
        } else {
            self.amount += amount;
            0
        }
    }
}

pub type Slot = Option<Stack>;

#[derive(Component, Debug)]
pub struct Inventory {
    pub slots: Vec<Slot>,
    pub allowed_products: Option<HashSet<Item>>,
}

impl Inventory {
    pub fn new(size: u32) -> Self {
        Self {
            slots: vec![None; size as usize],
            allowed_products: None,
        }
    }

    pub fn new_with_filter(size: u32, allowed_products: HashSet<Item>) -> Self {
        Self {
            slots: vec![None; size as usize],
            allowed_products: Some(allowed_products),
        }
    }

    /// Return true if the inventory has enough space for the items
    pub fn can_add(&self, items: &[(Item, u32)]) -> bool {
        if let Some(allowed_products) = &self.allowed_products {
            for (product, _) in items {
                if !allowed_products.contains(product) {
                    return false;
                }
            }
        }

        let mut slots = self.slots.clone();
        let items = items.to_vec();
        for (item_resource, mut item_amount) in items {
            let mut added = false;
            for slot in slots.iter_mut() {
                if let Some(stack) = slot {
                    if stack.resource == item_resource {
                        if stack.amount + item_amount <= MAX_STACK_SIZE {
                            stack.amount += item_amount;
                            added = true;
                            break;
                        } else {
                            let diff = MAX_STACK_SIZE - stack.amount;
                            stack.amount = MAX_STACK_SIZE;
                            item_amount -= diff;
                        }
                    }
                } else {
                    *slot = Some(Stack::new(item_resource, item_amount));
                    added = true;
                    break;
                }
            }
            if !added {
                return false;
            }
        }
        true
    }

    /// Add the items to the inventory, returning the remainder
    pub fn add_items(&mut self, items: &[(Item, u32)]) -> Vec<(Item, u32)> {
        let mut remainder = Vec::new();
        for (resource, amount) in items {
            let mut amount = *amount;

            // First iterate over existing stacks
            for stack in self.slots.iter_mut().flatten() {
                if stack.resource == *resource {
                    let space = MAX_STACK_SIZE - stack.amount;
                    if space >= amount {
                        stack.amount += amount;
                        amount = 0;
                    } else {
                        stack.amount = MAX_STACK_SIZE;
                        amount -= space;
                    }
                }
                if amount == 0 {
                    break;
                }
            }

            if amount == 0 {
                return remainder;
            }

            // Then put in the first empty slot
            if let Some(slot) = self.slots.iter_mut().find(|s| s.is_none()) {
                *slot = Some(Stack {
                    resource: resource.clone(),
                    amount: amount.min(MAX_STACK_SIZE),
                });
                amount = 0;
            }

            if amount > 0 {
                remainder.push((resource.clone(), amount));
            }
        }

        remainder
    }

    pub fn add_item(&mut self, resource: Item, amount: u32) -> Vec<(Item, u32)> {
        self.add_items(&[(resource, amount)])
    }

    pub fn has_items(&self, items: &[(Item, u32)]) -> bool {
        for (resource, amount) in items {
            let mut amount = *amount;
            for slot in self.slots.iter() {
                if amount == 0 {
                    break;
                }
                if let Some(stack) = slot {
                    if stack.resource == *resource {
                        if stack.amount >= amount {
                            amount = 0;
                        } else {
                            amount -= stack.amount;
                        }
                    }
                }
            }
            if amount > 0 {
                return false;
            }
        }
        true
    }

    /// Removes all items atomically, returning true on success
    pub fn remove_items(&mut self, items: &[(Item, u32)]) -> bool {
        if !self.has_items(items) {
            return false;
        }

        for (resource, amount) in items {
            let mut amount = *amount;
            for slot in self.slots.iter_mut() {
                if amount == 0 {
                    break;
                }
                if let Some(stack) = slot {
                    if stack.resource == *resource {
                        if stack.amount >= amount {
                            stack.amount -= amount;
                            amount = 0;
                        } else {
                            amount -= stack.amount;
                            stack.amount = 0;
                        }
                        if stack.amount == 0 {
                            *slot = None;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn add_stack(&mut self, stack: Stack) -> Option<Stack> {
        let mut stack = stack;
        for slot in self.slots.iter_mut() {
            if let Some(existing_stack) = slot {
                if existing_stack.resource == stack.resource {
                    let overflow = existing_stack.add(stack.amount);
                    if overflow == 0 {
                        return None;
                    } else {
                        stack.amount = overflow;
                    }
                }
            } else {
                *slot = Some(stack);
                return None;
            }
        }
        Some(stack)
    }

    pub fn take_stack(&mut self, max_size: u32) -> Option<Stack> {
        let mut return_stack: Option<Stack> = None;

        for slot in self.slots.iter_mut() {
            if let Some(existing_stack) = slot {
                if let Some(ref mut stack) = return_stack {
                    if stack.resource == existing_stack.resource {
                        let taking_n = max_size.min(existing_stack.amount);
                        stack.amount += taking_n;
                        existing_stack.amount -= taking_n;
                        if existing_stack.amount == 0 {
                            *slot = None;
                        }
                    }
                } else {
                    let taking_n = existing_stack.amount.min(max_size);
                    return_stack = Some(Stack::new(existing_stack.resource.clone(), taking_n));
                    existing_stack.amount -= taking_n;
                    if existing_stack.amount == 0 {
                        *slot = None;
                    }
                }
            }
        }
        return_stack
    }
}
pub fn transfer_between_slots(source_slot: &mut Slot, target_slot: &mut Slot) {
    if let Some(ref mut source_stack) = source_slot {
        if let Some(ref mut target_stack) = target_slot {
            transfer_between_stacks(source_stack, target_stack);
            if source_stack.amount == 0 {
                *source_slot = None;
            }
        } else {
            info!("Moving source stack to target slot");
            *target_slot = Some(source_stack.clone());
            *source_slot = None;
        }
    }
}

#[instrument]
pub fn drop_within_inventory(inventory: &mut Inventory, source_slot: usize, target_slot: usize) {
    if let Some(mut source_stack) = inventory.slots.get(source_slot).cloned().flatten() {
        if let Some(mut target_stack) = inventory.slots.get(target_slot).cloned().flatten() {
            transfer_between_stacks(&mut source_stack, &mut target_stack);
            inventory.slots[target_slot] = Some(target_stack);
            inventory.slots[source_slot] = {
                if source_stack.amount > 0 {
                    info!(source_stack = ?source_stack, "Keeping source stack");
                    Some(source_stack)
                } else {
                    info!("Dropping source stack");
                    None
                }
            };
        } else {
            info!("Moving source stack to target slot");
            inventory.slots[target_slot] = Some(source_stack.clone());
            inventory.slots[source_slot] = None;
        }
    } else {
        error!("Source slot is empty");
    }
}

#[instrument]
pub fn transfer_between_stacks(source_stack: &mut Stack, target_stack: &mut Stack) {
    if source_stack == target_stack {
        return;
    }
    if target_stack.resource == source_stack.resource {
        info!("Adding source stack to target stack");
        let remainder = target_stack.add(source_stack.amount);
        source_stack.amount = remainder;
    } else {
        info!("Swapping stacks");
        std::mem::swap(source_stack, target_stack);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn has_items() {
        let mut inventory = Inventory::new(12);
        inventory.add_items(&[
            (Item::new("Stone".into()), 10),
            (Item::new("Wood".into()), 20),
        ]);
        assert!(inventory.has_items(&[
            (Item::new("Stone".into()), 5),
            (Item::new("Wood".into()), 10)
        ]));
        assert!(!inventory.has_items(&[
            (Item::new("Stone".into()), 5),
            (Item::new("Wood".into()), 30)
        ]));
    }

    #[test]
    fn remove_items() {
        let mut inventory = Inventory::new(12);
        inventory.add_items(&[
            (Item::new("Stone".into()), 10),
            (Item::new("Wood".into()), 20),
        ]);
        inventory.remove_items(&[
            (Item::new("Stone".into()), 5),
            (Item::new("Wood".into()), 10),
        ]);
        assert_eq!(
            inventory.slots[0],
            Some(Stack::new(Item::new("Stone".into()), 5))
        );
        assert_eq!(
            inventory.slots[1],
            Some(Stack::new(Item::new("Wood".into()), 10))
        );
    }

    #[test]
    fn remove_items_empty() {
        let mut inventory = Inventory::new(12);
        inventory.add_items(&[
            (Item::new("Stone".into()), 10),
            (Item::new("Wood".into()), 20),
        ]);
        inventory.remove_items(&[
            (Item::new("Stone".into()), 10),
            (Item::new("Wood".into()), 20),
        ]);
        assert!(inventory.slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn remove_items_not_enough() {
        let mut inventory = Inventory::new(12);
        inventory.add_items(&[
            (Item::new("Stone".into()), 10),
            (Item::new("Wood".into()), 20),
        ]);
        assert!(!inventory.remove_items(&[
            (Item::new("Stone".into()), 5),
            (Item::new("Wood".into()), 30)
        ]));
        assert_eq!(
            inventory.slots[0],
            Some(Stack::new(Item::new("Stone".into()), 10))
        );
        assert_eq!(
            inventory.slots[1],
            Some(Stack::new(Item::new("Wood".into()), 20))
        );
    }
    #[test]
    fn add_items() {
        let mut inventory = Inventory::new(12);
        inventory.add_items(&[
            (Item::new("Stone".into()), 10),
            (Item::new("Wood".into()), 20),
        ]);
        assert_eq!(
            inventory.slots[0],
            Some(Stack::new(Item::new("Stone".into()), 10))
        );
        assert_eq!(
            inventory.slots[1],
            Some(Stack::new(Item::new("Wood".into()), 20))
        );
    }

    #[test]
    fn add_items_remainder() {
        let mut inventory = Inventory::new(1);
        let remainder = inventory.add_items(&[
            (Item::new("Stone".into()), 10),
            (Item::new("Wood".into()), 20),
        ]);
        assert_eq!(
            inventory.slots[0],
            Some(Stack::new(Item::new("Stone".into()), 10))
        );
        assert_eq!(remainder, vec![(Item::new("Wood".into()), 20)]);
    }

    #[test]
    fn add_items_stack() {
        let mut inventory = Inventory::new(2);
        inventory.slots[1] = Some(Stack::new(Item::new("Stone furnace".into()), 10));
        inventory.add_items(&[(Item::new("Stone furnace".into()), 1)]);
        assert_eq!(
            inventory.slots[1],
            Some(Stack::new(Item::new("Stone furnace".into()), 11))
        );
    }

    #[test]
    fn add_stack() {
        let mut inventory = Inventory::new(12);
        inventory.add_stack(Stack::new(Item::new("Stone".into()), 10));
        assert_eq!(
            inventory.slots[0],
            Some(Stack::new(Item::new("Stone".into()), 10))
        );
    }

    #[test]
    fn add_stack_remainder() {
        let mut inventory = Inventory::new(0);
        let remainder = inventory.add_stack(Stack::new(Item::new("Stone".into()), 10));
        assert_eq!(remainder, Some(Stack::new(Item::new("Stone".into()), 10)));
    }

    #[test]
    fn take_stack() {
        let mut inventory = Inventory::new(12);
        inventory.add_stack(Stack::new(Item::new("Stone".into()), 10));
        let taken = inventory.take_stack(100);

        assert_eq!(taken, Some(Stack::new(Item::new("Stone".into()), 10)));
        assert!(inventory.slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn take_stack_multiple_slots() {
        let mut inventory = Inventory::new(12);
        inventory.slots[0] = Some(Stack::new(Item::new("Stone".into()), 10));
        inventory.slots[1] = Some(Stack::new(Item::new("Stone".into()), 10));
        let taken = inventory.take_stack(100);

        assert_eq!(taken, Some(Stack::new(Item::new("Stone".into()), 20)));
        assert!(inventory.slots.iter().all(|s| s.is_none()));
    }

    #[test]
    fn transfer_between_stacks_swap() {
        let mut source_stack = Stack::new(Item::new("Stone".into()), 10);
        let mut target_stack = Stack::new(Item::new("Iron ore".into()), 20);

        transfer_between_stacks(&mut source_stack, &mut target_stack);

        assert_eq!(source_stack, Stack::new(Item::new("Iron ore".into()), 20));
        assert_eq!(target_stack, Stack::new(Item::new("Stone".into()), 10));
    }

    #[test]
    fn transfer_between_stacks_same() {
        let mut source_stack = Stack::new(Item::new("Stone".into()), 10);
        let mut target_stack = Stack::new(Item::new("Stone".into()), 20);

        transfer_between_stacks(&mut source_stack, &mut target_stack);

        assert_eq!(source_stack, Stack::new(Item::new("Stone".into()), 0));
        assert_eq!(target_stack, Stack::new(Item::new("Stone".into()), 30));
    }

    #[test]
    fn transfer_between_slots_swap() {
        let mut source_slot = Some(Stack::new(Item::new("Stone".into()), 10));
        let mut target_slot = Some(Stack::new(Item::new("Iron ore".into()), 20));

        transfer_between_slots(&mut source_slot, &mut target_slot);

        assert_eq!(
            source_slot,
            Some(Stack::new(Item::new("Iron ore".into()), 20))
        );
        assert_eq!(target_slot, Some(Stack::new(Item::new("Stone".into()), 10)));
    }

    #[test]
    fn transfer_between_slots_merge_stacks() {
        let mut source_slot = Some(Stack::new(Item::new("Stone".into()), 10));
        let mut target_slot = Some(Stack::new(Item::new("Stone".into()), 20));

        transfer_between_slots(&mut source_slot, &mut target_slot);

        assert_eq!(source_slot, None);
        assert_eq!(target_slot, Some(Stack::new(Item::new("Stone".into()), 30)));
    }

    #[test]
    fn transfer_between_slots_empty() {
        let mut source_slot = Some(Stack::new(Item::new("Stone".into()), 10));
        let mut target_slot = None;

        transfer_between_slots(&mut source_slot, &mut target_slot);

        assert_eq!(source_slot, None);
        assert_eq!(target_slot, Some(Stack::new(Item::new("Stone".into()), 10)));
    }

    #[test]
    fn drop_within_inventory_swap() {
        let mut inventory = Inventory::new(12);

        inventory.slots[0] = Some(Stack::new(Item::new("Stone".into()), 10));
        inventory.slots[1] = Some(Stack::new(Item::new("Iron ore".into()), 20));

        drop_within_inventory(&mut inventory, 1, 0);

        assert_eq!(
            inventory.slots[0],
            Some(Stack::new(Item::new("Iron ore".into()), 20))
        );
        assert_eq!(
            inventory.slots[1],
            Some(Stack::new(Item::new("Stone".into()), 10))
        );
    }
}
