use anyhow::Result;
use std::io::Write;

#[derive(Debug, Clone, Copy)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Shell {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            "powershell" | "pwsh" => Some(Shell::PowerShell),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::PowerShell => "powershell",
        }
    }
}

pub struct CompletionGenerator;

impl CompletionGenerator {
    pub fn generate(shell: Shell) -> Result<String> {
        Ok(match shell {
            Shell::Bash => Self::generate_bash(),
            Shell::Zsh => Self::generate_zsh(),
            Shell::Fish => Self::generate_fish(),
            Shell::PowerShell => Self::generate_powershell(),
        })
    }
    
    pub fn write_to<W: Write>(shell: Shell, writer: &mut W) -> Result<()> {
        let completion = Self::generate(shell)?;
        writer.write_all(completion.as_bytes())?;
        Ok(())
    }
    
    fn generate_bash() -> String {
        r#"# strike bash completion

_strike_completions() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Main commands
    local commands="scan report init config completions help version"
    
    # Global options
    local global_opts="--help --version --verbose --quiet --config"
    
    # Scan options
    local scan_opts="--target --output --format --severity --max-findings --timeout --workers --allow-private"
    
    # Report options
    local report_opts="--input --output --format --template"
    
    # Format options
    local formats="json html pdf sarif markdown"
    
    # Severity options
    local severities="critical high medium low info"
    
    case "${prev}" in
        scan)
            COMPREPLY=( $(compgen -W "${scan_opts}" -- ${cur}) )
            return 0
            ;;
        report)
            COMPREPLY=( $(compgen -W "${report_opts}" -- ${cur}) )
            return 0
            ;;
        --format)
            COMPREPLY=( $(compgen -W "${formats}" -- ${cur}) )
            return 0
            ;;
        --severity)
            COMPREPLY=( $(compgen -W "${severities}" -- ${cur}) )
            return 0
            ;;
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell" -- ${cur}) )
            return 0
            ;;
        *)
            ;;
    esac
    
    if [[ ${cur} == -* ]] ; then
        COMPREPLY=( $(compgen -W "${global_opts} ${scan_opts} ${report_opts}" -- ${cur}) )
        return 0
    fi
    
    COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
    return 0
}

complete -F _strike_completions strike
"#.to_string()
    }
    
    fn generate_zsh() -> String {
        r#"#compdef strike

_strike() {
    local -a commands
    commands=(
        'scan:Run security scan on target'
        'report:Generate security report'
        'init:Initialize Strike configuration'
        'config:Manage configuration'
        'completions:Generate shell completions'
        'help:Show help information'
        'version:Show version information'
    )
    
    local -a scan_opts
    scan_opts=(
        '--target[Target URL to scan]:url:'
        '--output[Output file path]:file:_files'
        '--format[Output format]:format:(json html pdf sarif markdown)'
        '--severity[Minimum severity level]:severity:(critical high medium low info)'
        '--max-findings[Maximum findings to report]:number:'
        '--timeout[Scan timeout in seconds]:seconds:'
        '--workers[Number of worker threads]:number:'
        '--allow-private[Allow scanning private IPs]'
        '--help[Show help]'
    )
    
    local -a report_opts
    report_opts=(
        '--input[Input file path]:file:_files'
        '--output[Output file path]:file:_files'
        '--format[Output format]:format:(json html pdf sarif markdown)'
        '--template[Report template]:template:_files'
        '--help[Show help]'
    )
    
    local -a global_opts
    global_opts=(
        '--help[Show help]'
        '--version[Show version]'
        '--verbose[Verbose output]'
        '--quiet[Quiet mode]'
        '--config[Config file path]:file:_files'
    )
    
    _arguments -C \
        '1: :->command' \
        '*:: :->args' \
        $global_opts
    
    case $state in
        command)
            _describe 'command' commands
            ;;
        args)
            case $words[1] in
                scan)
                    _arguments $scan_opts
                    ;;
                report)
                    _arguments $report_opts
                    ;;
                completions)
                    _arguments '1:shell:(bash zsh fish powershell)'
                    ;;
            esac
            ;;
    esac
}

