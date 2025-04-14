import docker
import os
import glob
import subprocess
import tempfile
import shutil
import sys
import platform
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

    def _build_artifact(self, artifact, cache):
        subprocess.run(
            [
                "cargo",
                "near",
                "build",
                "reproducible-wasm",
                "--no-locked",
            ],
            cwd=os.path.join(artifact.path),
            check=True,
            env={
                **os.environ,  # Include the existing environment variables
                "RUSTFLAGS": "-C link-arg=-s",
                "CARGO_TARGET_DIR": f"{self._root_dir}/target",
            },
        )

    @property
    def _examples(self):
        examples = filter(os.DirEntry.is_dir, os.scandir(self._examples_dir))

        # build "status-message" first, as it's a dependency of some other examples
        return sorted(examples, key=lambda x: x.name != "status-message")

    def build_artifacts(self, cache):
        for example in self._examples:
            print(f"Building {example.name}...", file=sys.stderr)
            for _, dirs, _ in os.walk(example.path):
                if "src" in dirs:
                    self._build_artifact(example, cache)

    def sizes(self, cache):
        self.build_artifacts(cache)

        artifact_paths = glob.glob(self._examples_dir + "/*/res/*.wasm")
        return {
            os.path.basename(path): os.stat(path).st_size for path in artifact_paths
        }
