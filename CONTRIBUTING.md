# Contributing to Matrixon

Thank you for your interest in contributing to Matrixon! We welcome contributions from the community and are excited to see what you can bring to this next-generation Matrix server project.

## 🚀 Quick Start for Contributors

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Create a new branch** for your feature/fix
4. **Make your changes** following our coding standards
5. **Test your changes** thoroughly
6. **Submit a pull request**

## 📋 Ways to Contribute

### 🐛 Bug Reports
- Search existing issues first
- Use the bug report template
- Include steps to reproduce
- Provide system information
- Add relevant logs/error messages

### 💡 Feature Requests
- Check if feature already requested
- Use the feature request template
- Explain the use case clearly
- Consider implementation complexity
- Discuss with maintainers first for major features

### 📝 Code Contributions
- Follow coding standards (see below)
- Write tests for new functionality
- Update documentation as needed
- Ensure CI passes
- Keep PRs focused and atomic

### 📚 Documentation
- Fix typos and grammar
- Improve clarity and examples
- Add missing documentation
- Update outdated information
- Translate to other languages

## 🛠️ Development Setup

### Prerequisites
- Rust 1.85.0+ (managed via rustup)
- PostgreSQL 14+ (for testing)
- Redis 6+ (for caching)
- Docker & Docker Compose (optional but recommended)

### Local Development
```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/Matrixon.git
cd Matrixon

# Install development tools
cargo install cargo-watch cargo-audit cargo-tarpaulin

# Set up environment
cp .env.example .env
# Edit .env with your database configuration

# Run in development mode
cargo watch -x run

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt --check
```

### Project Structure
```
Matrixon/
├── src/                    # Main application code
│   ├── main.rs            # Application entry point
│   ├── lib.rs             # Library root
│   ├── api/               # Matrix API implementations
│   ├── cli/               # Command line interface
│   └── ...
├── crates/                # Workspace crates
│   ├── matrixon-core/     # Core server functionality
│   ├── matrixon-ai/       # AI features
│   ├── matrixon-web3/     # Web3 integration
│   └── ...
├── examples/              # Usage examples
├── tests/                 # Integration tests
├── docs/                  # Documentation
└── docker/               # Docker configurations
```

## 📏 Coding Standards

### Rust Code Style
```rust
// File header (mandatory)
/*
 * Copyright (c) 2024 Matrixon Team
 * 
 * This file is part of Matrixon.
 * 
 * Author: Your Name <your.email@example.com>
 * Date: 2024-01-XX
 * Version: 0.11.0-alpha
 * Purpose: Brief description of the file's purpose
 */

// Imports (organized: std -> external -> internal)
use std::{
    collections::HashMap,
    sync::Arc,
};

use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

use crate::{
    config::Config,
    error::MatrixonError,
};

// Function documentation
/// Handles user authentication with detailed error reporting
/// 
/// # Arguments
/// * `credentials` - User login credentials
/// * `config` - Server configuration
/// 
/// # Returns
/// * `Ok(AuthToken)` - Valid authentication token
/// * `Err(AuthError)` - Authentication failure details
/// 
/// # Example
/// ```rust
/// let token = authenticate_user(creds, &config).await?;
/// ```
#[instrument(level = "debug")]
pub async fn authenticate_user(
    credentials: UserCredentials,
    config: &Config,
) -> Result<AuthToken, AuthError> {
    let start = Instant::now();
    debug!("🔧 Starting user authentication");
    
    // Input validation
    if credentials.username.is_empty() {
        return Err(AuthError::InvalidUsername);
    }
    
    // Implementation...
    
    info!("✅ Authentication completed in {:?}", start.elapsed());
    Ok(token)
}
```

### Error Handling
- Use `Result<T, Error>` for all fallible operations
- Provide context with error messages
- Log errors with appropriate levels
- Use `anyhow` for error chaining
- Implement `Display` and `Error` traits

### Performance Guidelines
- Use `async/await` for I/O operations
- Avoid blocking calls in async context
- Implement proper connection pooling
- Use efficient data structures
- Profile performance-critical paths
- Add performance metrics

### Testing Standards
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_user_authentication_success() {
        // Test successful authentication
        let credentials = UserCredentials::new("test_user", "password");
        let config = Config::default();
        
        let result = authenticate_user(credentials, &config).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_user_authentication_invalid_credentials() {
        // Test authentication failure
        let credentials = UserCredentials::new("", "");
        let config = Config::default();
        
        let result = authenticate_user(credentials, &config).await;
        assert!(matches!(result, Err(AuthError::InvalidUsername)));
    }
}
```

