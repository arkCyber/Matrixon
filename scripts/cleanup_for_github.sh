#!/bin/bash

# =============================================================================
# Matrixon Matrix NextServer - GitHub Upload Preparation Script
# =============================================================================
#
# Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
# Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
# Contributors: Matrixon Development Team
# Date: 2024-12-11
# Version: 2.0.0-alpha (PostgreSQL Backend)
# License: Apache 2.0 / MIT
#
# Description:
#   Clean up project directory for GitHub upload. Removes temporary files,
#   build artifacts, test outputs, and organizes project structure.
#
# =============================================================================

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# Statistics
DELETED_FILES=0
DELETED_DIRS=0
MOVED_FILES=0
CLEANED_SIZE=0

print_header() {
    echo -e "${CYAN}🧹 Matrixon Project Cleanup for GitHub Upload${NC}"
    echo "================================================"
    echo "Preparing Matrixon for clean GitHub repository"
    echo "Date: $(date)"
    echo
}

log_action() {
    local action="$1"
    local target="$2"
    local size="${3:-0}"
    
    case "$action" in
        "DELETE_FILE")
            echo -e "${RED}🗑️  Deleted file: ${target}${NC}"
            ((DELETED_FILES++))
            CLEANED_SIZE=$((CLEANED_SIZE + size))
            ;;
        "DELETE_DIR")
            echo -e "${RED}📁 Deleted directory: ${target}${NC}"
            ((DELETED_DIRS++))
            ;;
        "MOVE")
            echo -e "${GREEN}📦 Moved: ${target}${NC}"
            ((MOVED_FILES++))
            ;;
        "KEEP")
            echo -e "${BLUE}✅ Kept: ${target}${NC}"
            ;;
        "INFO")
            echo -e "${YELLOW}ℹ️  ${target}${NC}"
            ;;
    esac
}

# Get file size in bytes
get_file_size() {
    if [[ -f "$1" ]]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            stat -f%z "$1" 2>/dev/null || echo 0
        else
            stat -c%s "$1" 2>/dev/null || echo 0
        fi
    else
        echo 0
    fi
}

# Clean build artifacts
clean_build_artifacts() {
    log_action "INFO" "Cleaning build artifacts..."
    
    if [[ -d "target" ]]; then
        local size=$(du -sb target 2>/dev/null | cut -f1 || echo 0)
        rm -rf target
        log_action "DELETE_DIR" "target/ (build artifacts)" "$size"
    fi
    
    # Remove backup files
    find . -name "*.backup" -type f | while read -r file; do
        local size=$(get_file_size "$file")
        rm -f "$file"
        log_action "DELETE_FILE" "$file" "$size"
    done
}

# Clean temporary and cache files
clean_temp_files() {
    log_action "INFO" "Cleaning temporary and cache files..."
    
    # Python cache
    if [[ -d "__pycache__" ]]; then
        rm -rf "__pycache__"
        log_action "DELETE_DIR" "__pycache__/ (Python cache)"
    fi
    
    # macOS files
    find . -name ".DS_Store" -type f | while read -r file; do
        local size=$(get_file_size "$file")
        rm -f "$file"
        log_action "DELETE_FILE" "$file" "$size"
    done
    
    # Log files
    find . -name "*.log" -type f | while read -r file; do
        local size=$(get_file_size "$file")
        rm -f "$file"
        log_action "DELETE_FILE" "$file" "$size"
    done
    
    # PID files
    find . -name "*.pid" -type f | while read -r file; do
        local size=$(get_file_size "$file")
        rm -f "$file"
        log_action "DELETE_FILE" "$file" "$size"
    done
}

