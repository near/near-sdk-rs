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
        client = docker.from_env()
        tag = "latest-arm64" if platform.machine() == "ARM64" else "latest-amd64"
        image = f"nearprotocol/contract-builder:{tag}"

        client.containers.run(
            image,
            "./build.sh",
            mounts=[
                docker.types.Mount("/host", self._root_dir, type="bind"),
                docker.types.Mount(
                    "/usr/local/cargo/registry", cache.registry, type="bind"
                ),
                docker.types.Mount("/usr/local/cargo/git", cache.git, type="bind"),
                docker.types.Mount("/target", cache.target, type="bind"),
            ],
            working_dir=f"/host/examples/{artifact.name}",
            cap_add=["SYS_PTRACE"],
            security_opt=["seccomp=unconfined"],
            remove=True,
            user=os.getuid(),
            environment={
                "RUSTFLAGS": "-C link-arg=-s",
                "CARGO_TARGET_DIR": "/target",
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
            self._build_artifact(example, cache)

    def sizes(self, cache):
        self.build_artifacts(cache)

        artifact_paths = glob.glob(self._examples_dir + "/*/res/*.wasm")
        return {
            os.path.basename(path): os.stat(path).st_size for path in artifact_paths
        }
