use std::env;

pub(crate) fn has_command(command: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| {
            let candidate = directory.join(command);
            if cfg!(windows) {
                candidate.is_file() || directory.join(format!("{command}.exe")).is_file()
            } else {
                candidate.is_file()
            }
        })
    })
}