# Clean test artifacts
clean_test_artifacts() {
    log_action "INFO" "Cleaning test artifacts..."
    
    # Test result directories
    local test_dirs=("test_results" "room_test_results" "quick_test_results" "database_test_results")
    for dir in "${test_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            rm -rf "$dir"
            log_action "DELETE_DIR" "$dir/ (test results)"
        fi
    done
    
    # Temporary database directories
    local db_dirs=(":memory:" "postgresql:" "matrixon.db")
    for dir in "${db_dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            rm -rf "$dir"
            log_action "DELETE_DIR" "$dir/ (temp database)"
        fi
    done
    
    # Remove test executable
    if [[ -f "user_levels_demo" ]]; then
        local size=$(get_file_size "user_levels_demo")
        rm -f "user_levels_demo"
        log_action "DELETE_FILE" "user_levels_demo (test binary)" "$size"
    fi
}

# Organize scripts into scripts directory
organize_scripts() {
    log_action "INFO" "Organizing scripts..."
    
    # Create scripts directory if it doesn't exist
    if [[ ! -d "scripts" ]]; then
        mkdir -p scripts
        log_action "INFO" "Created scripts/ directory"
    fi
    
    # Move shell scripts (except this one)
    local script_files=(
        "*.sh"
        "*.py"
    )
    
    for pattern in "${script_files[@]}"; do
        for file in $pattern; do
            if [[ -f "$file" && "$file" != "cleanup_for_github.sh" ]]; then
                # Skip if already in scripts directory
                if [[ ! "$file" =~ ^scripts/ ]]; then
                    mv "$file" "scripts/"
                    log_action "MOVE" "$file -> scripts/"
                fi
            fi
        done
    done
}

# Organize documentation
organize_documentation() {
    log_action "INFO" "Organizing documentation..."
    
    # Create docs directory if it doesn't exist  
    if [[ ! -d "docs" ]]; then
        mkdir -p docs
        log_action "INFO" "Created docs/ directory"
    fi
    
    # Move markdown files (except main README.md)
    find . -maxdepth 1 -name "*.md" -type f | while read -r file; do
        filename=$(basename "$file")
        if [[ "$filename" != "README.md" && ! "$file" =~ ^docs/ ]]; then
            mv "$file" "docs/"
            log_action "MOVE" "$file -> docs/"
        fi
    done
}

# Organize configuration files
organize_configs() {
    log_action "INFO" "Organizing configuration files..."
    
    # Create configs directory if it doesn't exist
    if [[ ! -d "configs" ]]; then
        mkdir -p configs
        log_action "INFO" "Created configs/ directory"
    fi
    
    # Move TOML configuration files
    find . -maxdepth 1 -name "*.toml" -type f | while read -r file; do
        filename=$(basename "$file")
        # Keep main Cargo.toml in root
        if [[ "$filename" != "Cargo.toml" && ! "$file" =~ ^configs/ ]]; then
            mv "$file" "configs/"
            log_action "MOVE" "$file -> configs/"
        fi
    done
}

# Remove empty directories
remove_empty_dirs() {
    log_action "INFO" "Removing empty directories..."
    
    # Find and remove empty directories (except .git and important ones)
    find . -type d -empty | while read -r dir; do
        # Skip important directories
        if [[ ! "$dir" =~ \.git && ! "$dir" =~ ^\./ && "$dir" != "." ]]; then
            rmdir "$dir" 2>/dev/null
            log_action "DELETE_DIR" "$dir/ (empty)"
        fi
    done
}

# Create/update .gitignore
create_gitignore() {
    log_action "INFO" "Creating/updating .gitignore..."
    
    cat > .gitignore << 'EOF'
# =============================================================================
# Matrixon Matrix NextServer - Git Ignore Rules
# =============================================================================

# Build artifacts
/target/
*.exe
*.dll
*.so
*.dylib

# Rust specific
Cargo.lock
*.pdb

# Database files
*.db
*.db-journal
*.sqlite
*.sqlite3
matrixon.db/
postgresql:/
:memory:/

# Logs and temporary files
*.log
*.tmp
*.pid
*.lock
debug.log
matrixon.log

# OS specific
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# Editor specific
.vscode/settings.json
.vscode/launch.json
.vscode/tasks.json
.idea/
*.swp
*.swo
*~

# Python
__pycache__/
*.py[cod]
*$py.class
*.so
.Python
env/
venv/
.env

# Test artifacts
test_results/
room_test_results/
quick_test_results/
database_test_results/
*.backup

# Media storage
media_storage/
uploads/

# Configuration secrets
*secret*
*password*
*.key
*.pem

# Development files
.cargo/config
.rustfmt.toml
clippy.toml

# Documentation build
book/
mdbook/

# Performance profiling
*.prof
perf.data*
flamegraph.svg

# Coverage reports
tarpaulin-report.html
cobertura.xml
lcov.info

# Temporary directories
tmp/
temp/
cache/

EOF

    log_action "INFO" "Updated .gitignore with comprehensive rules"
}

