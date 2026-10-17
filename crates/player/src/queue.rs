//! Play queue: the item list, the play order (shuffled or not), the cursor and the repeat mode.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

/// How the cursor moves: a track ending by itself, or the user pressing next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    Auto,
    Manual,
}

#[derive(Clone, Debug)]
pub struct Queue<T> {
    items: Vec<T>,
    /// Indices into `items` in play order.
    order: Vec<usize>,
    /// Cursor into `order`.
    cursor: Option<usize>,
    shuffle: bool,
    repeat: Repeat,
    seed: u64,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            order: Vec::new(),
            cursor: None,
            shuffle: false,
            repeat: Repeat::Off,
            seed: 0x9E37_79B9_7F4A_7C15,
        }
    }
}

impl<T> Queue<T> {
    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn repeat(&self) -> Repeat {
        self.repeat
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn set_repeat(&mut self, repeat: Repeat) {
        self.repeat = repeat;
    }

    /// Index into `items` of the current track.
    pub fn current(&self) -> Option<usize> {
        self.cursor.map(|c| self.order[c])
    }

    /// Replaces the queue and puts the cursor on `start`.
    pub fn set(&mut self, items: Vec<T>, start: usize) {
        self.items = items;
        self.rebuild_order((start < self.items.len()).then_some(start));
    }

    /// Moves the cursor to `index`, as when the user picks a track.
    pub fn jump(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }
        self.rebuild_order(Some(index));
        true
    }

    /// Turning shuffle on keeps the current track first; turning it off keeps it current.
    pub fn set_shuffle(&mut self, on: bool) {
        if self.shuffle != on {
            self.shuffle = on;
            self.rebuild_order(self.current());
        }
    }

    /// Appends to the end of the queue. The new tracks play last in the current order.
    pub fn push(&mut self, item: T) {
        self.items.push(item);
        self.order.push(self.items.len() - 1);
    }

    /// Inserts right after the current track, so it plays next. Without a current track it is appended.
    pub fn push_next(&mut self, item: T) {
        let Some(cursor) = self.cursor else {
            return self.push(item);
        };
        if self.shuffle {
            self.items.push(item);
            self.order.insert(cursor + 1, self.items.len() - 1);
        } else {
            // Unshuffled, the order is the item order, so the item goes into the list itself.
            self.items.insert(cursor + 1, item);
            self.order = (0..self.items.len()).collect();
        }
    }

    /// Removes an item. The cursor follows the current track; removing it moves on to the next one.
    pub fn remove(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.items.remove(index);
        let at = self.order.iter().position(|&i| i == index).unwrap();
        self.order.remove(at);
        for i in &mut self.order {
            if *i > index {
                *i -= 1;
            }
        }
        self.cursor = match self.cursor {
            _ if self.order.is_empty() => None,
            Some(c) if at < c => Some(c - 1),
            Some(c) => Some(c.min(self.order.len() - 1)),
            None => None,
        };
    }

    /// Swaps two items. The cursor and the play order keep following the same tracks.
    pub fn swap(&mut self, a: usize, b: usize) {
        if a >= self.items.len() || b >= self.items.len() || a == b {
            return;
        }
        self.items.swap(a, b);
        if self.shuffle {
            for i in &mut self.order {
                if *i == a {
                    *i = b;
                } else if *i == b {
                    *i = a;
                }
            }
        } else if let Some(c) = self.cursor {
            // Unshuffled, the order is the item order, so the cursor moves with the track.
            self.cursor = Some(if c == a {
                b
            } else if c == b {
                a
            } else {
                c
            });
        }
    }

    pub fn clear(&mut self) {
        *self = Self {
            shuffle: self.shuffle,
            repeat: self.repeat,
            seed: self.seed,
            ..Self::default()
        };
    }

