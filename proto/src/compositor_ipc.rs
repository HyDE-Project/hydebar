//! One request to the compositor, over the socket it already listens on.
//!
//! The compositor answers questions about itself two ways: a socket it opens
//! per session, and a command-line program that connects to that same socket
//! on the caller's behalf. Spawning the program costs a fork, an execve and a
//! dynamic link per question asked, and the bar asks ten of them every time it
//! reads the look — before its first surface is even mapped. Connecting
//! directly costs one open and one write.
//!
//! Two rules the compositor imposes are honoured here. Its request loop is
//! synchronous, so a connection left open stalls every other client until the
//! compositor times it out; the stream is therefore written, read and dropped
//! inside one call, and a read deadline keeps a wedged compositor from
//! becoming a wedged bar. Several questions travel as one batch, because the
//! round trip, not the question, is what costs.

use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::Duration
};

/// How long the bar waits for an answer before giving up on it.
///
/// The bar can draw without any of this — every reading it asks for is an
/// `Option` — so a compositor that has stopped answering costs a moment, not a
/// hang.
const ANSWER_DEADLINE: Duration = Duration::from_secs(1);

/// Sends one request and returns the answer.
///
/// Returns nothing when there is no compositor to ask, which is the normal
/// state under a different compositor and while the session is coming up.
///
/// ```no_run
/// use hydebar_proto::compositor_ipc;
///
/// let monitors = compositor_ipc::request("j/monitors");
/// ```
#[must_use]
pub fn request(command: &str) -> Option<String> {
    let mut stream = UnixStream::connect(socket_path()?).ok()?;
    stream.set_read_timeout(Some(ANSWER_DEADLINE)).ok()?;
    stream.write_all(command.as_bytes()).ok()?;
    stream.flush().ok()?;

    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;

    Some(answer)
}

/// Sends several requests as one batch and returns the answers together.
///
/// The compositor writes the answers back in the order asked, separated by
/// blank lines. Callers that ask in JSON get one answer per line and can read
/// them back by the option each names, which is why nothing here tries to
/// split them apart.
///
/// ```no_run
/// use hydebar_proto::compositor_ipc;
///
/// let answers = compositor_ipc::batch(["j/getoption general:gaps_in"]);
/// ```
#[must_use]
pub fn batch<'a>(commands: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let joined = commands.into_iter().collect::<Vec<_>>().join(";");

    if joined.is_empty() {
        return None;
    }

    request(&format!("[[BATCH]]{joined}"))
}

/// Name of the socket the compositor answers requests on.
const REQUEST_SOCKET: &str = ".socket.sock";

/// Name of the socket the compositor announces what happens on.
const EVENT_SOCKET: &str = ".socket2.sock";

/// Where the running compositor listens for requests.
fn socket_path() -> Option<PathBuf> {
    session_socket(REQUEST_SOCKET)
}

/// Where the running compositor announces what happens in the session.
///
/// Returns nothing when there is no compositor, which is the normal state
/// under a different one and while the session is coming up.
#[must_use]
pub fn event_socket_path() -> Option<PathBuf> {
    session_socket(EVENT_SOCKET)
}

/// The named socket of the running session, wherever the compositor put it.
fn session_socket(name: &str) -> Option<PathBuf> {
    locate(
        &non_empty("HYPRLAND_INSTANCE_SIGNATURE")?,
        non_empty("XDG_RUNTIME_DIR").as_deref(),
        name
    )
}

/// Picks the socket of the session named by `signature`.
///
/// The session directory is where a current compositor puts it; the temporary
/// directory is where every release before it did, and costs one `stat` to
/// keep working. Taking both as arguments keeps the choice testable without a
/// session to run in.
fn locate(signature: &str, runtime: Option<&str>, name: &str) -> Option<PathBuf> {
    runtime
        .map(|runtime| PathBuf::from(runtime).join("hypr"))
        .into_iter()
        .chain(std::iter::once(PathBuf::from("/tmp/hypr")))
        .map(|root| root.join(signature).join(name))
        .find(|path| path.exists())
}

/// Value of an environment variable, treating an empty one as unset.
fn non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn an_empty_batch_asks_nothing() {
        assert_eq!(batch(std::iter::empty()), None);
    }

    #[test]
    fn a_session_that_left_no_socket_behind_is_not_reached_for() {
        assert_eq!(
            locate("no_such_session", Some("/run/user/1000"), REQUEST_SOCKET),
            None
        );
    }

    #[test]
    fn a_session_without_a_runtime_directory_still_looks_in_the_old_place() {
        assert_eq!(locate("no_such_session", None, REQUEST_SOCKET), None);
    }

    #[test]
    fn the_socket_of_a_live_session_is_the_one_found() {
        let base = std::env::temp_dir().join("hydebar-ipc-test");
        let session = base.join("hypr").join("live_session");
        std::fs::create_dir_all(&session).expect("a directory to hold the socket");
        let socket = session.join(REQUEST_SOCKET);
        std::fs::write(&socket, b"").expect("a file standing in for the socket");
        let events = session.join(EVENT_SOCKET);
        std::fs::write(&events, b"").expect("a file standing in for the event socket");

        let found = locate("live_session", base.to_str(), REQUEST_SOCKET);
        let found_events = locate("live_session", base.to_str(), EVENT_SOCKET);

        std::fs::remove_dir_all(&base).ok();
        assert_eq!(found, Some(socket));
        assert_eq!(found_events, Some(events));
    }
}
