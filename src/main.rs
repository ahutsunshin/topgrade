extern crate failure;
extern crate os_type;
extern crate which;
#[macro_use]
extern crate failure_derive;
extern crate termion;

mod git;
mod report;
mod steps;
mod terminal;

use failure::Error;
use git::Git;
use os_type::OSType;
use report::{Report, Reporter};
use std::collections::HashSet;
use std::env::home_dir;
use std::path::PathBuf;
use std::process::ExitStatus;
use steps::*;
use terminal::Terminal;
use which::which;

#[derive(Fail, Debug)]
#[fail(display = "Process failed")]
struct ProcessFailed;

trait Check {
    fn check(self) -> Result<(), Error>;
}

impl Check for ExitStatus {
    fn check(self) -> Result<(), Error> {
        if self.success() {
            Ok(())
        } else {
            Err(Error::from(ProcessFailed {}))
        }
    }
}

fn home_path(p: &str) -> PathBuf {
    let mut path = home_dir().unwrap();
    path.push(p);
    path
}

#[cfg(unix)]
fn tpm() -> Option<PathBuf> {
    let mut path = home_dir().unwrap();
    path.push(".tmux/plugins/tpm/bin/update_plugins");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn main() -> Result<(), Error> {
    let git = Git::new();
    let mut git_repos: HashSet<String> = HashSet::new();
    let terminal = Terminal::new();
    let mut reports = Report::new();

    {
        let mut collect_repo = |path| {
            if let Some(repo) = git.get_repo_root(path) {
                git_repos.insert(repo);
            }
        };

        collect_repo(home_path(".emacs.d"));

        if cfg!(unix) {
            collect_repo(home_path(".zshrc"));
            collect_repo(home_path(".oh-my-zsh"));
            collect_repo(home_path(".tmux"));
        }
    }

    for repo in git_repos {
        terminal.print_separator(format!("Pulling {}", repo));
        if let Some(success) = git.pull(&repo)? {
            success.report(format!("git: {}", repo), &mut reports);
        }
    }

    if cfg!(unix) {
        if let Ok(zsh) = which("zsh") {
            if home_path(".zplug").exists() {
                terminal.print_separator("zplug");
                run_zplug(&zsh).report("zplug", &mut reports);
            }
        }

        if let Some(tpm) = tpm() {
            terminal.print_separator("tmux plugins");
            run_tpm(&tpm).report("tmux", &mut reports);
        }
    }

    let cargo_upgrade = home_path(".cargo/bin/cargo-install-update");
    if cargo_upgrade.exists() {
        terminal.print_separator("Cargo");
        run_cargo_update(&cargo_upgrade).report("Cargo", &mut reports);
    }

    if let Ok(emacs) = which("emacs") {
        let init_file = home_path(".emacs.d/init.el");
        if init_file.exists() {
            terminal.print_separator("Emacs");
            run_emacs(&emacs, &home_path(".emacs.d/init.el")).report("Emacs", &mut reports);
        }
    }

    if let Ok(gem) = which("gem") {
        terminal.print_separator("RubyGems");
        run_gem(&gem).report("RubyGems", &mut reports);
    }

    if let Ok(npm) = which("npm") {
        terminal.print_separator("Node Package Manager");
        run_npm(&npm).report("Node Package Manager", &mut reports);
    }

    if let Ok(apm) = which("apm") {
        terminal.print_separator("Atom Package Manager");
        run_apm(&apm).report("Atom Package Manager", &mut reports);
    }

    if cfg!(target_os = "linux") {
        let sudo = which("sudo");

        terminal.print_separator("System update");
        match os_type::current_platform().os_type {
            OSType::Arch => Some(upgrade_arch_linux(&sudo, &terminal)),
            OSType::CentOS | OSType::Redhat => Some(upgrade_redhat(&sudo, &terminal)),
            OSType::Ubuntu | OSType::Debian => Some(upgrade_debian(&sudo, &terminal)),
            OSType::Unknown => {
                terminal.print_warning(
                    "Could not detect your Linux distribution. Do you have lsb-release installed?",
                );

                None
            }

            _ => None,
        }.report("System upgrade", &mut reports);

        if let Ok(fwupdmgr) = which("fwupdmgr") {
            terminal.print_separator("Firmware upgrades");
            run_fwupdmgr(&fwupdmgr).report("Firmware upgrade", &mut reports);
        }

        if let Ok(sudo) = &sudo {
            if let Ok(_) = which("needrestart") {
                terminal.print_separator("Check for needed restarts");
                run_needrestart(&sudo).report("Restarts", &mut reports);
            }
        }
    }

    if cfg!(target_os = "macos") {
        if let Ok(brew) = which("brew") {
            terminal.print_separator("Homebrew");
            run_homebrew(&brew).report("Homebrew", &mut reports);
        }

        terminal.print_separator("System update");
        upgrade_macos().report("System upgrade", &mut reports);;
    }

    let mut reports: Vec<_> = reports.into_iter().collect();
    reports.sort();

    if !reports.is_empty() {
        terminal.print_separator("Summary");

        for (key, succeeded) in reports {
            terminal.print_result(key, succeeded);
        }
    }

    Ok(())
}