    /// The next track to play, or `None` when the queue is over (the cursor stays on the last track).
    pub fn advance(&mut self, how: Advance) -> Option<usize> {
        let cursor = self.cursor?;
        if how == Advance::Auto && self.repeat == Repeat::One {
            return self.current();
        }
        if cursor + 1 < self.order.len() {
            self.cursor = Some(cursor + 1);
        } else if self.repeat == Repeat::All {
            if self.shuffle {
                self.reshuffle_keeping_last_away_from_first();
            }
            self.cursor = Some(0);
        } else {
            return None;
        }
        self.current()
    }

    /// The previous track. At the start of the queue it stays there (or wraps with repeat all).
    pub fn previous(&mut self) -> Option<usize> {
        let cursor = self.cursor?;
        self.cursor = Some(match cursor {
            0 if self.repeat == Repeat::All => self.order.len() - 1,
            0 => 0,
            c => c - 1,
        });
        self.current()
    }

    /// Rebuilds the play order with the item `current` (an index into `items`) at the cursor.
    fn rebuild_order(&mut self, current: Option<usize>) {
        self.order = (0..self.items.len()).collect();
        match current {
            Some(current) if self.shuffle => {
                self.order.swap_remove(current);
                self.shuffle_order();
                self.order.insert(0, current);
                self.cursor = Some(0);
            }
            Some(current) => self.cursor = Some(current),
            None => {
                if self.shuffle {
                    self.shuffle_order();
                }
                self.cursor = None;
            }
        }
    }

    fn shuffle_order(&mut self) {
        for i in (1..self.order.len()).rev() {
            let j = (self.next_random() % (i as u64 + 1)) as usize;
            self.order.swap(i, j);
        }
    }

    /// A new lap of shuffle: a new order, and the track that just ended does not open the next lap.
    fn reshuffle_keeping_last_away_from_first(&mut self) {
        let last = *self.order.last().unwrap();
        self.shuffle_order();
        if self.order.len() > 1 && self.order[0] == last {
            self.order.swap(0, 1);
        }
    }