_strike "$@"
"#.to_string()
    }
    
    fn generate_fish() -> String {
        r#"# strike fish completion

# Commands
complete -c strike -f -n "__fish_use_subcommand" -a "scan" -d "Run security scan"
complete -c strike -f -n "__fish_use_subcommand" -a "report" -d "Generate report"
complete -c strike -f -n "__fish_use_subcommand" -a "init" -d "Initialize config"
complete -c strike -f -n "__fish_use_subcommand" -a "config" -d "Manage config"
complete -c strike -f -n "__fish_use_subcommand" -a "completions" -d "Generate completions"
complete -c strike -f -n "__fish_use_subcommand" -a "help" -d "Show help"
complete -c strike -f -n "__fish_use_subcommand" -a "version" -d "Show version"

# Global options
complete -c strike -l help -d "Show help"
complete -c strike -l version -d "Show version"
complete -c strike -l verbose -d "Verbose output"
complete -c strike -l quiet -d "Quiet mode"
complete -c strike -l config -r -d "Config file path"

# Scan options
complete -c strike -n "__fish_seen_subcommand_from scan" -l target -r -d "Target URL"
complete -c strike -n "__fish_seen_subcommand_from scan" -l output -r -d "Output file"
complete -c strike -n "__fish_seen_subcommand_from scan" -l format -r -a "json html pdf sarif markdown" -d "Output format"
complete -c strike -n "__fish_seen_subcommand_from scan" -l severity -r -a "critical high medium low info" -d "Min severity"
complete -c strike -n "__fish_seen_subcommand_from scan" -l max-findings -r -d "Max findings"
complete -c strike -n "__fish_seen_subcommand_from scan" -l timeout -r -d "Timeout (seconds)"
complete -c strike -n "__fish_seen_subcommand_from scan" -l workers -r -d "Worker threads"
complete -c strike -n "__fish_seen_subcommand_from scan" -l allow-private -d "Allow private IPs"

# Report options
complete -c strike -n "__fish_seen_subcommand_from report" -l input -r -d "Input file"
complete -c strike -n "__fish_seen_subcommand_from report" -l output -r -d "Output file"
complete -c strike -n "__fish_seen_subcommand_from report" -l format -r -a "json html pdf sarif markdown" -d "Output format"
complete -c strike -n "__fish_seen_subcommand_from report" -l template -r -d "Report template"

# Completions shells
complete -c strike -n "__fish_seen_subcommand_from completions" -a "bash zsh fish powershell" -d "Shell type"
"#.to_string()
    }
    
    fn generate_powershell() -> String {
        r#"# strike PowerShell completion

Register-ArgumentCompleter -Native -CommandName strike -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    
    $commands = @('scan', 'report', 'init', 'config', 'completions', 'help', 'version')
    $formats = @('json', 'html', 'pdf', 'sarif', 'markdown')
    $severities = @('critical', 'high', 'medium', 'low', 'info')
    $shells = @('bash', 'zsh', 'fish', 'powershell')
    
    $commandElements = $commandAst.CommandElements
    $command = @(
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [System.Management.Automation.Language.StringConstantExpressionAst] -or
                $element.StringConstantType -ne [System.Management.Automation.Language.StringConstantType]::BareWord -or
                $element.Value.StartsWith('-')) {
                break
            }
            $element.Value
        }
    )
    
    $completions = @()
    
    if ($command.Count -eq 0) {
        $completions = $commands | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
    elseif ($command[0] -eq 'scan') {
        if ($wordToComplete -like '--format*') {
            $completions = $formats | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new("--format $_", $_, 'ParameterValue', $_)
            }
        }
        elseif ($wordToComplete -like '--severity*') {
            $completions = $severities | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new("--severity $_", $_, 'ParameterValue', $_)
            }
        }
    }
    elseif ($command[0] -eq 'completions') {
        $completions = $shells | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
    
    $completions
}
"#.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_from_str() {
        assert!(matches!(Shell::from_str("bash"), Some(Shell::Bash)));
        assert!(matches!(Shell::from_str("zsh"), Some(Shell::Zsh)));
        assert!(matches!(Shell::from_str("fish"), Some(Shell::Fish)));
        assert!(matches!(Shell::from_str("powershell"), Some(Shell::PowerShell)));
        assert!(Shell::from_str("invalid").is_none());
    }
    
    #[test]
    fn test_shell_as_str() {
        assert_eq!(Shell::Bash.as_str(), "bash");
        assert_eq!(Shell::Zsh.as_str(), "zsh");
        assert_eq!(Shell::Fish.as_str(), "fish");
        assert_eq!(Shell::PowerShell.as_str(), "powershell");
    }
    
    #[test]
    fn test_generate_bash() {
        let completion = CompletionGenerator::generate(Shell::Bash).unwrap();
        assert!(completion.contains("_strike_completions"));
        assert!(completion.contains("complete -F"));
    }
    
    #[test]
    fn test_generate_zsh() {
        let completion = CompletionGenerator::generate(Shell::Zsh).unwrap();
        assert!(completion.contains("#compdef strike"));
        assert!(completion.contains("_strike"));
    }
    
    #[test]
    fn test_generate_fish() {
        let completion = CompletionGenerator::generate(Shell::Fish).unwrap();
        assert!(completion.contains("complete -c strike"));
    }
    
    #[test]
    fn test_generate_powershell() {
        let completion = CompletionGenerator::generate(Shell::PowerShell).unwrap();
        assert!(completion.contains("Register-ArgumentCompleter"));
    }
}
