//! Tests of the updates module state.

use std::{
    fs,
    num::NonZeroUsize,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH}
};

use tokio::runtime::Runtime;

use super::{
    failures::{FAILURE_REPEAT, FailureLog},
    hyde_clone::clone_path_from,
    schedule::{MIN_INTERVAL, check_interval},
    *
};
use crate::{
    ModuleContext,
    components::icons::IconTheme,
    config::UpdatesModuleConfig,
    event_bus::{EventBus, ModuleEvent},
    modules::Module
};

fn context(runtime: &Runtime) -> (EventBus, ModuleContext) {
    let bus = EventBus::new(NonZeroUsize::new(16).expect("capacity"));
    let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());

    (bus, ctx)
}

/// Waits for the aborted task behind `handle` to actually finish.
///
/// An abort is a request, not an event: the task ends when the runtime next
/// polls it. A fixed sleep is long enough on an idle machine and too short
/// under a loaded one — instrumented coverage runs are exactly that — so the
/// wait is on the condition, with a deadline generous enough that a genuine
/// hang still fails the test.
fn wait_until_finished(runtime: &Runtime, handle: &tokio::task::AbortHandle) {
    runtime.block_on(async {
        for _ in 0..600 {
            if handle.is_finished() {
                return;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
}

fn config(check: &str, interval: u64) -> UpdatesModuleConfig {
    UpdatesModuleConfig {
        check_cmd:      check.to_owned(),
        update_cmd:     ":".to_owned(),
        check_interval: interval,
        hyde_branch:    hydebar_proto::config::HydeBranch::default()
    }
}

fn task_id(updates: &Updates) -> tokio::task::Id {
    updates
        .schedule
        .as_ref()
        .expect("a running schedule")
        .handle
        .id()
}

#[test]
fn a_reload_that_changes_nothing_keeps_the_running_check() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();
    let definition = config("sleep 30", 3600);

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
        .expect("the first registration succeeds");
    let first = task_id(&updates);

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
        .expect("the second registration succeeds");

    assert_eq!(first, task_id(&updates));
}

/// The update command may be edited without touching the check, and
/// that alone must not disturb a check in flight.
#[test]
fn a_reload_that_only_changes_the_update_command_keeps_the_check() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();
    let mut definition = config("sleep 30", 3600);

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
        .expect("the first registration succeeds");
    let first = task_id(&updates);

    definition.update_cmd = "pacman -Syu".to_owned();
    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
        .expect("the second registration succeeds");

    assert_eq!(first, task_id(&updates));
    assert_eq!(
        updates
            .update_command
            .as_deref()
            .expect("a recorded update command"),
        "pacman -Syu"
    );
}

#[test]
fn a_changed_check_command_replaces_the_schedule() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 30", 3600)))
        .expect("the first registration succeeds");
    let first = task_id(&updates);

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 40", 3600)))
        .expect("the second registration succeeds");

    assert_ne!(first, task_id(&updates));
}

/// Picking the other branch must restart the check, or the bar would
/// keep measuring the clone against the line it just left.
#[test]
fn a_changed_branch_replaces_the_schedule() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 30", 3600)))
        .expect("the first registration succeeds");
    let first = task_id(&updates);

    let mut definition = config("sleep 30", 3600);
    definition.hyde_branch = hydebar_proto::config::HydeBranch::Dev;
    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&definition))
        .expect("the second registration succeeds");

    assert_ne!(first, task_id(&updates));
    assert_eq!(updates.hyde_branch_name(), "dev");
}

#[test]
fn a_changed_interval_replaces_the_schedule() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 30", 3600)))
        .expect("the first registration succeeds");
    let first = task_id(&updates);

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 30", 1800)))
        .expect("the second registration succeeds");

    assert_ne!(first, task_id(&updates));
}

#[test]
fn a_replaced_schedule_ends_the_task_it_replaced() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 30", 3600)))
        .expect("the first registration succeeds");
    let first = updates
        .schedule
        .as_ref()
        .expect("a running schedule")
        .handle
        .abort_handle();

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 40", 3600)))
        .expect("the second registration succeeds");

    wait_until_finished(&runtime, &first);

    assert!(first.is_finished());
}

#[test]
fn a_module_without_a_configuration_schedules_nothing() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();

    <Updates as Module<Message>>::register(&mut updates, &ctx, None)
        .expect("registration succeeds without a configuration");

    assert!(updates.schedule.is_none());
    assert!(updates.update_command.is_none());
}

#[test]
fn losing_the_configuration_ends_the_running_schedule() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 30", 3600)))
        .expect("the first registration succeeds");
    let handle = updates
        .schedule
        .as_ref()
        .expect("a running schedule")
        .handle
        .abort_handle();

    <Updates as Module<Message>>::register(&mut updates, &ctx, None)
        .expect("registration succeeds without a configuration");

    wait_until_finished(&runtime, &handle);

    assert!(updates.schedule.is_none());
    assert!(handle.is_finished());
}

#[test]
fn deregistering_ends_the_schedule() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let mut updates = Updates::default();

    <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config("sleep 30", 3600)))
        .expect("registration succeeds");
    let handle = updates
        .schedule
        .as_ref()
        .expect("a running schedule")
        .handle
        .abort_handle();

    <Updates as Module<Message>>::deregister(&mut updates);

    wait_until_finished(&runtime, &handle);

    assert!(updates.schedule.is_none());
    assert!(handle.is_finished());
}

