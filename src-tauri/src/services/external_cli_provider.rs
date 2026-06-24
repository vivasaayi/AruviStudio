use crate::error::AppError;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExternalCliProvider {
    Codex,
    Claude,
    Cursor,
    Copilot,
}

impl ExternalCliProvider {
    pub(crate) fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude" => Ok(Self::Claude),
            "cursor" => Ok(Self::Cursor),
            "copilot" => Ok(Self::Copilot),
            _ => Err(AppError::Validation(format!(
                "Unsupported external CLI provider: {value}"
            ))),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex CLI",
            Self::Claude => "Claude Code CLI",
            Self::Cursor => "Cursor Agent CLI",
            Self::Copilot => "GitHub Copilot CLI",
        }
    }

    pub(crate) fn command_spec(self, prompt: &str, cwd: &str) -> (&'static str, Vec<String>) {
        match self {
            Self::Codex => (
                "codex",
                vec![
                    "exec".to_string(),
                    "--ignore-user-config".to_string(),
                    "--sandbox".to_string(),
                    "workspace-write".to_string(),
                    "--cd".to_string(),
                    cwd.to_string(),
                    prompt.to_string(),
                ],
            ),
            Self::Claude => ("claude", vec!["-p".to_string(), prompt.to_string()]),
            Self::Cursor => ("cursor-agent", vec!["-p".to_string(), prompt.to_string()]),
            Self::Copilot => (
                "copilot",
                vec![
                    "--autopilot".to_string(),
                    "--no-ask-user".to_string(),
                    "--max-autopilot-continues".to_string(),
                    "10".to_string(),
                    "-s".to_string(),
                    "--allow-tool".to_string(),
                    "shell(git:*),shell(npm:*),shell(cargo:*),shell(rg:*),shell(ls:*),shell(cat:*),shell(sed:*),write".to_string(),
                    "-p".to_string(),
                    prompt.to_string(),
                ],
            ),
        }
    }
}
