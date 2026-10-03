use std::{collections::VecDeque, sync::RwLock};

use crate::source_builder::SourceType;

struct SourceQueueContents {
    previous: Vec<SourceType>,
    upcoming: VecDeque<SourceType>,
}

pub struct SourceQueue {
    contents: RwLock<SourceQueueContents>,
}

impl SourceQueue {
    pub fn new() -> SourceQueue {
        SourceQueue {
            contents: RwLock::new(SourceQueueContents {
                previous: Vec::new(),
                upcoming: VecDeque::new(),
            }),
        }
    }

    pub fn enqueue(&self, item: SourceType) {
        self.contents
            .write()
            .expect("RwLock acquired")
            .upcoming
            .push_back(item);
    }

    pub fn current_to_previous(&self) {
        let mut contents = self.contents.write().expect("queue lock poisoned");

        if let Some(item) = contents.upcoming.pop_front() {
            contents.previous.push(item);
        }
    }

    pub fn peek(&self) -> Option<SourceType> {
        self.contents
            .read()
            .expect("RwLock acquired")
            .upcoming
            .front()
            .cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.contents
            .read()
            .expect("RwLock acquired")
            .upcoming
            .is_empty()
    }

    pub fn snapshot(&self) -> (usize, Vec<SourceType>) {
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

    pub fn prev_len(&self) -> usize {
        self.contents
            .read()
            .expect("RwLock acquired")
            .previous
            .len()
    }

    pub fn replay(&self) {
        let mut contents = self.contents.write().expect("queue lock poisoned");
        if let Some(item) = contents.previous.pop() {
            contents.upcoming.push_front(item);
        }
    }
}