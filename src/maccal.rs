//! Events from the macOS Calendar app, through EventKit: whatever accounts it
//! syncs, Google included.
//!
//! macOS decides on calendar access for the "responsible" app, which for a
//! command-line tool is normally its terminal, and terminals built with the
//! hardened runtime and no calendar entitlement (cmux, for one) make it refuse
//! without asking. So EventKit runs in a helper, `meridian --calendar-helper`,
//! started as responsible for itself: macOS then asks on behalf of meridian,
//! with the reason in the Info.plist embedded by build.rs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::calendar::Event;

/// Whether this system has the Calendar app to read from.
pub const SUPPORTED: bool = cfg!(target_os = "macos");

/// System Settings › Privacy & Security › Calendars.
pub const PRIVACY_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars";
/// System Settings › Internet Accounts, where a Google account is added.
pub const ACCOUNTS_SETTINGS: &str =
    "x-apple.systempreferences:com.apple.Internet-Accounts-Settings.extension";

/// Access to the calendars, as macOS reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    Granted,
    Denied,
    NotAsked,
}

/// Why the calendars could not be read: the access macOS reports and, when a
/// request just failed, what macOS said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub access: Access,
    pub detail: Option<String>,
}

/// An event as EventKit describes it, before deciding whether to show it.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Raw {
    id: String,
    title: String,
    start: f64,
    all_day: bool,
    cancelled: bool,
    declined: bool,
}

/// What the helper prints.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Reply {
    access: Access,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    events: Vec<Raw>,
}

/// All-day, cancelled and declined events are left out.
#[cfg(any(target_os = "macos", test))]
fn keep(raw: Raw) -> Option<Event> {
    if raw.all_day || raw.cancelled || raw.declined {
        return None;
    }
    let start = DateTime::from_timestamp(raw.start.floor() as i64, 0)?;
    Some(Event {
        uid: raw.id,
        title: raw.title,
        start,
    })
}

/// Events starting in `[from, until)` in every calendar, earliest first. With
/// `ask`, macOS is asked for access first if it never was (its prompt shows
/// then, and this waits for the answer).
pub fn read(ask: bool, from: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<Event>, Refusal> {
    #[cfg(target_os = "macos")]
    {
        let (from_arg, until_arg) = (from.timestamp().to_string(), until.timestamp().to_string());
        let mode = if ask { "ask" } else { "check" };
        let text = helper::run(&["--calendar-helper", "read", mode, &from_arg, &until_arg])
            .map_err(|error| Refusal {
                access: Access::Denied,
                detail: Some(error.to_string()),
            })?;
        parse_reply(&text, from, until)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (ask, from, until);
        Err(Refusal {
            access: Access::Denied,
            detail: None,
        })
    }
}

#[cfg(any(target_os = "macos", test))]
fn parse_reply(
    text: &str,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Result<Vec<Event>, Refusal> {
    let Ok(reply) = serde_json::from_str::<Reply>(text.trim()) else {
        return Err(Refusal {
            access: Access::Denied,
            detail: Some("the calendar helper did not answer".into()),
        });
    };
    if reply.access != Access::Granted {
        return Err(Refusal {
            access: reply.access,
            detail: reply.detail,
        });
    }
    let mut events: Vec<Event> = reply
        .events
        .into_iter()
        .filter_map(keep)
        .filter(|event| event.start >= from && event.start < until)
        .collect();
    events.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.title.cmp(&b.title)));
    Ok(events)
}

/// The helper side: `read ask|check <from> <until>` prints a JSON reply.
pub fn serve(args: &[String]) -> std::process::ExitCode {
    let [command, mode, from, until] = args else {
        return std::process::ExitCode::FAILURE;
    };
    let (Ok(from), Ok(until)) = (from.parse::<f64>(), until.parse::<f64>()) else {
        return std::process::ExitCode::FAILURE;
    };
    if command != "read" {
        return std::process::ExitCode::FAILURE;
    }
    #[cfg(target_os = "macos")]
    let reply = eventkit::read(mode == "ask", from, until);
    #[cfg(not(target_os = "macos"))]
    let reply = {
        let _ = (mode, from, until);
        Reply {
            access: Access::Denied,
            detail: None,
            events: Vec::new(),
        }
    };
    println!("{}", serde_json::to_string(&reply).unwrap_or_default());
    std::process::ExitCode::SUCCESS
}

#[cfg(target_os = "macos")]
mod helper {
    use std::ffi::{CString, c_char, c_int};
    use std::fs::File;
    use std::io::{self, Read};
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStringExt;

