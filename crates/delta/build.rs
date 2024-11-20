use vergen::Emitter;

use std::process::Command;

fn main() {
    if let Ok(output) = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .output()
    {
        if let Ok(git_origin) = String::from_utf8(output.stdout) {
            println!("cargo:rustc-env=GIT_ORIGIN_URL={git_origin}");
        }
    }

    let _ = Emitter::default().emit();
}
