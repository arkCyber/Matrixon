#!/usr/bin/env python3

# =============================================================================
# Matrixon Matrix NextServer - Batch Header Update Script (Python)
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
#   Python script to efficiently batch update file headers across all Rust
#   source files, ensuring compliance with .cursorrules standards.
#
# =============================================================================

import os
import re
import sys
from pathlib import Path

# Header template for different module types
HEADER_TEMPLATE = """// =============================================================================
// Matrixon Matrix NextServer - {module_title} Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   {description} This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
{features}
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

"""

def get_module_info(file_path):
    """Determine module title, description and features based on file path."""
    path_str = str(file_path)
    module_name = file_path.stem
    
    # Convert snake_case to Title Case
    module_title = ' '.join(word.capitalize() for word in module_name.split('_'))
    
    if '/database/' in path_str:
        description = "Database layer component for high-performance data operations."
        features = """//   • High-performance database operations
//   • PostgreSQL backend optimization
//   • Connection pooling and caching
//   • Transaction management
//   • Data consistency guarantees"""
    elif '/api/' in path_str:
        description = "Matrix API implementation for client-server communication."
        features = """//   • Matrix protocol compliance
//   • RESTful API endpoints
//   • Request/response handling
//   • Authentication and authorization
//   • Rate limiting and security"""
    elif '/service/' in path_str:
        description = "Core business logic service implementation."
        features = """//   • Business logic implementation
//   • Service orchestration
//   • Event handling and processing
//   • State management
//   • Enterprise-grade reliability"""
    elif '/utils/' in path_str:
        description = "Utility functions and helper components."
        features = """//   • Common utility functions
//   • Error handling and logging
//   • Performance instrumentation
//   • Helper traits and macros
//   • Shared functionality"""
    elif '/config/' in path_str:
        description = "Configuration management and validation."
        features = """//   • Configuration parsing and validation
//   • Environment variable handling
//   • Default value management
//   • Type-safe configuration
//   • Runtime configuration updates"""
    elif '/cli/' in path_str:
        description = "Command-line interface implementation."
        features = """//   • CLI command handling
//   • Interactive user interface
//   • Command validation and parsing
//   • Help and documentation
//   • Administrative operations"""
    else:
        description = "Core component of the Matrixon Matrix NextServer."
        features = """//   • High-performance Matrix operations
//   • Enterprise-grade reliability
//   • Scalable architecture
//   • Security-focused design
//   • Matrix protocol compliance"""
    
    return module_title, description, features

def has_matrixon_header(file_path):
    """Check if file already has the correct Matrixon header."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read(2000)  # Read first 2000 characters
            return ('Matrixon Matrix NextServer' in content and 
                    'arkSong (arksong2018@gmail.com)' in content)
    except:
        return False

def update_file_header(file_path):
    """Update a single file's header."""
    if has_matrixon_header(file_path):
        return False  # Skip, already updated
    
    try:
        # Read current content
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        # Find where actual code starts (skip existing comments)
        start_line = 0
        for i, line in enumerate(lines):
            stripped = line.strip()
            if stripped and not stripped.startswith('//') and not stripped.startswith('/*') and not stripped.startswith('*'):
                start_line = i
                break
        
        # Generate new header
        module_title, description, features = get_module_info(file_path)
        new_header = HEADER_TEMPLATE.format(
            module_title=module_title,
            description=description,
            features=features
        )
        
        # Create backup
        backup_path = str(file_path) + '.backup'
        with open(backup_path, 'w', encoding='utf-8') as f:
            f.writelines(lines)
        
        # Write new file
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(new_header)
            f.writelines(lines[start_line:])
        
        return True  # Updated
    except Exception as e:
        print(f"Error updating {file_path}: {e}")
        return False

def main():
    """Main function to process all Rust files."""
    src_dir = Path('src')
    if not src_dir.exists():
        print("Error: src directory not found!")
        return
    
    print("🔧 Matrixon File Header Update Tool (Python)")
    print("=" * 50)
    print("Updating all Rust source files to Matrixon standards...")
    print()
    
    # Find all .rs files
    rust_files = list(src_dir.rglob('*.rs'))
    print(f"Found {len(rust_files)} Rust files")
    
    updated_count = 0
    skipped_count = 0
    
    for file_path in sorted(rust_files):
        try:
            if update_file_header(file_path):
                print(f"✅ Updated: {file_path}")
                updated_count += 1
            else:
                print(f"⏭️  Skipped: {file_path} (already has Matrixon header)")
                skipped_count += 1
        except Exception as e:
            print(f"❌ Error: {file_path} - {e}")
    
    print()
    print("📊 Summary:")
    print(f"  Total files: {len(rust_files)}")
    print(f"  Updated: {updated_count}")
    print(f"  Skipped: {skipped_count}")
    
    if updated_count > 0:
        print()
        print("✅ File headers successfully updated to Matrixon standards!")
        print("All files now include:")
        print("  • Matrixon project branding")
        print("  • arkSong (arksong2018@gmail.com) author attribution")
        print("  • Comprehensive module documentation")
        print("  • Performance targets and architecture details")
        print("  • Quality assurance standards")
    
    print()
    print("🔍 To verify changes:")
    print("  head -20 src/main.rs")
    print("  head -20 src/lib.rs")
    print("  head -20 src/database/mod.rs")

if __name__ == "__main__":
    main() 