# Create proper README.md for GitHub
update_readme() {
    log_action "INFO" "Updating README.md for GitHub..."
    
    cat > README.md << 'EOF'
# Matrixon Matrix NextServer

**Ultra High Performance Matrix NextServer (Synapse Alternative)**

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![Matrix](https://img.shields.io/badge/matrix-v1.13-blue.svg)](https://matrix.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0%2FMIT-blue.svg)](LICENSE)

---

## 🚀 Overview

Matrixon is a next-generation Matrix NextServer implementation written in Rust, designed for enterprise-grade deployment with exceptional performance characteristics. Built as a high-performance alternative to Synapse, Matrixon targets the most demanding Matrix deployments.

### 🎯 Performance Targets

- **20,000+** concurrent connections
- **<50ms** response latency
- **>99%** success rate
- **Memory-efficient** operation
- **Horizontal scalability**

## ✨ Features

### 🔧 Core Matrix Protocol
- ✅ Matrix v1.13 specification compliance
- ✅ Client-Server API implementation  
- ✅ Server-Server federation protocol
- ✅ End-to-end encryption support
- ✅ Media repository functionality
- ✅ User authentication and authorization

### 🏢 Enterprise Features
- 🔐 Advanced security and compliance
- 📊 Comprehensive monitoring and metrics
- 🌐 Multi-language support (i18n)
- 🤖 AI-powered content moderation
- 🔌 Plugin architecture
- 📱 VoIP and WebRTC support

### 🛠️ Operations & Management
- 📈 Performance monitoring
- 🔄 Automated backup and recovery
- 🎛️ Advanced administration tools
- 📊 Real-time analytics
- 🔍 Comprehensive logging

## 🏗️ Architecture

```
Matrixon/
├── src/
│   ├── api/           # Matrix API implementations
│   ├── database/      # Database abstraction layer
│   ├── service/       # Core business logic
│   ├── config/        # Configuration management
│   ├── utils/         # Utility functions
│   └── cli/          # Command-line interface
├── configs/          # Configuration files
├── docs/            # Documentation
├── scripts/         # Utility scripts
└── examples/        # Usage examples
```

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.70+ (latest stable recommended)
- **PostgreSQL** 13+ (recommended) or SQLite
- **System**: Linux, macOS, or Windows

### Installation

```bash
# Clone the repository
git clone https://github.com/arkSong/Matrixon.git
cd Matrixon

# Build the project
cargo build --release

# Copy example configuration
cp configs/matrixon-pg.toml matrixon.toml

# Edit configuration as needed
vim matrixon.toml

# Run Matrixon
./target/release/matrixon
```

### Configuration

Matrixon supports multiple database backends:

- **PostgreSQL** (recommended for production)
- **SQLite** (for development and testing)
- **RocksDB** (for high-performance scenarios)

See `configs/` directory for example configurations.

## 📚 Documentation

- **[Installation Guide](docs/SETUP_COMPLETE.md)** - Complete setup instructions
- **[Configuration](docs/DATABASE_CONFIGURATION_SUMMARY.md)** - Database and server configuration
- **[Administration](docs/ADMIN_API_ENHANCEMENT_SUMMARY.md)** - Admin tools and API
- **[Performance](docs/PERFORMANCE_RECOMMENDATIONS_20K.md)** - Performance tuning
- **[Federation](docs/FEDERATION_IMPROVEMENTS.md)** - Federation setup
- **[AI Features](docs/AI_CONTENT_MODERATION_SUMMARY.md)** - AI content moderation

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration

# Run performance benchmarks
cargo bench

# Matrix protocol compliance tests
./scripts/matrix_test_suite.sh
```

## 🤝 Contributing

We welcome contributions to Matrixon! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch cargo-tarpaulin

# Run tests in watch mode
cargo watch -x test

# Check code coverage
cargo tarpaulin --out html
```

## 📊 Project Status

Matrixon is currently in **Alpha** status. While it implements most Matrix features, some advanced functionality is still in development:

- ✅ Core Matrix protocol
- ✅ Basic federation
- ✅ E2E encryption (chat works)
- ⚠️ E2E emoji comparison over federation
- ⚠️ Outgoing read receipts/typing/presence over federation

## 🏢 Enterprise Support

For enterprise deployments, support, and custom development:

**Contact**: arkSong (arksong2018@gmail.com)

## 📜 License

This project is dual-licensed under:

- **Apache License 2.0** ([LICENSE-APACHE](LICENSE-APACHE))
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))