## 🧪 Testing Guidelines

### Test Categories
1. **Unit Tests** - Test individual functions
2. **Integration Tests** - Test component interactions
3. **End-to-End Tests** - Test full workflows
4. **Performance Tests** - Benchmark critical paths
5. **Compliance Tests** - Matrix protocol conformance

### Running Tests
```bash
# All tests
cargo test

# Specific test
cargo test test_authentication

# With output
cargo test -- --nocapture

# Performance tests
cargo test --release --test performance

# Coverage report
cargo tarpaulin --out html
```

## 📝 Documentation Standards

### Code Documentation
- Document all public functions and types
- Include examples in documentation
- Reference Matrix specification where applicable
- Document performance characteristics
- Explain error conditions

### API Documentation
- Follow OpenAPI specification
- Include request/response examples
- Document rate limits
- Explain authentication requirements
- Provide error code references

## 🔄 Pull Request Process

### Before Submitting
1. **Rebase** on latest main branch
2. **Run tests** and ensure they pass
3. **Check code style** with clippy and fmt
4. **Update documentation** if needed
5. **Add changelog entry** if applicable

### PR Requirements
- **Descriptive title** and description
- **Link related issues** with "Fixes #123"
- **Small, focused changes** (prefer multiple small PRs)
- **Comprehensive tests** for new features
- **No breaking changes** without discussion

### Review Process
1. **Automated checks** must pass (CI/CD)
2. **Code review** by maintainers
3. **Testing** in development environment
4. **Documentation review** if applicable
5. **Final approval** and merge

## 🌟 Recognition

Contributors are recognized in:
- **Contributors list** in README
- **Changelog** for significant contributions
- **Release notes** for major features
- **Blog posts** highlighting innovations

## 🤝 Code of Conduct

### Our Standards
- **Be respectful** and inclusive
- **Welcome newcomers** and help them learn
- **Focus on constructive feedback**
- **Assume positive intent**
- **Credit others' work** appropriately

### Unacceptable Behavior
- Harassment or discrimination
- Aggressive or derogatory language
- Personal attacks
- Spam or off-topic discussions
- Violation of others' privacy

## 📞 Getting Help

### Communication Channels
- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - Questions and general discussion
- **Matrix Room** - Real-time chat with community
- **Email** - Direct contact with maintainers

### Asking Questions
1. **Search existing** issues and discussions
2. **Use appropriate channel** for your question type
3. **Provide context** and relevant information
4. **Be patient** - maintainers are volunteers
5. **Follow up** if you find solutions

## 🏆 Contributor Levels

### 🌱 New Contributor
- Fix documentation typos
- Add examples or tests
- Report bugs with detailed information
- Participate in discussions

### 🚀 Regular Contributor
- Implement new features
- Fix complex bugs
- Review other contributors' code
- Help maintain documentation

### 🎯 Core Contributor
- Design system architecture
- Make breaking changes decisions
- Mentor new contributors
- Release management

### 🌟 Maintainer
- Repository administration
- Community management
- Strategic direction
- Security oversight

## 📊 Metrics and Goals

### Quality Metrics
- **Test Coverage**: Aim for 90%+
- **Documentation Coverage**: 100% for public APIs
- **Performance**: <50ms average response time
- **Reliability**: >99.9% uptime in production

### Community Metrics
- **Response Time**: <24 hours for issues
- **Review Time**: <48 hours for PRs
- **Release Cadence**: Monthly minor releases
- **Security**: <7 days for security fixes

---

## 🎉 Thank You!

Your contributions make Matrixon better for everyone. Whether you're fixing a typo or implementing a major feature, every contribution is valuable and appreciated.

**Happy coding!** 🦀

---

*For questions about this guide, please open an issue or contact the maintainers.* 
