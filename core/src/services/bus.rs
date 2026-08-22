//! Saying what kind of failure a bus call came back with.
//!
//! Every service in this crate talks to the session or the system bus, and
//! every one of them used to answer the same way: an internal error carrying
//! the message the bus printed. A caller could not tell a device that has gone
//! from a request that was refused, so nothing could act on the difference —
//! not a retry loop, not a log line, not a reader of the journal.
//!
//! The peer already says which it is. A method reply carries the error name
//! the interface raised, and the transport failures carry their own shape, so
//! the classification is read off what arrived rather than guessed.

use std::fmt::Display;

use masterror::AppError;

/// The tail of an error name that means the caller was not allowed.
const REFUSALS: [&str; 5] = [
    "AccessDenied",
    "AuthFailed",
    "NotAuthorized",
    "PermissionDenied",
    "InteractiveAuthorizationRequired"
];

/// The tail of an error name that means the thing asked for is not there.
const ABSENCES: [&str; 7] = [
    "UnknownObject",
    "UnknownInterface",
    "UnknownMethod",
    "UnknownProperty",
    "ServiceUnknown",
    "NameHasNoOwner",
    "NotFound"
];

/// The tail of an error name that means the peer never answered in time.
const SILENCES: [&str; 3] = ["NoReply", "Timeout", "TimedOut"];

/// Reads the last segment of a D-Bus error name.
fn leaf(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// Turns a bus failure into an error that states what kind of failure it was.
///
/// `context` names the operation in the caller's own words; it is what the log
/// line reads as, with the peer's message appended.
pub fn bus_failure(context: impl Display, err: &zbus::Error) -> AppError {
    let message = format!("{context}: {err}");

    match err {
        zbus::Error::MethodError(name, ..) => {
            let leaf = leaf(name.as_str());

            if REFUSALS.contains(&leaf) {
                AppError::unauthorized(message)
            } else if ABSENCES.contains(&leaf) {
                AppError::not_found(message)
            } else if SILENCES.contains(&leaf) {
                AppError::timeout(message)
            } else {
                AppError::service(message)
            }
        }
        zbus::Error::FDO(inner) => fdo_kind(inner, message),
        zbus::Error::InterfaceNotFound => AppError::not_found(message),
        zbus::Error::Address(_)
        | zbus::Error::Handshake(_)
        | zbus::Error::InputOutput(_)
        | zbus::Error::Connection(..) => AppError::service_unavailable(message),
        _ => AppError::service(message)
    }
}

/// Turns a failure raised by a standard bus interface into a kinded error.
///
/// `context` names the operation in the caller's own words.
pub fn fdo_failure(context: impl Display, err: &zbus::fdo::Error) -> AppError {
    fdo_kind(err, format!("{context}: {err}"))
}

/// The kind a standard bus failure stands for, under the message given.
fn fdo_kind(err: &zbus::fdo::Error, message: String) -> AppError {
    use zbus::fdo::Error as Fdo;

    match err {
        Fdo::AccessDenied(_) | Fdo::AuthFailed(_) | Fdo::InteractiveAuthorizationRequired(_) => {
            AppError::unauthorized(message)
        }
        Fdo::ServiceUnknown(_)
        | Fdo::NameHasNoOwner(_)
        | Fdo::UnknownMethod(_)
        | Fdo::UnknownObject(_)
        | Fdo::UnknownInterface(_)
        | Fdo::UnknownProperty(_)
        | Fdo::FileNotFound(_) => AppError::not_found(message),
        Fdo::NoReply(_) | Fdo::Timeout(_) | Fdo::TimedOut(_) => AppError::timeout(message),
        Fdo::Disconnected(_) | Fdo::NoServer(_) | Fdo::NoNetwork(_) | Fdo::BadAddress(_) => {
            AppError::service_unavailable(message)
        }
        Fdo::InvalidArgs(_) => AppError::bad_request(message),
        _ => AppError::service(message)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use masterror::AppErrorKind;

    use super::bus_failure;

    fn method_error(name: &str) -> zbus::Error {
        zbus::Error::MethodError(
            zbus::names::OwnedErrorName::try_from(name).expect("a well formed error name"),
            None,
            zbus::message::Message::method_call("/", "Nothing")
                .expect("a builder")
                .destination("org.example")
                .expect("a destination")
                .interface("org.example.Interface")
                .expect("an interface")
                .build(&())
                .expect("a message")
        )
    }

    #[test]
    fn a_refused_call_is_named_as_a_refusal() {
        let err = bus_failure(
            "join",
            &method_error("org.freedesktop.DBus.Error.AccessDenied")
        );

        assert_eq!(err.kind, AppErrorKind::Unauthorized);
    }

    #[test]
    fn a_call_to_something_gone_is_named_as_an_absence() {
        let err = bus_failure(
            "read",
            &method_error("org.freedesktop.DBus.Error.UnknownObject")
        );

        assert_eq!(err.kind, AppErrorKind::NotFound);
    }

    #[test]
    fn a_peer_that_never_answers_is_named_as_a_timeout() {
        let err = bus_failure("ask", &method_error("org.freedesktop.DBus.Error.NoReply"));

        assert_eq!(err.kind, AppErrorKind::Timeout);
    }

    #[test]
    fn a_bus_that_cannot_be_reached_is_named_as_unavailable() {
        let err = bus_failure("connect", &zbus::Error::Address(String::from("nowhere")));

        assert_eq!(err.kind, AppErrorKind::DependencyUnavailable);
    }

    #[test]
    fn a_standard_refusal_is_named_as_a_refusal() {
        let err = super::fdo_failure(
            "register",
            &zbus::fdo::Error::AccessDenied(String::from("no"))
        );

        assert_eq!(err.kind, AppErrorKind::Unauthorized);
    }

    #[test]
    fn a_standard_absence_is_named_as_an_absence() {
        let err = super::fdo_failure(
            "read",
            &zbus::fdo::Error::UnknownObject(String::from("gone"))
        );

        assert_eq!(err.kind, AppErrorKind::NotFound);
    }

    #[test]
    fn anything_else_is_named_as_a_failing_service() {
        let err = bus_failure("call", &method_error("net.connman.iwd.Failed"));

        assert_eq!(err.kind, AppErrorKind::Service);
    }

    #[test]
    fn the_context_and_the_peers_words_both_survive() {
        let err = bus_failure("join the network", &zbus::Error::InvalidReply);

        assert!(
            err.message
                .as_deref()
                .is_some_and(|m| m.contains("join the network"))
        );
    }
}
