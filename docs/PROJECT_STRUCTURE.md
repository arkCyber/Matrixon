# Matrixon Project Structure

**Matrixon Team** - Next-Generation Communication Platform for AI & Web3  
**Contact**: arksong2018@gmail.com

This document provides a comprehensive overview of the Matrixon project structure and organization.

## 📁 Root Directory Structure

```
Matrixon/
├── 📄 README.md                    # Project overview and quick start
├── 📄 LICENSE                      # Apache 2.0 / MIT dual license
├── 📄 Cargo.toml                   # Rust package configuration
├── 📄 Cargo.lock                   # Dependency lock file
├── 📄 .gitignore                   # Git ignore rules
├── 📄 .cursorrules                 # Development guidelines
├── 📄 .gitlab-ci.yml               # CI/CD pipeline configuration
│
├── 📂 src/                         # Main source code
├── 📂 docs/                        # Documentation files
├── 📂 configs/                     # Configuration files
├── 📂 scripts/                     # Utility scripts
├── 📂 tests/                       # Test files
├── 📂 examples/                    # Usage examples
├── 📂 migrations/                  # Database migrations
│
├── 📂 docker/                      # Docker configurations
├── 📂 debian/                      # Debian packaging
├── 📂 nix/                         # Nix packaging
│
├── 📂 monitoring/                  # Monitoring configurations
├── 📂 grafana/                     # Grafana dashboards
├── 📂 logs/                        # Log files
├── 📂 backups/                     # Backup scripts and data
│
└── 📂 target/                      # Rust build artifacts (git-ignored)
```

## 🦀 Source Code Structure (`src/`)

```
src/
├── main.rs                         # Application entry point
├── lib.rs                          # Library root
├── config/                         # Configuration management
├── database/                       # Database abstraction layer
├── service/                        # Business logic services
├── api/                            # API endpoint implementations
├── cli/                            # Command-line interface
└── utils/                          # Utility functions
```

### Core Modules

