use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock},
};

use rand::{rng, seq::SliceRandom};

use crate::source_builder::SourceType;

struct SourceQueueContents {
    previous: Vec<SourceType>,
    upcoming: VecDeque<SourceType>,
}

struct SourceQueue {
    contents: RwLock<SourceQueueContents>,
}

impl SourceQueue {
    fn new() -> SourceQueue {
        SourceQueue {
            contents: RwLock::new(SourceQueueContents {
                previous: Vec::new(),
                upcoming: VecDeque::new(),
            }),
        }
    }

    fn enqueue(&self, item: SourceType) {
        self.contents
            .write()
            .expect("RwLock acquired")
            .upcoming
            .push_back(item);
    }

    fn current_to_previous(&self) {
        let mut contents = self.contents.write().expect("queue lock poisoned");

        if let Some(item) = contents.upcoming.pop_front() {
            contents.previous.push(item);
        }
    }

    fn peek(&self) -> Option<SourceType> {
        self.contents
            .read()
            .expect("RwLock acquired")
            .upcoming
            .front()
            .cloned()
    }

    fn is_empty(&self) -> bool {
        self.contents
            .read()
            .expect("RwLock acquired")
            .upcoming
            .is_empty()
    }

    fn snapshot(&self) -> (usize, Vec<SourceType>) {
        let contents = self.contents.read().expect("queue lock poisoned");
        let prev_len = contents.previous.len();

        let items = contents
            .previous
            .iter()
            .chain(contents.upcoming.iter())
            .cloned()
            .collect();

        (prev_len, items)
    }

    fn prev_len(&self) -> usize {
        self.contents
            .read()
            .expect("RwLock acquired")
            .previous
            .len()
    }

    fn previous_to_current(&self) {
        let mut contents = self.contents.write().expect("queue lock poisoned");
        if let Some(item) = contents.previous.pop() {
            contents.upcoming.push_front(item);
        }
    }

    fn shuffle(&self) {
        let upcoming = &mut self.contents.write().expect("queue lock poisoned").upcoming;
        if upcoming.len() > 1 {
            upcoming
                .make_contiguous()
                .split_at_mut(1)
                .1
                .shuffle(&mut rng());
        }
    }

    fn clear(&self){
        let mut contents = self.contents.write().expect("queue lock poisoned");
        contents.previous.clear();
        contents.upcoming.clear();
    }
}

#[derive(Clone)]
pub struct SharedQueue {
    inner: Arc<Mutex<SourceQueue>>,
}

impl SharedQueue {
    pub fn new() -> SharedQueue {
        SharedQueue {
            inner: Arc::new(Mutex::new(SourceQueue::new())),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().is_empty()
    }

    pub fn previous_to_current(&self) {
        self.inner.lock().unwrap().previous_to_current();
    }

    pub fn current_to_previous(&self) {
        self.inner.lock().unwrap().current_to_previous();
    }

    pub fn peek(&self) -> Option<SourceType> {
        self.inner.lock().unwrap().peek()
    }

    pub fn prev_len(&self) -> usize {
        self.inner.lock().unwrap().prev_len()
    }

    pub fn snapshot(&self) -> (usize, Vec<SourceType>) {
        self.inner.lock().unwrap().snapshot()
    }

    pub fn enqueue(&self, item: SourceType) {
        self.inner.lock().unwrap().enqueue(item);
    }

    pub fn shuffle(&self) {
        self.inner.lock().unwrap().shuffle();
    }

    pub fn clear(&self){
        self.inner.lock().unwrap().clear();
    }
}
