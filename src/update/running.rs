use std::path::Path;

pub(crate) fn another_copy_runs(exe: &Path) -> bool {
    let this = std::process::id();
    let exe = exe.to_string_lossy().to_lowercase();
    processes::running()
        .into_iter()
        .any(|(id, path)| id != this && path.to_lowercase() == exe)
}

#[cfg(windows)]
mod processes {
    use windows::{
        Win32::{
            Foundation::CloseHandle,
            System::{
                ProcessStatus::EnumProcesses,
                Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
            },
        },
        core::PWSTR,
    };

    const FIRST_GUESS: usize = 1024;
    const LONGEST_PATH: usize = 32 * 1024;

    pub(super) fn running() -> Vec<(u32, String)> {
        ids().into_iter().filter_map(|id| Some((id, path_of(id)?))).collect()
    }

    fn ids() -> Vec<u32> {
        let mut ids = vec![0u32; FIRST_GUESS];
        loop {
            let offered = (ids.len() * size_of::<u32>()) as u32;
            let mut filled = 0u32;
            if unsafe { EnumProcesses(ids.as_mut_ptr(), offered, &mut filled) }.is_err() {
                return Vec::new();
            }
            if filled < offered {
                ids.truncate(filled as usize / size_of::<u32>());
                return ids;
            }
            ids.resize(ids.len() * 2, 0);
        }
    }

    fn path_of(id: u32) -> Option<String> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, id) }.ok()?;
        let mut path = vec![0u16; LONGEST_PATH];
        let mut length = path.len() as u32;
        let found = unsafe { QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(path.as_mut_ptr()), &mut length) };
        let _ = unsafe { CloseHandle(process) };
        found.ok()?;
        Some(String::from_utf16_lossy(&path[..length as usize]))
    }
}

#[cfg(not(windows))]
mod processes {
    pub(super) fn running() -> Vec<(u32, String)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::Path,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    use super::another_copy_runs;

    #[test]
    #[ignore = "another test starts it as a second copy of this exe"]
    fn stays_running_for_a_few_seconds() {
        std::thread::sleep(Duration::from_secs(5));
    }

    #[cfg(windows)]
    #[test]
    fn a_second_process_running_the_same_exe_is_found() {
        let this_exe = std::env::current_exe().unwrap();
        assert!(!another_copy_runs(&this_exe));
        let mut twin = Command::new(&this_exe)
            .args(["--ignored", "--exact", "update::running::tests::stays_running_for_a_few_seconds"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let started = Instant::now();
        let mut seen = false;
        while !seen && started.elapsed() < Duration::from_secs(4) {
            seen = another_copy_runs(&this_exe);
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = twin.kill();
        let _ = twin.wait();
        assert!(seen);
    }

    #[test]
    fn nothing_runs_from_a_folder_that_does_not_exist() {
        assert!(!another_copy_runs(Path::new(r"C:\nowhere\sternegit.exe")));
    }
}
