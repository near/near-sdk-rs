# Compare example contract sizes

This is a script to compare example contract sizes between the current non-master branch and `master`, and then produce a markdown report.

# Usage

The script is mostly triggered in PRs by posting `/compare` in a comment. For details, check out [the workflow](../../.github/workflows/compare_sizes.yml).

It's also possible to test it locally like so:

```bash
# from the root dir
pip install GitPython
ci/compare_sizes/compare_sizes.py
```

