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
            capture_output=True,
            cwd=artifact,
            check=True,
        )

    @property
    def _examples(self):
        examples = [
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
            os.path.join(self._examples_dir, "status-message"),
            os.path.join(self._examples_dir, "test-contract"),
            os.path.join(self._examples_dir, "versioned"),
        ]
        return examples

    def build_artifacts(self):
        for example in self._examples:
            print("###############################", file=sys.stderr)
            print(f"Building `{example}` ...", file=sys.stderr)
            self._build_artifact(example)
            print(f"Finished Building `{example}` ...", file=sys.stderr)
            print("###############################", file=sys.stderr)

    def sizes(self):
        self.build_artifacts()

        # (not a meaningful comment to trigger stuck ci)
        print(f"finding result wasms in {self._examples_dir}", file=sys.stderr)

        artifact_paths1 = glob.glob(self._examples_dir + "/*/target/near/*.wasm")
        artifact_paths2 = glob.glob(self._examples_dir + "/*/target/near/*/*.wasm")

        artifact_paths = artifact_paths1 + artifact_paths2

        return {
            os.path.basename(path): os.stat(path).st_size for path in artifact_paths
        }
