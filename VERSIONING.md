# Versioning Strategy

This document explains the versioning strategy for the `scim-server` library during its active development phase.

## ⚠️ Development Phase (Current)

**Current Status**: Under active development until version 0.9.0

### Breaking Changes Policy

- **Breaking changes** are signaled by **minor version increments** (0.2.0 → 0.3.0 → 0.4.0)
- **Patch releases** (0.2.1 → 0.2.2 → 0.2.3) contain **only non-breaking changes**
- **Pre-release versions** (0.3.0-alpha, 0.3.0-beta) may contain breaking changes

### Recommended Version Pinning

```toml
[dependencies]
# ✅ Recommended: Pin to exact version
scim-server = "=0.2.3"

# ❌ Not recommended during development phase
scim-server = "0.2.3"  # Allows patch updates
scim-server = "^0.2.3" # Allows minor updates (breaking changes!)
```

## 🎯 API Stabilization (Future)

### Version 0.9.0: API Freeze
- **Final breaking changes** before stabilization
- **API locked** for 1.0.0 preparation
- **Release candidate** period for ecosystem testing

### Version 1.0.0: Stable Release
- **Semantic versioning** (semver) compliance begins
- **Breaking changes** only in major versions (1.0.0 → 2.0.0)
- **Minor versions** (1.1.0, 1.2.0) are additive only
- **Patch versions** (1.0.1, 1.0.2) are bug fixes only

## 📊 Current Version Lifecycle

| Version Range | Status | Breaking Changes | Recommended Use |
|---------------|--------|------------------|-----------------|
| 0.1.x | Legacy | N/A | Migrate to 0.2.x |
| 0.2.x | Current Stable | Patch only | ✅ Pin exact version |
| 0.3.x | Next Major | Expected Q2 2025 | 🔄 Migration planning |
| 0.4.x+ | Future | TBD | ⏳ Follow roadmap |

## 🛡️ Stability Guarantees

### What We Guarantee
- **Patch releases** never break existing code
- **Breaking changes** are clearly documented
- **Migration guides** provided for major changes
- **Deprecation warnings** before removal (when possible)

### What We Don't Guarantee (Until 1.0.0)
- **Minor version compatibility** - 0.3.0 may break 0.2.x code
- **API stability** - Method signatures may change
- **Feature completeness** - APIs may be removed or redesigned

## 📋 Version Selection Guide

### For Learning/Experimentation
```toml
# Latest version for newest features
scim-server = "0.2.3"
```

### For Development Projects
```toml
# Pin exact version, plan for updates
scim-server = "=0.2.3"
```

### For Production/Critical Systems
```toml
# Pin exact version, test updates thoroughly
scim-server = "=0.2.3"

# Consider using git dependency for maximum control
scim-server = { git = "https://github.com/pukeko37/scim-server", tag = "v0.2.3" }
```

### For Library Authors
```toml
# Support range of compatible versions
scim-server = ">=0.2.3, <0.3.0"
```

## 🔄 Migration Strategy

### Stay Current
1. **Monitor releases** via GitHub notifications
2. **Read CHANGELOG.md** for breaking changes
3. **Test migration** in development environment
4. **Update incrementally** (0.2.x → 0.3.x → 0.4.x)

### Plan for Stability
1. **Target 0.9.x** for pre-stable testing
2. **Migrate to 1.0.0** when released
3. **Enjoy semver** compatibility thereafter

## 📈 Release Schedule

### Current Pattern (0.x Era)
- **Major releases** (breaking): Every 2-3 months
- **Minor releases** (features): Every 3-4 weeks  
- **Patch releases** (fixes): As needed

### Post-1.0 Pattern (Stable Era)
- **Major releases** (breaking): Yearly or less
- **Minor releases** (features): Every 1-2 months
- **Patch releases** (fixes): As needed

## 🤝 Community Communication

### Release Announcements
- **GitHub Releases** - Full release notes
- **CHANGELOG.md** - Detailed change documentation
- **Migration Guides** - Step-by-step upgrade instructions
- **Discord/Discussions** - Community support

### Breaking Change Process
1. **Proposal** - RFC or GitHub issue
2. **Community feedback** - Discussion period
3. **Implementation** - Alpha/beta releases
4. **Documentation** - Migration guides
5. **Release** - Stable version with breaking changes

## 💡 Best Practices

### For Application Developers
```toml
# Pin exact versions during development phase
scim-server = "=0.2.3"

# Monitor for updates
# cargo update --dry-run
```

### For CI/CD
```yaml
# Ensure reproducible builds
- name: Install dependencies
  run: cargo build --locked
```

### For Docker
```dockerfile
# Pin Rust version and dependencies
FROM rust:1.75 as builder
COPY Cargo.lock .
COPY Cargo.toml .
RUN cargo build --locked
```

## 🔮 Future Vision

### Stability Timeline
- **2025 Q2**: Version 0.3.0 (Storage provider architecture)
- **2025 Q3**: Version 0.4.0 (HTTP framework integration)
- **2025 Q4**: Version 0.5.0 (Database providers)
- **2026 Q1**: Version 0.9.0 (API freeze)
- **2026 Q2**: Version 1.0.0 (Stable release)

### Long-term Commitment
- **Semantic versioning** compliance post-1.0
- **Long-term support** for stable versions
- **Ecosystem stability** for the Rust SCIM community
- **Enterprise-grade** reliability and support

---

**Questions?** Join our [GitHub Discussions](https://github.com/pukeko37/scim-server/discussions) or check the [Migration Guides](docs/guides/) for version-specific help.

The development phase ensures we build the best possible API before committing to long-term stability. Your feedback during this phase helps shape the final stable API.