/// The runner is single: a check that takes longer than the interval
/// delays the next one instead of running beside it.
#[test]
fn two_checks_never_run_at_once() {
    let runtime = Runtime::new().expect("runtime");
    let (_bus, ctx) = context(&runtime);
    let trace = trace_path();
    let _ = fs::remove_file(&trace);

    let command = format!(
        "printf 'in\\n' >> {trace}; sleep 0.2; printf 'out\\n' >> {trace}",
        trace = trace.display()
    );

    let schedule = Schedule::start(
        ctx.runtime_handle(),
        ctx.module_sender(ModuleEvent::Updates),
        Arc::from(command.as_str()),
        Duration::from_millis(10),
        None,
        Arc::from("master")
    );

    runtime.block_on(async {
        for _ in 0..20 {
            schedule.request_check();
            tokio::time::sleep(Duration::from_millis(45)).await;
        }
    });

    drop(schedule);

    let recorded = fs::read_to_string(&trace).expect("the checks left a trace");
    let _ = fs::remove_file(&trace);

    let mut inside = false;
    let mut runs = 0;

    for line in recorded.lines() {
        match line {
            "in" => {
                assert!(!inside, "a check started while another was running");
                inside = true;
                runs += 1;
            }
            "out" => {
                assert!(inside, "a check ended without having started");
                inside = false;
            }
            other => panic!("unexpected trace line {other:?}")
        }
    }

    assert!(runs >= 2, "the schedule ran {runs} times, expected repeats");
}

fn trace_path() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());

    std::env::temp_dir().join(format!("hydebar-updates-{}-{stamp}", std::process::id()))
}

#[test]
fn the_interval_never_drops_below_the_floor() {
    assert_eq!(check_interval(&config("true", 0)), MIN_INTERVAL);
    assert_eq!(check_interval(&config("true", 1)), MIN_INTERVAL);
    assert_eq!(
        check_interval(&config("true", 7200)),
        Duration::from_hours(2)
    );
}

#[test]
fn the_first_failure_of_its_kind_is_always_reported() {
    let mut failures = FailureLog::default();

    assert_eq!(failures.record("mirror is down"), Some(1));
    assert_eq!(failures.record("database is locked"), Some(1));
}

#[test]
fn the_same_failure_is_reported_once_in_a_while() {
    let mut failures = FailureLog::default();

    assert_eq!(failures.record("mirror is down"), Some(1));

    for _ in 2..FAILURE_REPEAT {
        assert_eq!(failures.record("mirror is down"), None);
    }

    assert_eq!(failures.record("mirror is down"), Some(FAILURE_REPEAT));
}

#[test]
fn a_check_that_works_forgets_the_failure_before_it() {
    let mut failures = FailureLog::default();

    assert_eq!(failures.record("mirror is down"), Some(1));
    failures.clear();

    assert_eq!(failures.record("mirror is down"), Some(1));
}

#[test]
fn an_unavailable_check_takes_the_indicator_off_the_bar() {
    let updates = Updates {
        state: CheckState::Unavailable,
        ..Updates::default()
    };
    let icons = IconTheme::default();
    let config = Some(config("true", 3600));

    assert!(updates.bar_view::<Message>(&config, &icons).is_none());
}

#[test]
fn a_failed_check_keeps_what_the_bar_already_knows() {
    let mut updates = Updates {
        pending: vec![Update {
            package: "pkg".to_owned(),
            from:    "1".to_owned(),
            to:      "2".to_owned()
        }],
        state: CheckState::Checking,
        ..Updates::default()
    };

    updates.observe(Message::CheckFailed);

    assert_eq!(updates.pending.len(), 1);
    assert_eq!(updates.state, CheckState::Ready);
}

#[test]
fn a_stale_hyde_clone_reaches_the_menu() {
    let mut updates = Updates::default();

    updates.observe(Message::HydeChecked(HydeSnapshot {
        version: "v25.10.1".to_owned(),
        commits: vec!["fix: one".to_owned()]
    }));

    assert_eq!(updates.hyde_pending(), 1);
}

#[test]
fn collapsing_folds_both_lists_shut() {
    let mut updates = Updates {
        is_updates_list_open: true,
        ..Updates::default()
    };
    updates.observe(Message::ToggleHydeList);

    updates.collapse();

    assert!(!updates.is_updates_list_open);
    assert!(!updates.is_hyde_list_open);
}

#[test]
fn the_tooltip_names_a_hyde_clone_that_fell_behind() {
    let mut updates = Updates {
        state: CheckState::Ready,
        ..Updates::default()
    };
    updates.observe(Message::HydeChecked(HydeSnapshot {
        version: "v25.10.1".to_owned(),
        commits: vec!["fix: one".to_owned(), "feat: two".to_owned()]
    }));

    assert_eq!(
        updates.tooltip().expect("a tooltip"),
        "Updates: none pending · HyDE: 2 commits behind"
    );
}

#[test]
fn the_clone_path_is_read_from_the_version_file() {
    let content = "HYDE_BRANCH='master'\nHYDE_CLONE_PATH='/home/user/HyDE'\n";

    assert_eq!(
        clone_path_from(content),
        Some(std::path::PathBuf::from("/home/user/HyDE"))
    );
    assert_eq!(clone_path_from("HYDE_CLONE_PATH=''\n"), None);
    assert_eq!(clone_path_from("nothing here"), None);
}

#[test]
fn an_unavailable_check_clears_what_the_bar_knew() {
    let mut updates = Updates {
        pending: vec![Update {
            package: "pkg".to_owned(),
            from:    "1".to_owned(),
            to:      "2".to_owned()
        }],
        ..Updates::default()
    };

    updates.observe(Message::UpdatesUnavailable);

    assert!(updates.pending.is_empty());
    assert_eq!(updates.state, CheckState::Unavailable);
}
