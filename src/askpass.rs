//! Headless `SSH_ASKPASS` helper mode.
//!
//! `ssh` invokes us as `sshelf "<prompt>"` (with `SSHELF_ASKPASS=1` in the environment). We
//! answer **only** password prompts — fetching the secret for `SSHELF_HOST_ID` — and decline
//! everything else (host-key `yes/no`, key passphrases, OTP) by exiting non-zero, so ssh
//! handles those itself. This split is mandatory because `SSH_ASKPASS_REQUIRE=force` routes
//! *every* prompt here (proven by the M0 spike — see docs/ssh-command.md).

use crate::paths::Paths;
use crate::secrets;

const HOST_ID_ENV: &str = "SSHELF_HOST_ID";

/// Run askpass mode for the given prompt; returns the process exit code.
pub fn run(prompt: &str) -> i32 {
    if !is_secret_prompt(prompt) {
        return 1; // decline; let ssh handle it
    }
    let Ok(id) = std::env::var(HOST_ID_ENV) else {
        return 1;
    };
    if id.is_empty() {
        return 1;
    }
    let Ok(paths) = Paths::resolve() else {
        return 1;
    };
    match secrets::get_password(&paths.vault_file(), &id) {
        Ok(Some(pw)) => {
            let pw = zeroize::Zeroizing::new(pw);
            // ssh reads one line and strips the trailing newline.
            println!("{}", pw.as_str());
            0
        }
        _ => 1,
    }
}

/// True if the prompt is asking for the host's stored secret — a login **password** or a key
/// **passphrase**.
///
/// We match the *shape* of OpenSSH's standard prompts (not just the substring), so a
/// compromised server can't phish the stored secret with a keyboard-interactive prompt like
/// "Type your password to continue:". Recognized prompts:
///
/// - classic password auth `user@host's password:` and PAM `Password:` → end with "password:"
/// - key passphrase `Enter passphrase for key '<path>':` → contains "passphrase for"
///
/// Host-key confirmations, OTP/verification codes, and arbitrary server text are declined.
fn is_secret_prompt(prompt: &str) -> bool {
    let p = prompt.trim().to_lowercase();
    p.ends_with("password:") || p.contains("passphrase for")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_standard_password_and_passphrase_prompts() {
        assert!(is_secret_prompt("tester@host's password: "));
        assert!(is_secret_prompt("Password:"));
        assert!(is_secret_prompt(
            "Enter passphrase for key '/home/u/.ssh/id_ed25519': "
        ));
    }

    #[test]
    fn declines_host_key_otp_and_phishing_prompts() {
        assert!(!is_secret_prompt(
            "Are you sure you want to continue connecting (yes/no/[fingerprint])? "
        ));
        assert!(!is_secret_prompt("Verification code: "));
        // Server-controlled keyboard-interactive prompts that merely mention the word:
        assert!(!is_secret_prompt(
            "Please confirm your password for this operation:"
        ));
        assert!(!is_secret_prompt("Type your password to continue:"));
    }
}
