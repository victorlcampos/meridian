//! Downloads in a background thread, so a slow network never freezes the clock.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use chrono::Utc;

use crate::calendar::{self, Event};
use crate::maccal;

pub enum Job {
    /// Download an address, or read a local file.
    Get(String),
    /// Read the macOS Calendar, asking macOS for access first when `ask`.
    MacCalendar { ask: bool },
}

#[derive(Debug, PartialEq)]
pub enum Answer {
    Text(Result<String, String>),
    MacCalendar(Result<Vec<Event>, maccal::Refusal>),
}

/// A finished job: the key it was asked with, and its answer.
pub type Download = (String, Answer);

pub struct Net {
    jobs: Sender<(String, Job)>,
    done: Receiver<Download>,
}

impl Net {
    /// Starts the download thread. Addresses that are not `http(s)://` are read
    /// as local files, and `webcal://` is fetched over https.
    pub fn start() -> Self {
        let (jobs, queue) = mpsc::channel::<(String, Job)>();
        let (report, done) = mpsc::channel();
        std::thread::spawn(move || {
            // Certificates are checked against the system's trust store, like
            // browsers and curl do: networks that inspect TLS (at work, in a
            // coworking) sign with roots only the system knows.
            let tls = ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build();
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(20)))
                .tls_config(tls)
                .build()
                .into();
            for (key, job) in queue {
                match job {
                    Job::Get(address) => {
                        let answer = Answer::Text(fetch(&agent, &address));
                        if report.send((key, answer)).is_err() {
                            break;
                        }
                    }
                    Job::MacCalendar { ask } => {
                        // Asking waits for the user to answer macOS: never hold up the downloads.
                        let report = report.clone();
                        std::thread::spawn(move || {
                            let now = Utc::now();
                            let until = now + chrono::Duration::days(calendar::WINDOW_DAYS);
                            let events = maccal::read(ask, now, until);
                            let _ = report.send((key, Answer::MacCalendar(events)));
                        });
                    }
                }
            }
        });
        Self { jobs, done }
    }

    pub fn fetch(&self, key: String, address: String) {
        // The thread only stops when the app does.
        let _ = self.jobs.send((key, Job::Get(address)));
    }

    pub fn mac_calendar(&self, key: String, ask: bool) {
        let _ = self.jobs.send((key, Job::MacCalendar { ask }));
    }

    /// The jobs that finished since the last call.
    pub fn finished(&self) -> Vec<Download> {
        self.done.try_iter().collect()
    }

    /// A `Net` without a thread: the test sees the jobs and answers them.
    #[cfg(test)]
    pub fn manual() -> (Self, Receiver<(String, Job)>, Sender<Download>) {
        let (jobs, queue) = mpsc::channel();
        let (report, done) = mpsc::channel();
        (Self { jobs, done }, queue, report)
    }
}

fn fetch(agent: &ureq::Agent, address: &str) -> Result<String, String> {
    let address = match address.strip_prefix("webcal://") {
        Some(rest) => format!("https://{rest}"),
        None => address.to_owned(),
    };
    if !(address.starts_with("https://") || address.starts_with("http://")) {
        return std::fs::read_to_string(&address).map_err(|error| error.to_string());
    }
    agent
        .get(&address)
        .call()
        .and_then(|mut response| {
            response
                .body_mut()
                .with_config()
                .limit(64 * 1024 * 1024)
                .read_to_string()
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_local_files_through_the_thread() {
        let path = std::env::temp_dir().join(format!("meridian-net-{}.txt", std::process::id()));
        std::fs::write(&path, "hello").unwrap();
        let net = Net::start();
        net.fetch("file".into(), path.display().to_string());
        net.fetch("missing".into(), "/no/such/meridian/file".into());
        let mut done = Vec::new();
        while done.len() < 2 {
            done.extend(net.finished());
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            done[0],
            ("file".to_owned(), Answer::Text(Ok("hello".to_owned())))
        );
        assert!(matches!(done[1].1, Answer::Text(Err(_))));
        let _ = std::fs::remove_file(path);
    }
}
