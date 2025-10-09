# 🚀 Benchmark Dashboard Setup Guide

This guide will help you set up the web-based benchmark tracking dashboard for the Merlin project.

## Overview

The benchmark dashboard provides:
- **Real-time visualization** of all three benchmark types (Quality, Performance, Gungraun)
- **Historical trend tracking** with interactive charts
- **Automatic updates** from CI workflows
- **Comparison metrics** showing changes over time

## Quick Setup

### 1. Initialize gh-pages Branch

```bash
# Create and switch to gh-pages branch
git checkout --orphan gh-pages

# Remove all files from working directory
git rm -rf .

# Generate the dashboard structure directly
bash scripts/generate-benchmark-dashboard.sh .

# Commit and push
git add .
git commit -m "Initialize benchmark dashboard"
git push -u origin gh-pages

# Switch back to master
git checkout master
```

This creates directly on the gh-pages branch:
- `index.html` - Main dashboard page
- `assets/dashboard.js` - Interactive JavaScript
- `data/` - JSON data files (will be populated by CI)
- Sample data for testing

### 3. Enable GitHub Pages

1. Go to your repository on GitHub
2. Click **Settings** → **Pages**
3. Under **Source**, select:
   - Branch: `gh-pages`
   - Folder: `/ (root)`
4. Click **Save**

Your dashboard will be available at:
```
https://YOUR_USERNAME.github.io/agentic_optimizer/
```

## How It Works

### Data Flow

```
┌─────────────────┐
│ CI Workflows    │
│ - Quality       │
│ - Performance   │
│ - Gungraun      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ parse-          │
│ benchmarks.py   │
│ (Extracts data) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ JSON Files      │
│ - quality.json  │
│ - perf.json     │
│ - gungraun.json │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Web Dashboard   │
│ (Visualizes)    │
└─────────────────┘
```


The CI workflows automatically:
1. Run benchmarks on every push to master
2. Parse results into JSON format
3. Update historical data
4. Push to gh-pages

### 2. Updating Dashboard Data

The dashboard automatically updates when benchmarks run in CI. To manually update:

```bash
# Run benchmarks locally
cargo bench --workspace

# Parse results and generate JSON
python3 scripts/parse-benchmarks.py \
  --criterion-dir target/criterion \
  --gungraun-output gungraun-results.md \
  --quality-results quality-results.md \
  --output-dir /tmp/benchmark-data

# Switch to gh-pages branch and update
git checkout gh-pages
mkdir -p data
cp /tmp/benchmark-data/*.json data/
git add data/
git commit -m "Update benchmark data"
git push origin gh-pages
git checkout master
## Dashboard Features

### 📊 Stats Grid
- Quick overview of key metrics
- Change indicators (↑ improvement, ↓ regression)
- Color-coded performance (green = good, red = bad)

### 📈 Performance Trends
- Interactive charts with Chart.js
- Historical data (last 30 runs)
- Tabbed interface for each benchmark type

### 📋 Detailed Tables
- Current vs. previous comparison
- Percentage change calculations
- Sortable columns

### 🔄 Auto-Refresh
- Dashboard checks for updates every 5 minutes
- Seamless data loading
- No page refresh needed

## Customization

### Change History Retention

Edit `scripts/parse-benchmarks.py`:

```python
def merge_with_history(current, history_file, max_history=30):  # Change 30 to desired number
```

### Modify Dashboard Appearance

Edit `gh-pages/index.html` and `gh-pages/assets/dashboard.js`:
- Colors: Search for hex codes (e.g., `#667eea`)
- Chart types: Modify Chart.js configuration
- Metrics displayed: Update stats grid generation

### Add New Metrics

1. Update `scripts/parse-benchmarks.py` to extract new metrics
2. Modify `dashboard.js` to display them
3. Update chart configurations as needed

## Troubleshooting

### Dashboard shows "No data available"

**Solution:**
```bash
# Check if JSON files exist
ls gh-pages/data/

# If missing, generate sample data
bash scripts/generate-benchmark-dashboard.sh gh-pages

# Or run benchmarks and parse
cargo bench --workspace
python3 scripts/parse-benchmarks.py
```

### Charts not rendering

**Solution:**
- Check browser console for errors
- Verify Chart.js CDN is accessible
- Ensure JSON data is valid:
  ```bash
  python3 -m json.tool gh-pages/data/quality-latest.json
  ```

### CI not updating dashboard

**Solution:**
1. Check workflow runs in GitHub Actions
2. Verify gh-pages branch exists
3. Check workflow permissions (needs `contents: write`)
4. Review parse-benchmarks.py output in CI logs

### Historical data not showing

**Solution:**
```bash
# Check history files
ls .benchmark-history/

# Verify history is being saved
cat .benchmark-history/quality-history.json
```

## File Structure

```
agentic_optimizer/
├── scripts/
│   ├── generate-benchmark-dashboard.sh  # Dashboard generator
│   ├── parse-benchmarks.py              # Data parser
│   └── benchmark-tracker.sh             # CLI tracker
├── .benchmark-history/                  # Historical data (gitignored)
│   ├── quality-history.json
│   ├── perf-history.json
│   └── gungraun-history.json
└── gh-pages/                            # Dashboard files (separate branch)
    ├── index.html                       # Main page
    ├── assets/
    │   └── dashboard.js                 # JavaScript
    ├── data/                            # JSON data
    │   ├── quality-latest.json
    │   ├── perf-latest.json
    │   └── gungraun-latest.json
    ├── quality-bench/                   # Raw results
    ├── perf-bench/
    └── gungraun-bench/
```

## Next Steps

1. **Run Initial Benchmarks**
   ```bash
   cargo bench --workspace
   ```

2. **Generate Dashboard**
   ```bash
   bash scripts/generate-benchmark-dashboard.sh gh-pages
   ```

3. **Deploy to gh-pages**
   ```bash
   # Follow "Initialize gh-pages Branch" steps above
   ```

4. **Enable GitHub Pages**
   - Settings → Pages → Source: gh-pages

5. **View Your Dashboard**
   - Visit: https://YOUR_USERNAME.github.io/agentic_optimizer/

## Resources

- **Documentation**: `docs/BENCHMARKING.md`
- **Criterion.rs**: https://github.com/bheisler/criterion.rs
- **Chart.js**: https://www.chartjs.org/
- **GitHub Pages**: https://pages.github.com/

## Support

For issues or questions:
1. Check the troubleshooting section above
2. Review CI workflow logs
3. Inspect browser console for errors
4. Verify JSON data format

---

**Generated by Merlin Benchmark Tracker**
