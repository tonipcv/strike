# Contributing to Strike

Thank you for your interest in contributing to Strike! This document provides guidelines and instructions for contributing.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Adding New Tools](#adding-new-tools)
- [Creating Workflows](#creating-workflows)
- [Testing](#testing)
- [Code Style](#code-style)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors.

### Our Standards

- **Be respectful** - Treat everyone with respect
- **Be constructive** - Provide helpful feedback
- **Be collaborative** - Work together towards common goals
- **Be professional** - Maintain professionalism in all interactions

### Unacceptable Behavior

- Harassment or discrimination
- Trolling or insulting comments
- Personal or political attacks
- Publishing others' private information

---

## Getting Started

### Prerequisites

- Rust 1.75 or higher
- Git
- Basic knowledge of security testing
- Familiarity with Rust and async programming

### Fork and Clone

```bash
# Fork the repository on GitHub
# Then clone your fork
git clone https://github.com/YOUR_USERNAME/strike.git
cd strike

# Add upstream remote
git remote add upstream https://github.com/ORIGINAL_OWNER/strike.git
```

---

## Development Setup

### 1. Install Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install security tools (optional but recommended)
# Ubuntu/Debian
sudo apt-get install nmap masscan

# macOS
brew install nmap masscan

# Install Go tools
go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest
go install github.com/projectdiscovery/subfinder/v2/cmd/subfinder@latest
```

### 2. Build Strike

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- --help
```

### 3. Set Up Development Environment

```bash
# Copy environment template
cp .env.example .env

# Add your API keys
nano .env

# Run in development mode
cargo run -- scan --target example.com --dry-run
```

---

## How to Contribute

### Types of Contributions

1. **Bug Reports** - Report bugs via GitHub Issues
2. **Feature Requests** - Suggest new features
3. **Code Contributions** - Submit pull requests
4. **Documentation** - Improve docs and guides
5. **Tool Wrappers** - Add support for new security tools
6. **Workflows** - Share custom YAML workflows
7. **Testing** - Write tests and improve coverage

### Reporting Bugs

Use the bug report template:

```markdown
**Describe the bug**
A clear description of the bug.

**To Reproduce**
Steps to reproduce:
1. Run command '...'
2. See error

**Expected behavior**
What you expected to happen.

**Environment**
- OS: [e.g., Ubuntu 22.04]
- Strike version: [e.g., 1.0.0]
- Rust version: [e.g., 1.75]

**Additional context**
Any other relevant information.
```

### Suggesting Features

Use the feature request template:

```markdown
**Feature Description**
Clear description of the feature.

**Use Case**
Why is this feature needed?

**Proposed Solution**
How should it work?

**Alternatives**
Other solutions you've considered.
```

---

## Adding New Tools

### Quick Start

```rust
// src/tools/wrappers/mytool.rs

use crate::tools::trait::*;
use async_trait::async_trait;

pub struct MyToolWrapper {
    binary_path: PathBuf,
}

#[async_trait]
impl Tool for MyToolWrapper {
    fn name(&self) -> &str {
        "mytool"
    }
    
    fn description(&self) -> &str {
        "Description of my tool"
    }
    
    fn category(&self) -> ToolCategory {
        ToolCategory::Scanning
    }
    
    // Implement other trait methods...
}
```

2. **Register Tool**

```rust
// src/tools/wrappers/mod.rs

pub mod mytool;
pub use mytool::MyToolWrapper;
```

3. **Add Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mytool_installation() {
        let tool = MyToolWrapper::new();
        let installed = tool.check_installation().await.unwrap();
        assert!(installed);
    }
}
```

4. **Update Documentation**

Add your tool to:
- README.md (list of supported tools)
- Dockerfile (if it needs to be pre-installed)

---

## Creating Workflows

### Quick Start

1. **Create YAML File**

```yaml
# workflows/my-workflow.yaml

name: My Custom Workflow
description: Description of what this workflow does
version: 1.0

phases:
  - name: phase1
    tools:
      - tool1
      - tool2
  
  - name: phase2
    depends_on:
      - phase1
    tools:
      - tool3
```

2. **Test Workflow**

```bash
strike workflow validate my-workflow.yaml
strike scan --target example.com --workflow my-workflow.yaml --dry-run
```

3. **Share Workflow**

Submit a PR to add your workflow to the `workflows/` directory.

---

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run ignored tests (require tools installed)
cargo test -- --ignored

# Run integration tests
cargo test --test integration_test
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_function() {
        let result = my_function();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = my_async_function().await.unwrap();
        assert!(result.is_valid());
    }

    #[tokio::test]
    #[ignore] // Requires external dependencies
    async fn test_integration() {
        // Integration test code
    }
}
```

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage
```

---

## Code Style

### Rust Style Guide

Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/).

### Formatting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings
```

### Best Practices

1. **Error Handling**
   - Use `Result<T>` for fallible operations
   - Provide context with `.context()`
   - Use custom error types when appropriate

2. **Async Code**
   - Use `async/await` syntax
   - Avoid blocking operations in async context
   - Use `tokio::spawn` for concurrent tasks

3. **Documentation**
   - Document public APIs with `///`
   - Include examples in doc comments
   - Keep docs up to date

4. **Logging**
   - Use `tracing` for structured logging
   - Use appropriate log levels (trace, debug, info, warn, error)
   - Include context in log messages

### Example

```rust
use anyhow::{Context, Result};
use tracing::{debug, info};

/// Performs a security scan on the target.
///
/// # Arguments
///
/// * `target` - The target URL or IP address
/// * `options` - Scan configuration options
///
/// # Examples
///
/// ```
/// let result = scan_target("example.com", &options).await?;
/// ```
pub async fn scan_target(target: &str, options: &ScanOptions) -> Result<ScanResult> {
    info!("Starting scan of {}", target);
    debug!("Scan options: {:?}", options);
    
    let result = perform_scan(target, options)
        .await
        .context("Failed to perform scan")?;
    
    Ok(result)
}
```

---

## Pull Request Process

### Before Submitting

1. **Update your fork**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature/my-feature
   ```

3. **Make your changes**
   - Write clean, documented code
   - Add tests for new functionality
   - Update documentation

4. **Test your changes**
   ```bash
   cargo test
   cargo fmt
   cargo clippy
   ```

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

   Use conventional commits:
   - `feat:` - New feature
   - `fix:` - Bug fix
   - `docs:` - Documentation changes
   - `test:` - Test additions/changes
   - `refactor:` - Code refactoring
   - `chore:` - Maintenance tasks

### Submitting PR

1. **Push to your fork**
   ```bash
   git push origin feature/my-feature
   ```

2. **Create Pull Request**
   - Go to GitHub and create a PR
   - Fill out the PR template
   - Link related issues

3. **PR Template**
   ```markdown
   ## Description
   Brief description of changes.
   
   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation update
   
   ## Testing
   - [ ] Tests pass locally
   - [ ] Added new tests
   - [ ] Updated documentation
   
   ## Checklist
   - [ ] Code follows style guidelines
   - [ ] Self-review completed
   - [ ] Comments added for complex code
   - [ ] Documentation updated
   - [ ] No new warnings
   ```

### Review Process

1. **Automated Checks**
   - CI/CD pipeline runs tests
   - Code formatting verified
   - Clippy lints checked

2. **Code Review**
   - Maintainers review code
   - Address feedback
   - Make requested changes

3. **Approval and Merge**
   - PR approved by maintainer
   - Squash and merge to main
   - Branch deleted

---

## Community

### Communication Channels

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - General questions and ideas
- **Discord** - Real-time chat (link in README)
- **Twitter** - Updates and announcements

### Getting Help

- Check [README.md](README.md) and documentation
- Search existing issues
- Ask in GitHub Discussions
- Join Discord community

### Recognition

Contributors are recognized in:
- CONTRIBUTORS.md file
- Release notes
- Project README

---

## Development Workflow

### Typical Workflow

1. **Pick an Issue**
   - Browse open issues
   - Comment to claim issue
   - Ask questions if unclear

2. **Develop**
   - Create feature branch
   - Write code and tests
   - Commit regularly

3. **Test**
   - Run all tests
   - Test manually
   - Check edge cases

4. **Document**
   - Update relevant docs
   - Add code comments
   - Update CHANGELOG

5. **Submit**
   - Create pull request
   - Respond to feedback
   - Iterate until approved

### Release Process

Maintainers handle releases:

1. Version bump in Cargo.toml
2. Update CHANGELOG.md
3. Create git tag
4. Publish to crates.io
5. Create GitHub release
6. Announce on social media

---

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (see LICENSE file).

---

## Questions?

If you have questions about contributing:

1. Check this guide and other documentation
2. Search existing issues and discussions
3. Ask in GitHub Discussions
4. Reach out on Discord

Thank you for contributing to Strike! 🚀

---

**Last Updated:** 2024-01-01  
**Strike Version:** 1.0.0
