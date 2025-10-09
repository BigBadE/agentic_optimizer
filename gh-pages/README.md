# Merlin Benchmark Dashboard

This directory contains the web-based benchmark tracking dashboard for the Merlin project.

## Structure

- `index.html` - Main dashboard page
- `assets/dashboard.js` - Dashboard JavaScript
- `data/` - Benchmark data in JSON format
  - `quality-latest.json` - Latest quality benchmark results
  - `perf-latest.json` - Latest performance benchmark results
  - `gungraun-latest.json` - Latest gungraun benchmark results
- `quality-bench/` - Historical quality benchmark results
- `perf-bench/` - Historical performance benchmark results
- `gungraun-bench/` - Historical gungraun benchmark results

## Viewing the Dashboard

Visit: https://YOUR_USERNAME.github.io/agentic_optimizer/

## Updating Data

The dashboard automatically updates when new benchmark results are pushed to the gh-pages branch by CI workflows.

## Local Development

To test locally:
```bash
cd gh-pages
python3 -m http.server 8000
# Visit http://localhost:8000
```
