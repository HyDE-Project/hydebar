//! Burial of adopted children nobody is waiting for.

/// How often the reaper looks for adopted children nobody buried.
const REAP_PERIOD: std::time::Duration = std::time::Duration::from_secs(60);

/// Buries the adopted children nobody is waiting for.
///
/// Claiming orphans makes the bar the parent of every helper a listener
/// leaves behind — including the ones that exit on their own, and a dead
/// child stays a zombie until its parent buries it. The bar's own spawns all
/// have a waiter and are gone within moments; an adopted stray has none by
/// definition. The reaper therefore only buries a child seen dead on two
/// looks in a row: anything with a live waiter never survives even one.
pub fn start_orphan_reaper() {
    std::thread::Builder::new()
        .name(String::from("hydebar-reaper"))
        .spawn(|| {
            let mut seen: std::collections::HashSet<i32> = std::collections::HashSet::new();

            loop {
                std::thread::sleep(REAP_PERIOD);

                let dead = zombie_children();

                for pid in dead.intersection(&seen) {
                    unsafe {
                        libc::waitpid(*pid, std::ptr::null_mut(), libc::WNOHANG);
                    }
                }

                seen = dead;
            }
        })
        .ok();
}

/// The children of this process that already exited.
///
/// The kernel lists a task's children in one file per thread; reading those
/// costs a handful of syscalls where the old walk paid one stat per process
/// on the machine, every period, forever. The walk remains as the fallback
/// for a kernel built without that file.
fn zombie_children() -> std::collections::HashSet<i32> {
    match children_of_self() {
        Some(children) => children
            .into_iter()
            .filter(|pid| {
                std::fs::read_to_string(format!("/proc/{pid}/stat"))
                    .ok()
                    .and_then(|stat| {
                        Some(stat.rsplit(')').next()?.split_whitespace().next()? == "Z")
                    })
                    .unwrap_or(false)
            })
            .collect(),
        None => zombie_children_by_scan()
    }
}

/// Every child pid the kernel lists for this process, across its threads.
fn children_of_self() -> Option<std::collections::HashSet<i32>> {
    let tasks = std::fs::read_dir("/proc/self/task").ok()?;
    let mut children = std::collections::HashSet::new();
    let mut listed = false;

    for task in tasks.flatten() {
        let Ok(list) = std::fs::read_to_string(task.path().join("children")) else {
            continue;
        };

        listed = true;
        children.extend(
            list.split_whitespace()
                .filter_map(|pid| pid.parse::<i32>().ok())
        );
    }

    listed.then_some(children)
}

/// The children of this process that already exited, by scanning `/proc`.
fn zombie_children_by_scan() -> std::collections::HashSet<i32> {
    let me = std::process::id() as i32;
    let mut zombies = std::collections::HashSet::new();

    let Ok(entries) = std::fs::read_dir("/proc") else {
        return zombies;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|name| name.parse::<i32>().ok()) else {
            continue;
        };

        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };

        let Some(tail) = stat.rsplit(')').next() else {
            continue;
        };
        let mut fields = tail.split_whitespace();
        let state = fields.next();
        let parent = fields.next().and_then(|ppid| ppid.parse::<i32>().ok());

        if state == Some("Z") && parent == Some(me) {
            zombies.insert(pid);
        }
    }

    zombies
}