    unsafe extern "C" {
        /// Private but long-standing (macOS 10.14+, used by Chromium): the
        /// child answers for itself in macOS's privacy checks.
        fn responsibility_spawnattrs_setdisclaim(
            attrs: *mut libc::posix_spawnattr_t,
            disclaim: c_int,
        ) -> c_int;
    }

    /// Runs this executable with `args`, responsible for itself, and returns
    /// what it prints.
    pub fn run(args: &[&str]) -> io::Result<String> {
        let program = CString::new(std::env::current_exe()?.into_os_string().into_vec())?;
        let mut owned = vec![program.clone()];
        for arg in args {
            owned.push(CString::new(*arg)?);
        }
        let mut argv: Vec<*mut c_char> = owned.iter().map(|arg| arg.as_ptr().cast_mut()).collect();
        argv.push(std::ptr::null_mut());
        let null = c"/dev/null";
        let mut pipe = [0; 2];
        // SAFETY: plain POSIX calls on values owned here; every pointer passed
        // lives until posix_spawn returns.
        unsafe {
            if libc::pipe(pipe.as_mut_ptr()) != 0 {
                return Err(io::Error::last_os_error());
            }
            let mut attr: libc::posix_spawnattr_t = std::ptr::null_mut();
            libc::posix_spawnattr_init(&mut attr);
            // Only the descriptors set up below reach the helper, not the terminal's.
            libc::posix_spawnattr_setflags(
                &mut attr,
                libc::POSIX_SPAWN_CLOEXEC_DEFAULT as libc::c_short,
            );
            responsibility_spawnattrs_setdisclaim(&mut attr, 1);
            let mut actions: libc::posix_spawn_file_actions_t = std::ptr::null_mut();
            libc::posix_spawn_file_actions_init(&mut actions);
            libc::posix_spawn_file_actions_addopen(
                &mut actions,
                0,
                null.as_ptr(),
                libc::O_RDONLY,
                0,
            );
            libc::posix_spawn_file_actions_adddup2(&mut actions, pipe[1], 1);
            libc::posix_spawn_file_actions_addopen(
                &mut actions,
                2,
                null.as_ptr(),
                libc::O_WRONLY,
                0,
            );
            let mut pid = 0;
            let spawned = libc::posix_spawn(
                &mut pid,
                program.as_ptr(),
                &actions,
                &attr,
                argv.as_ptr(),
                (*libc::_NSGetEnviron()).cast_const(),
            );
            libc::posix_spawn_file_actions_destroy(&mut actions);
            libc::posix_spawnattr_destroy(&mut attr);
            libc::close(pipe[1]);
            let mut output = File::from_raw_fd(pipe[0]);
            if spawned != 0 {
                return Err(io::Error::from_raw_os_error(spawned));
            }
            let mut text = String::new();
            let read = output.read_to_string(&mut text);
            let mut status = 0;
            libc::waitpid(pid, &mut status, 0);
            read?;
            Ok(text)
        }
    }
}

#[cfg(target_os = "macos")]
mod eventkit {
    use std::sync::mpsc;
    use std::time::Duration;

    use block2::RcBlock;
    use objc2::rc::autoreleasepool;
    use objc2::runtime::{Bool, NSObjectProtocol};
    use objc2::sel;
    use objc2_event_kit::{
        EKAuthorizationStatus, EKEntityType, EKEventStatus, EKEventStore, EKParticipantStatus,
    };
    use objc2_foundation::{NSDate, NSError};

    use super::{Access, Raw, Reply};

    /// Asks for access first when `ask` and it never was, then reads.
    pub fn read(ask: bool, from: f64, until: f64) -> Reply {
        let mut detail = None;
        if ask && access() == Access::NotAsked {
            detail = request_access();
        }
        let access = access();
        let events = match access {
            Access::Granted => events(from, until),
            _ => Vec::new(),
        };
        Reply {
            access,
            detail,
            events,
        }
    }

    pub fn access() -> Access {
        // SAFETY: a class method taking a plain enum value.
        let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
        match status {
            EKAuthorizationStatus::FullAccess => Access::Granted,
            EKAuthorizationStatus::NotDetermined => Access::NotAsked,
            _ => Access::Denied,
        }
    }