#### `src/database/` - Database Layer
- **PostgreSQL**: Production database support
- **SQLite**: Development and testing
- **RocksDB**: High-performance scenarios
- **key_value/**: Key-value storage implementations
- **migrations/**: Database schema migrations

#### `src/service/` - Business Logic
- **users/**: User management and authentication
- **rooms/**: Room creation and management  
- **federation/**: Server-to-server communication
- **media/**: File upload and storage
- **i18n/**: Internationalization support
- **ai/**: AI integration services
- **web3/**: Blockchain integration

#### `src/api/` - API Endpoints
- **client/**: Matrix Client-Server API
- **federation/**: Matrix Server-Server API
- **admin/**: Administration API
- **health/**: Health check endpoints

## 📖 Documentation Structure (`docs/`)

```
docs/
├── SYSTEM_INTRODUCTION.md          # Marketing and feature overview
├── FEATURES_DOCUMENTATION.md       # Detailed feature specifications
├── OPERATION_MANUAL.md             # Installation and setup guide
├── OPERATIONS_MAINTENANCE_GUIDE.md # Production operations guide
├── ARCHITECTURE_PRINCIPLES.md      # Technical architecture documentation
├── PROJECT_STRUCTURE.md            # This file
│
├── SETUP_COMPLETE.md               # Complete setup instructions
├── DATABASE_CONFIGURATION_SUMMARY.md # Database setup guide
├── PERFORMANCE_RECOMMENDATIONS_200K.md # Performance tuning
├── FEDERATION_IMPROVEMENTS.md      # Federation setup
├── AI_CONTENT_MODERATION_SUMMARY.md # AI feature documentation
│
└── [Additional technical documentation...]
```

## ⚙️ Configuration Structure (`configs/`)

```
configs/
├── matrixon.toml                   # Main server configuration
├── postgresql_config.toml          # PostgreSQL database config
├── sqlite_config.toml              # SQLite database config
├── production.env                  # Production environment variables
│
├── prometheus.yml                  # Monitoring configuration
├── alertmanager.yml                # Alert management
├── nginx.conf                      # Reverse proxy configuration
├── redis.conf                      # Cache configuration
│
└── [Additional service configurations...]
```

## 🛠️ Scripts Structure (`scripts/`)

```
scripts/
├── ssl-setup.sh                    # SSL certificate setup
├── start-monitoring.sh             # Start monitoring stack
├── log-monitoring.sh               # Log monitoring utilities
├── rolling-update.sh               # Rolling deployment script
├── automated-backup.sh             # Backup automation
├── firewall-setup.sh               # Security configuration
├── connection-pool-monitor.sh      # Connection monitoring
│
└── [Additional operational scripts...]
```

## 🧪 Testing Structure (`tests/`)

```
tests/
├── integration/                    # Integration tests
├── unit/                          # Unit tests
├── performance/                   # Performance benchmarks
├── compliance/                    # Matrix protocol compliance
└── e2e/                          # End-to-end tests
```

## 🐳 Deployment Structure

### Docker Configuration (`docker/`)
```
docker/
├── Dockerfile                     # Multi-stage build
├── docker-compose.yml             # Development setup
├── docker-compose.200k.yml        # High-performance setup
├── docker-compose.postgresql.yml  # PostgreSQL setup
└── [Environment-specific configs...]
```

### Kubernetes Configuration
```
kubernetes/
├── deployment.yaml                # Application deployment
├── service.yaml                   # Service definition
├── configmap.yaml                 # Configuration management
├── ingress.yaml                   # Ingress rules
└── monitoring/                    # Monitoring stack
```

## 📊 Monitoring Structure (`monitoring/`)

```
monitoring/
├── prometheus/                    # Prometheus configuration
├── grafana/                      # Grafana dashboards
├── alertmanager/                 # Alert management
├── loki/                         # Log aggregation
└── jaeger/                       # Distributed tracing
```

## 🔧 Development Tools

### IDE Configuration (`.vscode/`)
```
.vscode/
├── settings.json                  # VS Code settings
├── launch.json                    # Debug configurations
├── tasks.json                     # Build tasks
└── extensions.json                # Recommended extensions
```

### Package Management
```
nix/                              # Nix package definitions
debian/                           # Debian packaging files
flake.nix                         # Nix flake configuration
default.nix                       # Default Nix expression
```

## 🚀 Build and Deployment

### Build Artifacts (`target/`)
```
target/
├── debug/                        # Debug builds
├── release/                      # Release builds
├── doc/                         # Generated documentation
└── [Cargo build artifacts...]
```

### Continuous Integration
- **GitLab CI**: `.gitlab-ci.yml`
- **GitHub Actions**: `.github/workflows/`
- **Docker**: Multi-stage builds with optimization
- **Testing**: Automated test suites
- **Security**: Vulnerability scanning

## 📦 Dependencies

### Core Dependencies
- **Tokio**: Async runtime
- **Axum**: Web framework
- **SQLx**: Database abstraction
- **Serde**: Serialization
- **Tracing**: Structured logging
- **Ruma**: Matrix protocol types

### AI & Web3 Dependencies
- **OpenAI**: AI integration
- **Web3**: Blockchain connectivity
- **Ethereum**: Smart contract interaction
- **IPFS**: Decentralized storage

## 🔒 Security Considerations

### File Permissions
- Configuration files: `600` (owner read/write only)
- Scripts: `755` (owner execute, group/other read)
- Data directories: `700` (owner access only)

### Sensitive Data
- Private keys: Stored in secure configuration
- Database credentials: Environment variables
- API tokens: Configuration files (not in git)

## 📝 Development Workflow

1. **Code Organization**: Follow Rust conventions
2. **Documentation**: Comprehensive inline docs
3. **Testing**: Test-driven development
4. **Security**: Regular security audits
5. **Performance**: Continuous benchmarking

## 🎯 Future Expansion

### Planned Additions
- `plugins/`: Plugin system architecture
- `ai/`: Enhanced AI capabilities
- `web3/`: Advanced blockchain features
- `mobile/`: Mobile app support
- `sdk/`: Client SDK development

---

This structure ensures:
- ✅ **Clean separation** of concerns
- ✅ **Scalable architecture** for growth
- ✅ **Easy navigation** for developers
- ✅ **Production readiness** with proper configurations
- ✅ **Comprehensive documentation** for all components

**Matrixon Team** - Building the future of AI & Web3 communication  
**Contact**: arksong2018@gmail.com 
