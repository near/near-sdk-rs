import os
import glob
import subprocess
import tempfile
import sys
from contextlib import contextmanager
from git import Repo


class ProjectInstance:
    def __init__(self, root_dir):
        self._root_dir = root_dir

    @contextmanager
    def branch(self, branch):
        repo = Repo(self._root_dir)

        try:
            with tempfile.TemporaryDirectory() as tempdir:
                repo.git.worktree("add", tempdir, branch)
                branch_project = ProjectInstance(tempdir)

                yield branch_project
        finally:
            repo.git.worktree("prune")

    @property
    def _examples_dir(self):
        return os.path.join(self._root_dir, "examples")

    def _build_artifact(self, artifact):
        subprocess.run(
            [
                "cargo",
                "near",
                "build",
                "reproducible-wasm",
                "--no-locked",
            ],
            cwd=os.path.join(artifact),
            check=True,
        )

    @property
    def _examples(self):
        # build "status-message" first, as it's a dependency of some other examples
        examples = [
            os.path.join(self._examples_dir, "status-message"),
            os.path.join(self._examples_dir, "adder"),
            os.path.join(self._examples_dir, "callback-results"),
            os.path.join(self._examples_dir, "cross-contract-calls", "high-level"),
            os.path.join(self._examples_dir, "cross-contract-calls", "low-level"),
            os.path.join(self._examples_dir, "factory-contract", "high-level"),
            os.path.join(self._examples_dir, "factory-contract", "low-level"),
            os.path.join(self._examples_dir, "fungible-token", "ft"),
            os.path.join(self._examples_dir, "fungible-token", "test-contract-defi"),
            os.path.join(self._examples_dir, "lockable-fungible-token"),
            os.path.join(self._examples_dir, "mission-control"),
            os.path.join(self._examples_dir, "mpc-contract"),
            os.path.join(self._examples_dir, "non-fungible-token", "nft"),
            os.path.join(self._examples_dir, "non-fungible-token", "test-approval-receiver"),
            os.path.join(self._examples_dir, "non-fungible-token", "test-token-receiver"),
            os.path.join(self._examples_dir, "test-contract"),
            os.path.join(self._examples_dir, "versioned"),
        ]
        return examples

    def build_artifacts(self, cache):
        for example in self._examples:
            print(f"Building {example}...", file=sys.stderr)
            self._build_artifact(example)

    def sizes(self, cache):
        self.build_artifacts(cache)

        artifact_paths = glob.glob(self._examples_dir + "/*/res/*.wasm")
        return {
            os.path.basename(path): os.stat(path).st_size for path in artifact_paths
        }
