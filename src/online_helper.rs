use std::{
    sync::{Arc, OnceLock},
    thread,
};

use crate::{metadata::MetadataLoader, source_builder::SourceType, source_queue::SharedQueue};

static BROWSER_FOR_COOKIES: OnceLock<String> = OnceLock::new();
static COOKIES_PATH: OnceLock<String> = OnceLock::new();

pub struct OnlineHelper {}

impl OnlineHelper {
    pub fn set_cookie_browser(browser: String) {
        let _ = BROWSER_FOR_COOKIES.set(browser);
    }

    pub fn set_cookie_path(path: String) {
        let _ = COOKIES_PATH.set(path);
    }

    pub fn fetch_urls(queue: SharedQueue, loader: Arc<MetadataLoader>, urls: Vec<String>) {
        thread::spawn(move || {
            Self::fetch_url_internal(&queue, &loader, urls);
        });
    }

    fn fetch_url_internal(queue: &SharedQueue, loader: &Arc<MetadataLoader>, urls: Vec<String>) {
        let sources: Vec<Arc<SourceType>> = urls
            .into_iter()
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty())
            .map(|u| Arc::new(SourceType::Url(u)))
            .collect();

        for source in &sources {
            queue.enqueue(source.clone());
        }
        loader.load_many(sources);
    }

    pub fn ytdlp_default_args() -> Vec<String> {
        let mut args = vec!["--remote-components".to_string(), "ejs:github".to_string()];
        if let Some(path) = COOKIES_PATH.get() {
            args.push("--cookies".to_string());
            args.push(path.clone());
        } else if let Some(browser) = BROWSER_FOR_COOKIES.get() {
            args.push("--cookies-from-browser".to_string());
            args.push(browser.clone());
        }
        args
    }
}