## 🙏 Acknowledgments

- **[Synapse](https://github.com/element-hq/synapse)** - Reference implementation
- **[Matrix.org](https://matrix.org/)** - Protocol specification
- **Rust Community** - Amazing ecosystem and tools

---

**Founded by**: arkSong (arksong2018@gmail.com) - Matrixon Innovation Project  
**Matrix Spec**: [matrix.org/docs/spec](https://matrix.org/docs/spec/)  
**Performance**: Designed for 20k+ connections, <50ms latency, >99% uptime
EOF

    log_action "INFO" "Updated README.md with comprehensive GitHub-ready content"
}

# Show summary
show_summary() {
    echo
    echo -e "${CYAN}📊 GitHub Cleanup Summary${NC}"
    echo "========================="
    echo "Files deleted: $DELETED_FILES"
    echo "Directories removed: $DELETED_DIRS"
    echo "Files moved: $MOVED_FILES"
    echo "Space cleaned: $(numfmt --to=iec $CLEANED_SIZE 2>/dev/null || echo "${CLEANED_SIZE} bytes")"
    
    echo
    echo -e "${GREEN}✅ Project Structure After Cleanup:${NC}"
    echo "📁 Matrixon/"
    echo "├── 📄 README.md (GitHub-ready)"
    echo "├── 📄 Cargo.toml (main manifest)"
    echo "├── 📄 .gitignore (comprehensive)"
    echo "├── 📁 src/ (source code)"
    echo "├── 📁 configs/ (configuration files)"
    echo "├── 📁 docs/ (documentation)"
    echo "├── 📁 scripts/ (utility scripts)"
    echo "├── 📁 examples/ (usage examples)"
    echo "└── 📁 tests/ (test files)"
    
    echo
    echo -e "${BLUE}🚀 Ready for GitHub Upload!${NC}"
    echo "Next steps:"
    echo "1. Review the changes: git status"
    echo "2. Add files: git add ."
    echo "3. Commit: git commit -m 'Prepare Matrixon for GitHub upload'"
    echo "4. Push to GitHub: git push origin main"
}

# Main execution
main() {
    clear
    print_header
    
    # Confirm before proceeding
    echo -e "${YELLOW}⚠️  This will clean up the project for GitHub upload.${NC}"
    echo "The following actions will be performed:"
    echo "  • Remove build artifacts and temporary files"
    echo "  • Organize scripts, docs, and configs into subdirectories"  
    echo "  • Remove empty directories"
    echo "  • Update .gitignore and README.md"
    echo
    read -p "Continue? (y/N): " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Operation cancelled."
        exit 0
    fi
    
    echo
    
    # Perform cleanup operations
    clean_build_artifacts
    clean_temp_files
    clean_test_artifacts
    organize_scripts
    organize_documentation
    organize_configs
    remove_empty_dirs
    create_gitignore
    update_readme
    
    show_summary
}

# Execute main function
main "$@" 
