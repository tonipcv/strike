use strike_security::cli::completions::{CompletionGenerator, Shell};

#[test]
fn test_bash_completion_generation() {
    let completion = CompletionGenerator::generate(Shell::Bash).unwrap();
    
    assert!(completion.contains("_strike_completions"));
    assert!(completion.contains("COMPREPLY"));
    assert!(completion.contains("complete -F _strike_completions strike"));
}

#[test]
fn test_bash_completion_has_commands() {
    let completion = CompletionGenerator::generate(Shell::Bash).unwrap();
    
    assert!(completion.contains("scan"));
    assert!(completion.contains("report"));
    assert!(completion.contains("init"));
    assert!(completion.contains("config"));
    assert!(completion.contains("completions"));
}

#[test]
fn test_bash_completion_has_options() {
    let completion = CompletionGenerator::generate(Shell::Bash).unwrap();
    
    assert!(completion.contains("--target"));
    assert!(completion.contains("--output"));
    assert!(completion.contains("--format"));
    assert!(completion.contains("--severity"));
}

#[test]
fn test_zsh_completion_generation() {
    let completion = CompletionGenerator::generate(Shell::Zsh).unwrap();
    
    assert!(completion.contains("#compdef strike"));
    assert!(completion.contains("_strike"));
    assert!(completion.contains("_arguments"));
}

#[test]
fn test_zsh_completion_has_descriptions() {
    let completion = CompletionGenerator::generate(Shell::Zsh).unwrap();
    
    assert!(completion.contains("Run security scan"));
    assert!(completion.contains("Generate security report"));
}

#[test]
fn test_fish_completion_generation() {
    let completion = CompletionGenerator::generate(Shell::Fish).unwrap();
    
    assert!(completion.contains("complete -c strike"));
    assert!(completion.contains("__fish_use_subcommand"));
}

#[test]
fn test_fish_completion_has_subcommands() {
    let completion = CompletionGenerator::generate(Shell::Fish).unwrap();
    
    assert!(completion.contains("-a \"scan\""));
    assert!(completion.contains("-a \"report\""));
    assert!(completion.contains("-a \"completions\""));
}

#[test]
fn test_powershell_completion_generation() {
    let completion = CompletionGenerator::generate(Shell::PowerShell).unwrap();
    
    assert!(completion.contains("Register-ArgumentCompleter"));
    assert!(completion.contains("-CommandName strike"));
}

#[test]
fn test_shell_from_str_bash() {
    assert!(matches!(Shell::from_str("bash"), Some(Shell::Bash)));
    assert!(matches!(Shell::from_str("BASH"), Some(Shell::Bash)));
}

#[test]
fn test_shell_from_str_zsh() {
    assert!(matches!(Shell::from_str("zsh"), Some(Shell::Zsh)));
    assert!(matches!(Shell::from_str("ZSH"), Some(Shell::Zsh)));
}

#[test]
fn test_shell_from_str_fish() {
    assert!(matches!(Shell::from_str("fish"), Some(Shell::Fish)));
    assert!(matches!(Shell::from_str("FISH"), Some(Shell::Fish)));
}

#[test]
fn test_shell_from_str_powershell() {
    assert!(matches!(Shell::from_str("powershell"), Some(Shell::PowerShell)));
    assert!(matches!(Shell::from_str("pwsh"), Some(Shell::PowerShell)));
}

#[test]
fn test_shell_from_str_invalid() {
    assert!(Shell::from_str("invalid").is_none());
    assert!(Shell::from_str("").is_none());
    assert!(Shell::from_str("cmd").is_none());
}

#[test]
fn test_shell_as_str() {
    assert_eq!(Shell::Bash.as_str(), "bash");
    assert_eq!(Shell::Zsh.as_str(), "zsh");
    assert_eq!(Shell::Fish.as_str(), "fish");
    assert_eq!(Shell::PowerShell.as_str(), "powershell");
}

#[test]
fn test_all_shells_generate_successfully() {
    assert!(CompletionGenerator::generate(Shell::Bash).is_ok());
    assert!(CompletionGenerator::generate(Shell::Zsh).is_ok());
    assert!(CompletionGenerator::generate(Shell::Fish).is_ok());
    assert!(CompletionGenerator::generate(Shell::PowerShell).is_ok());
}

#[test]
fn test_bash_completion_format_options() {
    let completion = CompletionGenerator::generate(Shell::Bash).unwrap();
    
    assert!(completion.contains("json"));
    assert!(completion.contains("html"));
    assert!(completion.contains("pdf"));
    assert!(completion.contains("sarif"));
    assert!(completion.contains("markdown"));
}

#[test]
fn test_bash_completion_severity_options() {
    let completion = CompletionGenerator::generate(Shell::Bash).unwrap();
    
    assert!(completion.contains("critical"));
    assert!(completion.contains("high"));
    assert!(completion.contains("medium"));
    assert!(completion.contains("low"));
}

#[test]
fn test_completion_write_to_buffer() {
    let mut buffer = Vec::new();
    CompletionGenerator::write_to(Shell::Bash, &mut buffer).unwrap();
    
    let output = String::from_utf8(buffer).unwrap();
    assert!(output.contains("_strike_completions"));
}

#[test]
fn test_zsh_completion_has_format_options() {
    let completion = CompletionGenerator::generate(Shell::Zsh).unwrap();
    
    assert!(completion.contains("json html pdf sarif markdown"));
}

#[test]
fn test_fish_completion_has_global_options() {
    let completion = CompletionGenerator::generate(Shell::Fish).unwrap();
    
    assert!(completion.contains("--help"));
    assert!(completion.contains("--version"));
    assert!(completion.contains("--verbose"));
    assert!(completion.contains("--quiet"));
}
