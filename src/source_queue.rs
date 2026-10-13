use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};

use rand::{rng, seq::SliceRandom};

use crate::source_builder::SourceType;

struct SourceQueueContents {
    previous: Vec<Arc<SourceType>>,
    upcoming: VecDeque<Arc<SourceType>>,
}

struct SourceQueue {
    contents: Mutex<SourceQueueContents>,
}

impl SourceQueue {
    fn new() -> SourceQueue {
        SourceQueue {
            contents: Mutex::new(SourceQueueContents {
                previous: Vec::new(),
                upcoming: VecDeque::new(),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, SourceQueueContents> {
        self.contents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn enqueue(&self, item: SourceType) {
        self.lock().upcoming.push_back(Arc::new(item));
    }

    fn current_to_previous(&self) {
        let mut contents = self.lock();

        if let Some(item) = contents.upcoming.pop_front() {
            contents.previous.push(item);
        }
    }

    fn peek(&self) -> Option<Arc<SourceType>> {
        self.lock().upcoming.front().cloned()
    }

    fn is_empty(&self) -> bool {
        self.lock().upcoming.is_empty()
    }

    fn snapshot(&self) -> Vec<Arc<SourceType>> {
        let contents = self.lock();

        contents
            .previous
            .iter()
            .chain(contents.upcoming.iter())
            .cloned()
            .collect()
    }

    fn prev_len(&self) -> usize {
        self.lock().previous.len()
    }

    fn previous_to_current(&self) {
        let mut contents = self.lock();
        if let Some(item) = contents.previous.pop() {
            contents.upcoming.push_front(item);
        }
    }

    fn shuffle_queue(&self, shuffle_first: bool) {
        let upcoming = &mut self.lock().upcoming;
        if upcoming.len() > 1 {
            upcoming
                .make_contiguous()
                .split_at_mut(if shuffle_first { 0 } else { 1 })
                .1
                .shuffle(&mut rng());
        }
    }

    fn clear(&self) {
        let mut contents = self.lock();
        contents.previous.clear();
        contents.upcoming.clear();
    }

    fn swap(&self, mut insert_index: usize, mut indexes_to_insert: Vec<usize>) {
        let mut contents = self.lock();
        let mut temp_vec: Vec<Arc<SourceType>> = Vec::new();
        indexes_to_insert.sort_by(|a, b| b.cmp(a));
        let prev_len = contents.previous.len();
        let mut decrement = 0;
        let max_selected = indexes_to_insert.first().copied();
        for index in indexes_to_insert {
            if index < insert_index {
                decrement += 1;
            }

            if index < prev_len {
                temp_vec.push(contents.previous.remove(index))
            } else if let Some(source) = contents.upcoming.remove(index - prev_len) {
                temp_vec.push(source);
            }
        }
        temp_vec.reverse();

        if decrement > 0 {
            insert_index += 1;
        }
        insert_index -= decrement;
        if insert_index == contents.previous.len() {
            insert_index += 1;
        }

        if let Some(max) = max_selected {
            if decrement > 0 && max + 1 == insert_index + decrement {
                insert_index += 1;
            }
        }

        if insert_index < contents.previous.len() {
            contents
                .previous
                .splice(insert_index..insert_index, temp_vec);
        } else {
            let at = (insert_index - contents.previous.len()).min(contents.upcoming.len());
            let tail = contents.upcoming.split_off(at);
            contents.upcoming.extend(temp_vec);
            contents.upcoming.extend(tail);
        }
    }

    fn delete_at(&self, index: usize){
        let mut contents = self.lock();
        if index < contents.previous.len(){
            contents.previous.remove(index);
        }
        else{
            contents.upcoming.remove(index);
        }
    }
}

#[derive(Clone)]
pub struct SharedQueue {
    inner: Arc<SourceQueue>,
}

impl SharedQueue {
    pub fn new() -> SharedQueue {
        SharedQueue {
            inner: Arc::new(SourceQueue::new()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn previous_to_current(&self) {
        self.inner.previous_to_current();
    }

    pub fn current_to_previous(&self) {
        self.inner.current_to_previous();
    }

    pub fn peek(&self) -> Option<Arc<SourceType>> {
        self.inner.peek()
    }

    pub fn prev_len(&self) -> usize {
        self.inner.prev_len()
    }

    pub fn snapshot(&self) -> Vec<Arc<SourceType>> {
        self.inner.snapshot()
    }

    pub fn enqueue(&self, item: SourceType) {
        self.inner.enqueue(item);
    }

    pub fn shuffle_queue(&self, shuffle_first: bool) {
        self.inner.shuffle_queue(shuffle_first);
    }

    pub fn clear(&self) {
        self.inner.clear();
    }

    pub fn swap(&self, insert_index: usize, indexes_to_insert: &Vec<usize>) {
        self.inner.swap(insert_index, indexes_to_insert.clone());
    }

    pub fn delete_at(&self, index: usize){
        self.inner.delete_at(index);
    }
}