    /// Shows macOS's prompt and waits for the answer; what macOS said when it
    /// did not grant access.
    fn request_access() -> Option<String> {
        autoreleasepool(|_| {
            // SAFETY: `init` of a plain NSObject subclass.
            let store = unsafe { EKEventStore::new() };
            // Before macOS 14 the method does not exist; calling it would abort.
            if !store.respondsToSelector(sel!(requestFullAccessToEventsWithCompletion:)) {
                return Some("macOS 14 or newer is needed".into());
            }
            let (answer, reply) = mpsc::channel();
            let block = RcBlock::new(move |granted: Bool, error: *mut NSError| {
                // SAFETY: EventKit passes a valid error or null.
                let detail =
                    unsafe { error.as_ref() }.map(|error| error.localizedDescription().to_string());
                let _ = answer.send((granted.as_bool(), detail));
            });
            // SAFETY: the block is alive for the call and EventKit copies it.
            unsafe { store.requestFullAccessToEventsWithCompletion(RcBlock::as_ptr(&block)) };
            // The store has to outlive the request: dropping it cancels the prompt.
            let reply = reply.recv_timeout(Duration::from_secs(600));
            drop(store);
            match reply {
                Ok((true, _)) => None,
                Ok((false, detail)) => detail,
                Err(_) => Some("macOS did not answer".into()),
            }
        })
    }

    fn events(from: f64, until: f64) -> Vec<Raw> {
        autoreleasepool(|_| {
            // SAFETY: EventKit calls on objects created here, used on this thread only.
            unsafe {
                let store = EKEventStore::new();
                let start = NSDate::dateWithTimeIntervalSince1970(from);
                let end = NSDate::dateWithTimeIntervalSince1970(until);
                let predicate =
                    store.predicateForEventsWithStartDate_endDate_calendars(&start, &end, None);
                store
                    .eventsMatchingPredicate(&predicate)
                    .iter()
                    .map(|event| {
                        let declined = event.attendees().is_some_and(|people| {
                            people.iter().any(|person| {
                                person.isCurrentUser()
                                    && person.participantStatus() == EKParticipantStatus::Declined
                            })
                        });
                        let id = event
                            .calendarItemExternalIdentifier()
                            .or_else(|| event.eventIdentifier())
                            .map(|id| id.to_string())
                            .unwrap_or_default();
                        Raw {
                            id,
                            title: event.title().to_string(),
                            start: event.startDate().timeIntervalSince1970(),
                            all_day: event.isAllDay(),
                            cancelled: event.status() == EKEventStatus::Canceled,
                            declined,
                        }
                    })
                    .collect()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(title: &str) -> Raw {
        Raw {
            id: "id".into(),
            title: title.into(),
            start: 1_790_355_600.0,
            all_day: false,
            cancelled: false,
            declined: false,
        }
    }

    fn at(timestamp: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(timestamp, 0).unwrap()
    }

    #[test]
    fn keeps_timed_events_and_leaves_out_the_rest() {
        let event = keep(raw("Review")).unwrap();
        assert_eq!(event.start.timestamp(), 1_790_355_600);
        assert_eq!(event.title, "Review");
        assert!(
            keep(Raw {
                all_day: true,
                ..raw("Holiday")
            })
            .is_none()
        );
        assert!(
            keep(Raw {
                cancelled: true,
                ..raw("Cancelled")
            })
            .is_none()
        );
        assert!(
            keep(Raw {
                declined: true,
                ..raw("Declined")
            })
            .is_none()
        );
    }

    #[test]
    fn reads_the_helper_reply() {
        let reply = Reply {
            access: Access::Granted,
            detail: None,
            events: vec![
                Raw {
                    start: 1_790_359_200.0,
                    ..raw("Later")
                },
                raw("Sooner"),
                Raw {
                    all_day: true,
                    ..raw("Holiday")
                },
                Raw {
                    start: 1_700_000_000.0,
                    ..raw("Past")
                },
            ],
        };
        let text = serde_json::to_string(&reply).unwrap();
        let events = parse_reply(&text, at(1_790_000_000), at(1_791_000_000)).unwrap();
        let titles: Vec<_> = events.iter().map(|event| event.title.as_str()).collect();
        assert_eq!(titles, ["Sooner", "Later"]);
    }

    #[test]
    fn reports_what_macos_said() {
        let text = r#"{"access":"not-asked","detail":"The request was refused"}"#;
        assert_eq!(
            parse_reply(text, at(0), at(1)),
            Err(Refusal {
                access: Access::NotAsked,
                detail: Some("The request was refused".into()),
            })
        );
        assert_eq!(
            parse_reply("", at(0), at(1)).unwrap_err().detail.as_deref(),
            Some("the calendar helper did not answer")
        );
    }

    /// Reading the permission in this process never prompts.
    #[cfg(target_os = "macos")]
    #[test]
    fn reports_access_without_asking() {
        let access = eventkit::access();
        assert!(matches!(
            access,
            Access::Granted | Access::Denied | Access::NotAsked
        ));
    }
}
