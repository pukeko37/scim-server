# Documentation Build Process

## Overview

The SCIM Server project uses [mdBook](https://rust-lang.github.io/mdBook/) to generate HTML documentation from Markdown source files. This document outlines the build process, release workflow, and maintenance procedures.

## Directory Structure

```
docs/
├── guide/
│   ├── book.toml          # mdBook configuration
│   ├── src/               # Source Markdown files (COMMIT THESE)
│   │   ├── SUMMARY.md     # Table of contents
│   │   ├── introduction.md
│   │   ├── getting-started/
│   │   ├── concepts/
│   │   ├── providers/
│   │   └── ...
│   └── book/              # Generated HTML files (COMMIT FOR RELEASES)
│       ├── index.html
│       ├── getting-started/
│       └── ...
└── README.md
```

## Build Workflow

### Development Process

1. **Edit source files** in `docs/guide/src/`
2. **Test locally** with `mdbook serve` for live preview
3. **Commit source changes** only (`.md` files)
4. **Do NOT commit** generated HTML during development

### Release Process

1. **Run the build script**: `./scripts/build-docs.sh`
2. **Review generated documentation** in browser
3. **Commit generated HTML** for release
4. **Include in release PR/commit**

## Scripts

### `scripts/build-docs.sh`

Automated documentation build script that:
- Checks for mdBook installation
- Cleans previous builds
- Generates fresh HTML
- Validates output
- Provides next-step guidance

Usage:
```bash
./scripts/build-docs.sh
```

### Manual Build

For manual builds:
```bash
cd docs/guide
mdbook clean      # Optional: clean previous build
mdbook build      # Generate HTML
mdbook serve      # Preview locally (optional)
```

## Git Workflow

### During Development
```bash
# Edit documentation
vim docs/guide/src/getting-started/first-server.md

# Test locally
cd docs/guide && mdbook serve

# Commit only source files
git add docs/guide/src/
git commit -m "Update getting started guide"
```

### For Releases
```bash
# Build documentation
./scripts/build-docs.sh

# Commit generated files
git add docs/guide/book/
git commit -m "Update documentation for release v0.3.2"

# Include in release
git tag v0.3.2
git push origin v0.3.2
```

## Why Commit Generated HTML?

We commit generated HTML files for releases because:

1. **Direct GitHub Access**: Users can browse documentation directly on GitHub without installing mdBook
2. **Release Artifacts**: Documentation is preserved as part of each release
3. **Offline Access**: Cloned repositories include readable documentation
4. **README Links**: README.md can link directly to HTML files

## File Management

### Source Files (`docs/guide/src/`)
- **Always commit** changes to `.md` files
- These are the authoritative documentation source
- Edit these files for content changes

### Generated Files (`docs/guide/book/`)
- **Commit only for releases** 
- Do not commit during development iterations
- Regenerate fresh for each release
- These files can become large and create noise in diffs

## Quality Assurance

### Before Release

1. **Content Review**: Ensure all documentation accurately reflects current API
2. **Link Validation**: Check that internal links work correctly
3. **Code Examples**: Verify all code examples compile and run
4. **Fresh Build**: Always do a clean rebuild for releases

### Validation Checklist

- [ ] Run `./scripts/build-docs.sh` successfully
- [ ] Review main sections in browser
- [ ] Test example code snippets
- [ ] Check README.md links point to correct locations
- [ ] Verify no broken internal links

## Integration Points

### README.md
Links to documentation use `docs/guide/book/` path for direct HTML access:
```markdown
📖 **[User Guide](docs/guide/book/)** | Comprehensive tutorials and concepts
```

### Crates.io
The `docs.rs` badge points to API documentation:
```markdown
[![Documentation](https://docs.rs/scim-server/badge.svg)](https://docs.rs/scim-server)
```

### GitHub Pages (Future)
Consider setting up GitHub Pages for professional documentation hosting:
- Automatic builds on push
- Custom domain support
- Better SEO and discoverability

## Troubleshooting

### mdBook Not Found
```bash
cargo install mdbook
```

### Build Failures
1. Check for syntax errors in Markdown files
2. Verify `SUMMARY.md` references all files correctly
3. Ensure file paths are correct and case-sensitive

### Missing Files
If generated files are missing:
1. Run `mdbook clean` to clear corrupted state
2. Run `./scripts/build-docs.sh` for fresh build
3. Check file permissions

### Large Diffs
Generated HTML files can create large diffs:
- This is normal for documentation updates
- Focus review on source `.md` file changes
- Generated files are mechanical transformations

## Best Practices

1. **Source First**: Always edit `.md` files, never HTML directly
2. **Clean Builds**: Use clean builds for releases to avoid artifacts
3. **Test Locally**: Preview changes with `mdbook serve` before committing
4. **Batch Updates**: Group related documentation changes in single commits
5. **Release Timing**: Regenerate documentation as final step before release

## Future Improvements

1. **CI/CD Integration**: Automate documentation builds in GitHub Actions
2. **Link Checking**: Add automated link validation
3. **GitHub Pages**: Set up automated deployment to GitHub Pages
4. **Version Management**: Consider versioned documentation for major releases
5. **Search Integration**: Enhanced search capabilities for larger documentation sets