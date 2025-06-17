# Matrixon Project Structure

This document describes the organized structure of the Matrixon project after cleanup and reorganization.

## 📁 Root Directory Structure

```
Matrixon/
├── 📄 Core Documentation
│   ├── README.md                    # Main project documentation
│   ├── CHANGELOG.md                 # Version history and changes
│   ├── CONTRIBUTING.md              # Contribution guidelines
│   ├── ARCHITECTURE_PRINCIPLES.md  # System architecture details
│   ├── LICENSE-APACHE              # Apache 2.0 license
│   └── LICENSE-MIT                 # MIT license
│
├── 🦀 Rust Project Files
│   ├── Cargo.toml                  # Main project manifest
│   ├── Cargo.lock                  # Dependency lock file
│   ├── rust-toolchain.toml         # Rust version specification
│   └── .cursorrules                # Development rules
│
├── 📂 Source Code
│   ├── src/                        # Main application source
│   ├── crates/                     # Workspace crates
│   ├── examples/                   # Usage examples
│   ├── tests/                      # Integration tests
│   └── benches/                    # Benchmarking code
│
├── 🐳 Development & Deployment
│   ├── deployment/
│   │   ├── docker/                 # Docker files
│   │   │   ├── docker-compose.yml
│   │   │   ├── docker-compose.production.yml
│   │   │   ├── Dockerfile
│   │   │   └── Dockerfile.local
│   │   ├── configs/                # Production configurations
│   │   │   ├── matrixon-production.toml
│   │   │   ├── production-config.toml
│   │   │   ├── matrixon-local.toml
│   │   │   ├── postgresql_simple.conf
│   │   │   ├── redis.conf
│   │   │   ├── prometheus.yml
│   │   │   └── nginx.conf
│   │   ├── nginx/                  # Nginx configuration
│   │   └── prometheus_rules/       # Monitoring rules
│   │
│   ├── monitoring/                 # Monitoring & observability
│   │   └── grafana/               # Grafana dashboards
│   │
│   └── scripts/                   # Deployment scripts
│       ├── deployment/            # Production deployment
│       └── test/                  # Testing scripts
│
├── 📚 Documentation
│   ├── docs/
│   │   ├── ROOM_COMMUNICATION_CURL_GUIDE.md
│   │   ├── FEATURES_DOCUMENTATION.md
│   │   ├── OPERATIONS_MAINTENANCE_GUIDE.md
│   │   └── OPERATION_MANUAL.md
│   │
├── 🗃️ Archives (Historical/Temporary)
│   ├── archives/
│   │   ├── old_configs/           # Old configuration files
│   │   ├── test_files/            # Test scripts and configs
│   │   ├── reports/               # Test reports
│   │   ├── temp_files/            # Temporary files
│   │   ├── logs/                  # Old log files
│   │   ├── backups/               # Database backups
│   │   ├── simple_matrixon/       # Simple test implementation
│   │   ├── simple_build/          # Simple build files
│   │   └── standalone_test/       # Standalone test files
│   │
├── 💾 Runtime Data
│   ├── data/                      # Application data
│   ├── media/                     # Media storage
│   ├── ssl/                       # SSL certificates
│   ├── ipfs/                      # IPFS storage
│   └── migrations/                # Database migrations
│
├── 🔧 Development Tools
│   ├── .github/                   # GitHub workflows
│   ├── .cargo/                    # Cargo configuration
│   ├── .sqlx/                     # SQLx metadata
│   ├── target/                    # Build artifacts
│   ├── docker/                    # Docker build context
│   ├── nix/                       # Nix build files
│   ├── debian/                    # Debian packaging
│   ├── complement/                # Matrix compliance tests
│   └── bin/                       # Binary files
│
└── 📝 Configuration Files
    ├── .gitignore                 # Git ignore rules
    ├── .dockerignore              # Docker ignore rules
    ├── .editorconfig              # Editor configuration
    ├── .envrc                     # Environment configuration
    ├── .gitlab-ci.yml             # GitLab CI configuration
    ├── default.nix                # Nix configuration
    ├── flake.nix                  # Nix flake configuration
    ├── flake.lock                 # Nix flake lock
    └── LICENSE                    # Additional license file
```

## 🎯 Organization Principles

### ✅ **Core Project Files** (Root Level)
- Essential documentation (README, CHANGELOG, etc.)
- Main Rust project files (Cargo.toml, src/, etc.)
- License files and project configuration

### 📂 **Organized by Function**
- **`deployment/`** - All deployment-related files (Docker, configs, nginx)
- **`docs/`** - Extended documentation and guides
- **`archives/`** - Historical files, old configs, test artifacts
- **`monitoring/`** - Observability and monitoring setup

### 🧹 **Cleaned Up**
- Removed duplicate configuration files
- Consolidated test files into archives
- Organized Docker files by environment
- Moved operational configs to deployment

## 🚀 Quick Navigation

### For Developers
- **Source Code**: `src/`, `crates/`, `examples/`
- **Documentation**: `README.md`, `docs/`, `ARCHITECTURE_PRINCIPLES.md`
- **Testing**: `tests/`, `archives/test_files/`

### For DevOps
- **Deployment**: `deployment/docker/`, `deployment/configs/`
- **Monitoring**: `monitoring/`, `deployment/prometheus_rules/`
- **Configuration**: `deployment/configs/`

### For Contributors
- **Guidelines**: `CONTRIBUTING.md`
- **Architecture**: `ARCHITECTURE_PRINCIPLES.md`
- **Examples**: `examples/`

## 📋 File Categories

### 🟢 **Keep in Root** (Essential)
- Main documentation files
- Rust project manifests
- License files
- Core configuration

### 🟡 **Organized in Subdirectories** (Functional)
- Deployment configurations
- Extended documentation
- Development tools
- Runtime data

### 🔴 **Archived** (Historical)
- Old test files
- Experimental configurations
- Legacy implementations
- Temporary artifacts

## 🔄 Maintenance

This structure should be maintained as follows:

1. **New Features**: Add to appropriate subdirectories
2. **Test Files**: Place in `archives/test_files/` if temporary
3. **Documentation**: Main docs in `docs/`, core docs in root
4. **Configuration**: Production configs in `deployment/configs/`
5. **Scripts**: Organized by purpose in `scripts/`

## ⚡ Quick Commands

```bash
# Start development
cargo run

# Run tests  
cargo test

# Build for production
cargo build --release

# Start with Docker
cd deployment/docker && docker-compose up

# View documentation
ls docs/

# Check archives
ls archives/
```

---

*This structure was organized on 2024-12-XX to improve project maintainability and clarity.* 
