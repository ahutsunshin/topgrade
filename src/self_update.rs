use super::error::{Error, ErrorKind};
use super::terminal::*;
use failure::ResultExt;
use self_update_crate;
#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(unix)]
use std::process::Command;

pub fn self_update() -> Result<(), Error> {
    print_separator("Self update");
    #[cfg(unix)]
    let current_exe = env::current_exe();

    let target = self_update_crate::get_target().context(ErrorKind::SelfUpdate)?;
    let result = self_update_crate::backends::github::Update::configure()
        .context(ErrorKind::SelfUpdate)?
        .repo_owner("r-darwish")
        .repo_name("topgrade")
        .target(&target)
        .bin_name(if cfg!(windows) { "topgrade.exe" } else { "topgrade" })
        .show_output(false)
        .show_download_progress(true)
        .current_version(self_update_crate::cargo_crate_version!())
        .no_confirm(true)
        .build()
        .context(ErrorKind::SelfUpdate)?
        .update()
        .context(ErrorKind::SelfUpdate)?;

    if let self_update_crate::Status::Updated(version) = &result {
        println!("\nTopgrade upgraded to {}", version);
    } else {
        println!("Topgrade is up-to-date");
    }

    #[cfg(unix)]
    {
        if result.updated() {
            print_warning("Respawning...");
            let err = Command::new(current_exe.context(ErrorKind::SelfUpdate)?)
                .args(env::args().skip(1))
                .exec();
            Err(err).context(ErrorKind::SelfUpdate)?
        }
    }

    Ok(())
}
