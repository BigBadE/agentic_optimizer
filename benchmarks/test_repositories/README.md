# Test Repositories

This directory contains external projects used for benchmarking context fetching.

## Why External Repositories?

Using external projects for benchmarking provides:

1. **Stability**: Test results aren't affected by changes to this project's structure
2. **Realism**: Tests run against real-world codebases
3. **Reproducibility**: Pinned commits ensure consistent benchmarks over time
4. **Objectivity**: Can't accidentally optimize for our own codebase structure

## Current Test Repositories

### Valor Browser Engine

**Repository**: https://github.com/BigBadE/Valor  
**Pinned Commit**: `367ecde76cfe1a587256f9c6f318a56afee5ac17`  
**Date**: 2025-10-02 12:26:10 -0500  
**Description**: A browser engine written in Rust with CSS, HTML, and rendering components

**Why Valor?**
- Multi-crate workspace (similar to real projects)
- Clear domain separation (CSS, HTML, rendering, JS)
- Good variety of file types and sizes
- Active development with realistic code patterns

## Setup

The repositories are cloned automatically when needed, but you can manually set them up:

```bash
# Clone Valor
git clone https://github.com/BigBadE/Valor.git benchmarks/test_repositories/valor

# Pin to specific commit
cd benchmarks/test_repositories/valor
git reset --hard 367ecde76cfe1a587256f9c6f318a56afee5ac17
```

## Adding New Test Repositories

1. **Clone the repository**:
   ```bash
   git clone <repo-url> benchmarks/test_repositories/<name>
   ```

2. **Pin to a specific commit**:
   ```bash
   cd benchmarks/test_repositories/<name>
   git reset --hard <commit-hash>
   ```

3. **Document the pin**:
   Create a `.git-pin` file with:
   ```
   Repository: <url>
   Commit: <hash>
   Date: <date>
   Message: <commit message>
   ```

4. **Create test cases**:
   Add `.toml` files in `benchmarks/test_cases/<name>/`

5. **Update this README** with the new repository info

## Maintenance

### Updating Pinned Commits

When updating to a newer commit:

1. Test that benchmarks still work
2. Update the `.git-pin` file
3. Re-run all benchmarks to establish new baseline
4. Document any significant changes in test expectations

### Cleaning Up

To save disk space, you can remove the `.git` directory after cloning:

```bash
rm -rf benchmarks/test_repositories/valor/.git
```

However, this prevents easy updates. Only do this if disk space is critical.

## .gitignore

Test repositories are gitignored to avoid bloating this repository. They're cloned on-demand or during setup.
