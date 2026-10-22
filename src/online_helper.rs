use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::{Arc, OnceLock},
    thread,
};

use crate::{source_builder::SourceType, source_queue::SharedQueue};

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

    pub fn fetch_urls(queue: SharedQueue, urls: Vec<String>) {
        thread::spawn(move || {
            Self::fetch_url_internal(&queue, urls);
        });
    }

    fn fetch_url_internal(queue: &SharedQueue, urls: Vec<String>) {
        let inputs: Vec<String> = urls
            .into_iter()
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty())
            .collect();

        if inputs.is_empty() {
            return;
        }

        let expanded = Self::expand_urls(&inputs);
        if expanded.is_empty() {
            return;
        }

        let sources: Vec<Arc<SourceType>> = expanded
            .into_iter()
            .map(|u| Arc::new(SourceType::Url(u)))
            .collect();

        for source in &sources {
            queue.enqueue(source.clone());
        }
    }

    fn expand_urls(inputs: &[String]) -> Vec<String> {
        let mut args = Self::ytdlp_default_args();
        args.extend(
            [
                "--flat-playlist",
                "--no-playlist",
                "--ignore-errors",
                "-a",
                "-",
                "--print",
                "%(webpage_url,url)s",
                "--match-filter",
                r"title!~=^\[(Private|Deleted) video\]$",
            ]
            .map(String::from),
        );

        let Ok(mut child) = Command::new("yt-dlp")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            return Vec::new();
        };

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(inputs.join("\n").as_bytes());
        }

        let mut result = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let line = line.trim();
                if !line.is_empty() && line != "NA" {
                    result.push(line.to_string());
                }
            }
        }
        let _ = child.wait();
        result
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
