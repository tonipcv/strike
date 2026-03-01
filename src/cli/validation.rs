use anyhow::Result;
use crate::config::validation::InputValidator;
use super::args::Commands;

pub fn validate_command(command: &Commands, allow_private: bool) -> Result<()> {
    match command {
        Commands::Init { target, auth, roe, output_dir, .. } => {
            if allow_private {
                InputValidator::validate_local_url(target)?;
            } else {
                InputValidator::validate_target_url(target)?;
            }
            
            if let Some(auth_path) = auth {
                InputValidator::validate_file_path(auth_path)?;
            }
            
            if let Some(roe_path) = roe {
                InputValidator::validate_file_path(roe_path)?;
            }
            
            InputValidator::validate_file_path(output_dir)?;
        }
        
        Commands::Run { workers, rate_limit, timeout, .. } => {
            InputValidator::validate_worker_count(*workers)?;
            InputValidator::validate_rate_limit(*rate_limit)?;
            InputValidator::validate_timeout(*timeout)?;
        }
        
        Commands::Recon { target, .. } => {
            if allow_private {
                InputValidator::validate_local_url(target)?;
            } else {
                InputValidator::validate_target_url(target)?;
            }
        }
        
        Commands::Scan { endpoint, auth_session, .. } => {
            if allow_private {
                InputValidator::validate_local_url(endpoint)?;
            } else {
                InputValidator::validate_target_url(endpoint)?;
            }
            
            if let Some(session_path) = auth_session {
                InputValidator::validate_file_path(session_path)?;
            }
        }
        
        Commands::Agent { target, .. } => {
            if allow_private {
                InputValidator::validate_local_url(target)?;
            } else {
                InputValidator::validate_target_url(target)?;
            }
        }
        
        Commands::Ci { config, .. } => {
            InputValidator::validate_file_path(config)?;
        }
        
        _ => {}
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_init_command_public_url() {
        let cmd = Commands::Init {
            target: "https://example.com".to_string(),
            env: "staging".to_string(),
            auth: None,
            roe: None,
            output_dir: ".strike".to_string(),
        };
        
        let result = validate_command(&cmd, false);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validate_init_command_blocks_localhost() {
        let cmd = Commands::Init {
            target: "http://localhost:8080".to_string(),
            env: "local".to_string(),
            auth: None,
            roe: None,
            output_dir: ".strike".to_string(),
        };
        
        let result = validate_command(&cmd, false);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_init_command_allows_localhost_with_flag() {
        let cmd = Commands::Init {
            target: "http://localhost:8080".to_string(),
            env: "local".to_string(),
            auth: None,
            roe: None,
            output_dir: ".strike".to_string(),
        };
        
        let result = validate_command(&cmd, true);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_validate_run_command_invalid_workers() {
        let cmd = Commands::Run {
            profile: "full".to_string(),
            focus: None,
            workers: 0,
            resume: None,
            dry_run: false,
            no_exploit: false,
            max_depth: 10,
            rate_limit: 50,
            timeout: 300,
            output: "json".to_string(),
            ci: false,
        };
        
        let result = validate_command(&cmd, false);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_validate_run_command_invalid_rate_limit() {
        let cmd = Commands::Run {
            profile: "full".to_string(),
            focus: None,
            workers: 16,
            resume: None,
            dry_run: false,
            no_exploit: false,
            max_depth: 10,
            rate_limit: 0,
            timeout: 300,
            output: "json".to_string(),
            ci: false,
        };
        
        let result = validate_command(&cmd, false);
        assert!(result.is_err());
    }
}
