fn main() {
    println!("cargo:rustc-env=PANEL_SERIAL_NUMBER={}", get_git_commit_short(),);
}

fn get_git_commit() -> String {
    run_cmd("git", &["rev-parse", "HEAD"])
}

pub fn get_git_commit_short() -> String {
    let long = get_git_commit();

    if long.len() < 7 {
        panic!("Unexpected commit hash: '{}'", long);
    }

    let short = &long[0..7];
    short.to_string()
}

fn run_cmd(cmd: &str, args: &[&str]) -> String {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("error executing '{} {}': {}", cmd, args.join(" "), e))
        .expect("Command failed to run");

    let stdout = std::str::from_utf8(&output.stdout).expect("Couldn't convert stdout to UTF8");
    stdout.trim().to_string()
}