    fn next_random(&mut self) -> u64 {
        // xorshift64*: plenty for shuffling, and no dependency.
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        self.seed.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Seeds the shuffle from the clock so every run differs. Tests keep the fixed seed.
    pub fn reseed(&mut self, seed: u64) {
        self.seed = seed | 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(n: usize, start: usize) -> Queue<usize> {
        let mut q = Queue::default();
        q.set((0..n).collect(), start);
        q
    }

    fn play_out(q: &mut Queue<usize>, how: Advance) -> Vec<usize> {
        let mut seen = vec![q.current().unwrap()];
        while let Some(i) = q.advance(how) {
            seen.push(i);
            assert!(seen.len() <= 100, "never ends");
        }
        seen
    }

    #[test]
    fn push_next_plays_right_after_the_current_track() {
        let mut q = queue(4, 1);
        q.push_next(9);
        let played: Vec<usize> = play_out(&mut q, Advance::Auto)
            .into_iter()
            .map(|i| q.items()[i])
            .collect();
        assert_eq!(played, [1, 9, 2, 3]);

        let mut q = queue(4, 1);
        q.set_shuffle(true);
        q.push_next(9);
        let current = q.current().unwrap();
        assert_eq!(q.items()[current], 1);
        let next = q.advance(Advance::Manual).unwrap();
        assert_eq!(q.items()[next], 9);
    }

    #[test]
    fn plays_in_order_and_stops_at_the_end_with_repeat_off() {
        let mut q = queue(3, 0);
        assert_eq!(play_out(&mut q, Advance::Auto), [0, 1, 2]);
        assert_eq!(q.current(), Some(2));
    }

    #[test]
    fn repeat_all_wraps_and_previous_wraps_back() {
        let mut q = queue(3, 2);
        q.set_repeat(Repeat::All);
        assert_eq!(q.advance(Advance::Auto), Some(0));
        assert_eq!(q.previous(), Some(2));
    }

    #[test]
    fn repeat_one_loops_on_auto_but_manual_next_moves_on() {
        let mut q = queue(3, 1);
        q.set_repeat(Repeat::One);
        assert_eq!(q.advance(Advance::Auto), Some(1));
        assert_eq!(q.advance(Advance::Manual), Some(2));
    }

    #[test]
    fn previous_at_the_start_stays_without_repeat() {
        let mut q = queue(3, 0);
        assert_eq!(q.previous(), Some(0));
    }

    #[test]
    fn shuffle_keeps_the_current_track_first_and_plays_each_once() {
        let mut q = queue(20, 7);
        q.set_shuffle(true);
        assert_eq!(q.current(), Some(7));
        let mut seen = play_out(&mut q, Advance::Auto);
        assert_eq!(seen[0], 7);
        seen.sort();
        assert_eq!(seen, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn turning_shuffle_off_keeps_the_current_track_and_restores_the_order() {
        let mut q = queue(10, 0);
        q.set_shuffle(true);
        q.advance(Advance::Manual);
        q.advance(Advance::Manual);
        let current = q.current().unwrap();
        q.set_shuffle(false);
        assert_eq!(q.current(), Some(current));
        assert_eq!(
            q.advance(Advance::Manual),
            (current + 1 < 10).then_some(current + 1)
        );
    }

    #[test]
    fn shuffled_repeat_all_starts_a_new_lap_without_repeating_the_last_track_first() {
        let mut q = queue(5, 0);
        q.set_shuffle(true);
        q.set_repeat(Repeat::All);
        for _ in 0..50 {
            let last = (0..4)
                .map(|_| q.advance(Advance::Auto))
                .last()
                .unwrap()
                .unwrap();
            assert_ne!(q.advance(Advance::Auto), Some(last));
        }
    }

    #[test]
    fn removing_the_current_track_moves_to_the_one_after_it() {
        let mut q = queue(4, 1);
        q.remove(1);
        assert_eq!(q.items(), [0, 2, 3]);
        assert_eq!(q.current(), Some(1));
        assert_eq!(q.items()[q.current().unwrap()], 2);
    }

    #[test]
    fn removing_an_earlier_track_keeps_the_current_one() {
        let mut q = queue(4, 2);
        q.remove(0);
        assert_eq!(q.items()[q.current().unwrap()], 2);
    }

    #[test]
    fn removing_the_last_item_empties_the_queue() {
        let mut q = queue(1, 0);
        q.remove(0);
        assert_eq!(q.current(), None);
        assert_eq!(q.advance(Advance::Auto), None);
    }

    #[test]
    fn pushed_tracks_play_after_the_rest() {
        let mut q = queue(2, 0);
        q.push(9);
        assert_eq!(play_out(&mut q, Advance::Auto), [0, 1, 2]);
        assert_eq!(q.items()[2], 9);
    }

    #[test]
    fn swapping_keeps_the_current_track_current_and_the_play_order_of_tracks() {
        let mut q = queue(4, 1);
        q.swap(1, 2);
        assert_eq!(q.items(), [0, 2, 1, 3]);
        assert_eq!(q.items()[q.current().unwrap()], 1);
        assert_eq!(q.advance(Advance::Manual).map(|i| q.items()[i]), Some(3));

        let mut q = queue(6, 0);
        q.set_shuffle(true);
        let before: Vec<usize> = play_out(&mut q.clone(), Advance::Auto)
            .iter()
            .map(|&i| q.items()[i])
            .collect();
        q.swap(1, 4);
        let after: Vec<usize> = play_out(&mut q.clone(), Advance::Auto)
            .iter()
            .map(|&i| q.items()[i])
            .collect();
        assert_eq!(before, after);
    }

    #[test]
    fn clear_empties_the_queue_but_keeps_the_modes() {
        let mut q = queue(3, 0);
        q.set_repeat(Repeat::All);
        q.set_shuffle(true);
        q.clear();
        assert_eq!(
            (q.current(), q.items().len(), q.repeat(), q.shuffle()),
            (None, 0, Repeat::All, true)
        );
    }

    #[test]
    fn an_empty_queue_has_nothing_to_play() {
        let mut q: Queue<usize> = Queue::default();
        assert_eq!(q.advance(Advance::Manual), None);
        assert_eq!(q.previous(), None);
        assert!(!q.jump(0));
    }
}
