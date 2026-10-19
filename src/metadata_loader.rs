use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        mpsc::{self, Sender},
    },
    thread,
};

use crate::{
    enums::Clmp3Event,
    metadata::{Metadata, SharedLoaderSlot},
    source_builder::{SourceId, SourceType},
};

pub struct MetadataLoader {
    loader_jobs: mpsc::Sender<Vec<Arc<SourceType>>>,
    pic_jobs: mpsc::Sender<Arc<SourceType>>,
    slot: SharedLoaderSlot,
}

impl Drop for MetadataLoader {
    fn drop(&mut self) {
        let mut s = self.slot.lock().unwrap();
        s.shutdown = true;
        if let Some(child) = &mut s.ytdlp {
            let _ = child.kill();
        }
    }
}

pub type MetadataMap = Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>;

impl MetadataLoader {
    pub fn new(event_tx: Sender<Clmp3Event>, map: MetadataMap) -> MetadataLoader {
        let slot = SharedLoaderSlot::default();
        let loader_jobs = Self::spawn_loader_thread(map.clone(), slot.clone());
        let pic_jobs = Self::spawn_pic_thread(event_tx, map);

        MetadataLoader {
            loader_jobs,
            pic_jobs,
            slot,
        }
    }

    fn spawn_loader_thread(
        map: MetadataMap,
        slot: SharedLoaderSlot,
    ) -> mpsc::Sender<Vec<Arc<SourceType>>> {
        let (jobs, inbox) = mpsc::channel::<Vec<Arc<SourceType>>>();

        thread::spawn(move || {
            for batch in inbox {
                if slot.lock().unwrap().shutdown {
                    break;
                }
                let online_data = Self::handle_local_and_get_online_data(batch, &map);
                Self::handle_online_sources(online_data, &map, &slot);
            }
        });
        jobs
    }

    fn handle_local_and_get_online_data(
        batch: Vec<Arc<SourceType>>,
        map: &MetadataMap,
    ) -> (Vec<String>, HashMap<String, Arc<SourceType>>) {
        let mut urls: Vec<String> = Vec::new();
        let mut pending: HashMap<String, Arc<SourceType>> = HashMap::new(); // need to remember which source belongs to which url

        for source in batch {
            if map.read().unwrap().contains_key(&source.id()) {
                continue;
            }
            match &*source {
                SourceType::File(path_buf) => {
                    let m = Metadata::from_file(source.clone(), path_buf);
                    map.write().unwrap().insert(source.id(), Arc::new(m));
                }
                SourceType::Url(url) => {
                    if pending.insert(url.clone(), source.clone()).is_none() {
                        urls.push(url.clone());
                    }
                }
            }
        }
        (urls, pending)
    }

    fn handle_online_sources(
        online_data: (Vec<String>, HashMap<String, Arc<SourceType>>),
        map: &MetadataMap,
        slot: &SharedLoaderSlot,
    ) {
        let (urls, pending) = online_data;

        if urls.is_empty() {
            return; // only local files were in this batch
        }

        Metadata::from_urls(&urls, slot, |data| {
            if let Some(source) = pending.get(&data.url) {
                let meta = Metadata::new(
                    data.title.unwrap_or_else(|| source.fallback_label()),
                    data.artist,
                    data.album,
                    source.clone(),
                    data.thumbnail_url,
                );
                map.write().unwrap().insert(source.id(), Arc::new(meta));
            }
        });

        // online sources that didn't get added to map yet need to use fallback data
        for url in &urls {
            let source = &pending[url];
            let missing = !map.read().unwrap().contains_key(&source.id());
            if missing {
                let meta = Metadata::new(source.fallback_label(), None, None, source.clone(), None);
                map.write().unwrap().insert(source.id(), Arc::new(meta));
            }
        }
    }

    fn spawn_pic_thread(
        event_tx: Sender<Clmp3Event>,
        map: MetadataMap,
    ) -> mpsc::Sender<Arc<SourceType>> {
        let (jobs, inbox) = mpsc::channel::<Arc<SourceType>>();

        thread::spawn(move || {
            while let Ok(mut source) = inbox.recv() {
                while let Ok(newer) = inbox.try_recv() {
                    // skip old entries until arrived at newest
                    source = newer;
                }
                let img = match &*source.clone() {
                    SourceType::File(path_buf) => Metadata::pic_from_file(path_buf),
                    SourceType::Url(url) => {
                        let thumb = map
                            .read()
                            .unwrap()
                            .get(&source.id())
                            .and_then(|m| m.thumbnail_url.clone());
                        Metadata::pic_from_url(url, thumb)
                    }
                };
                if event_tx.send(Clmp3Event::CoverLoaded(source, img)).is_err() {
                    break;
                }
            }
        });

        jobs
    }

    pub fn load(&self, source: Arc<SourceType>) {
        self.load_many(vec![source]);
    }

    pub fn load_many(&self, sources: Vec<Arc<SourceType>>) {
        let _ = self.loader_jobs.send(sources);
    }

    pub fn load_pic(&self, source: Arc<SourceType>) {
        let _ = self.pic_jobs.send(source);
    }
}
