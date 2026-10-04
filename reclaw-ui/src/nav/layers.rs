//! Things drawn over the page that Back should close before it leaves the page: a dialog, a menu.
//! The owner opens a layer when it shows and closes it when it hides; `Nav::back` closes the topmost
//! open layer first. Pure data, so the order rules are unit-tested.

/// Identifies one open layer. Ids are never reused, so a stale one cannot close a newer layer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LayerId(u64);

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Layers {
    next: u64,
    /// Open layers, topmost last.
    open: Vec<LayerId>,
}

impl Layers {
    pub fn open(&mut self) -> LayerId {
        let id = LayerId(self.next);
        self.next += 1;
        self.open.push(id);
        id
    }

    /// Close one layer wherever it is in the stack. Closing one that is already closed does nothing.
    pub fn close(&mut self, id: LayerId) {
        self.open.retain(|open| *open != id);
    }

    /// Close the topmost layer. Returns whether there was one.
    pub fn close_top(&mut self) -> bool {
        self.open.pop().is_some()
    }

    pub fn contains(&self, id: LayerId) -> bool {
        self.open.contains(&id)
    }

    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_closes_the_topmost_layer_first() {
        let mut layers = Layers::default();
        let (a, b) = (layers.open(), layers.open());
        assert!(layers.close_top());
        assert!(layers.contains(a) && !layers.contains(b));
        assert!(layers.close_top());
        assert!(layers.is_empty());
        assert!(!layers.close_top(), "nothing left: Back goes to the previous page");
    }

    #[test]
    fn closing_by_id_works_in_any_order_and_twice() {
        let mut layers = Layers::default();
        let (a, b) = (layers.open(), layers.open());
        layers.close(a);
        layers.close(a);
        assert!(layers.contains(b) && !layers.contains(a));
    }

    #[test]
    fn ids_are_not_reused() {
        let mut layers = Layers::default();
        let a = layers.open();
        layers.close(a);
        let b = layers.open();
        assert_ne!(a, b);
        layers.close(a);
        assert!(layers.contains(b), "a stale id does not close a newer layer");
    }
}
