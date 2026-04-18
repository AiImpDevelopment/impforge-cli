// SPDX-License-Identifier: MIT
//! Allowlist — which CLI commands the remote bridge can forward.

pub const ALLOWED_COMMANDS: &[&str] = &[
    "template list",
    "template show",
    "compliance",
    "skill list",
    "skill show",
    "mcp list",
    "mcp clients",
    "doctor",
    "introspect",
    "brain status",
    "brain chat",
    "model list",
    "model ping",
    "upgrade",
];

pub const BLOCKED_COMMANDS: &[&str] = &[
    "template scaffold",
    "mcp serve",
    "autopilot",
    "crown-jewel gate",
    "export-config",
    "brain pull",
    "model pull",
];

pub fn is_command_allowed(command: &str) -> bool {
    let trimmed = command.trim();
    for blocked in BLOCKED_COMMANDS {
        if trimmed.starts_with(blocked) {
            return false;
        }
    }
    for allowed in ALLOWED_COMMANDS {
        if trimmed.starts_with(allowed) {
            return true;
        }
    }
    false
}

pub fn upgrade_message_for_blocked(command: &str) -> String {
    format!(
        "`{}` is not available via the remote bridge in impforge-cli (free).\n\nUpgrade to impforge-aiimp Pro for full remote dispatch: https://impforge.com/pro",
        command.trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_template_list() {
        assert!(is_command_allowed("template list"));
        assert!(is_command_allowed("template list --verbose"));
    }

    #[test]
    fn block_scaffold() {
        assert!(!is_command_allowed("template scaffold fintech-saas /tmp/x"));
    }

    #[test]
    fn block_takes_priority_over_allow() {
        assert!(!is_command_allowed("template scaffold"));
    }

    #[test]
    fn unknown_command_rejected() {
        assert!(!is_command_allowed("rm -rf /"));
        assert!(!is_command_allowed("sudo poweroff"));
    }

    #[test]
    fn upgrade_message_links_to_pro() {
        let m = upgrade_message_for_blocked("template scaffold");
        assert!(m.contains("impforge.com/pro"));
    }

    #[test]
    fn allow_doctor_without_args() {
        assert!(is_command_allowed("doctor"));
    }
}
